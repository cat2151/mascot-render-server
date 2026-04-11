use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use mascot_render_client::mascot_render_server_address;
use mascot_render_protocol::{
    validate_motion_timeline_request, ChangeCharacterRequest, MotionTimelineRequest,
    ServerCommandKind, ServerCommandStage, ServerCommandStatus, ServerStatusStore,
};
use serde::Serialize;

use crate::command::{
    change_character_summary, timeline_summary, ControlCommandCompletion, ControlCommandWaitError,
    MascotControlCommand,
};
use crate::logging::{log_control_error, log_control_info};

const ACCEPT_POLL_INTERVAL: Duration = Duration::from_millis(50);
const IO_TIMEOUT: Duration = Duration::from_secs(2);
const APPLY_WAIT_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug)]
struct HttpRequest {
    method: String,
    path: String,
    body: Vec<u8>,
}

#[derive(Debug)]
struct HttpResponse {
    status_code: u16,
    status_text: &'static str,
    content_type: &'static str,
    body: Vec<u8>,
}

pub fn start_mascot_control_server(
    command_tx: Sender<MascotControlCommand>,
    status_store: ServerStatusStore,
) -> Result<JoinHandle<()>> {
    start_mascot_control_server_with_notify(command_tx, status_store, None)
}

pub fn start_mascot_control_server_with_notify(
    command_tx: Sender<MascotControlCommand>,
    status_store: ServerStatusStore,
    notify: Option<Arc<dyn Fn() + Send + Sync>>,
) -> Result<JoinHandle<()>> {
    let (_address, handle) = start_mascot_control_server_on_with_notify(
        mascot_render_server_address(),
        command_tx,
        status_store,
        notify,
    )?;
    Ok(handle)
}

#[cfg(test)]
pub(crate) fn start_mascot_control_server_on(
    address: SocketAddr,
    command_tx: Sender<MascotControlCommand>,
    status_store: ServerStatusStore,
) -> Result<(SocketAddr, JoinHandle<()>)> {
    start_mascot_control_server_on_with_notify(address, command_tx, status_store, None)
}

pub(crate) fn start_mascot_control_server_on_with_notify(
    address: SocketAddr,
    command_tx: Sender<MascotControlCommand>,
    status_store: ServerStatusStore,
    notify: Option<Arc<dyn Fn() + Send + Sync>>,
) -> Result<(SocketAddr, JoinHandle<()>)> {
    let listener = bind_control_listener(address)?;
    let bound_address = listener
        .local_addr()
        .context("failed to read bound mascot control address")?;
    listener
        .set_nonblocking(true)
        .with_context(|| format!("failed to set {bound_address} nonblocking"))?;

    let handle = thread::spawn(move || accept_loop(listener, command_tx, status_store, notify));
    Ok((bound_address, handle))
}

fn bind_control_listener(address: SocketAddr) -> Result<TcpListener> {
    TcpListener::bind(address).map_err(|error| match error.kind() {
        std::io::ErrorKind::AddrInUse => anyhow!(
            "failed to bind {address}: mascot-render-server may already be running on this port; reuse the existing server or stop it first"
        ),
        _ => anyhow!("failed to bind {address}: {error}"),
    })
}

fn accept_loop(
    listener: TcpListener,
    command_tx: Sender<MascotControlCommand>,
    status_store: ServerStatusStore,
    notify: Option<Arc<dyn Fn() + Send + Sync>>,
) {
    loop {
        match listener.accept() {
            Ok((mut stream, peer)) => {
                if let Err(error) = stream.set_read_timeout(Some(IO_TIMEOUT)) {
                    log_control_error(format!(
                        "event=control_connection stage=set_read_timeout peer={peer} error={error}"
                    ));
                    continue;
                }
                if let Err(error) = stream.set_write_timeout(Some(IO_TIMEOUT)) {
                    log_control_error(format!(
                        "event=control_connection stage=set_write_timeout peer={peer} error={error}"
                    ));
                    continue;
                }
                if let Err(error) = handle_connection(
                    &mut stream,
                    peer,
                    &command_tx,
                    &status_store,
                    notify.as_ref(),
                ) {
                    log_control_error(format!(
                        "event=control_connection stage=handle peer={peer} error={error:#}"
                    ));
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(ACCEPT_POLL_INTERVAL);
            }
            Err(error) => {
                log_control_error(format!("event=control_accept stage=accept error={error}"));
                thread::sleep(ACCEPT_POLL_INTERVAL);
            }
        }
    }
}

fn handle_connection(
    stream: &mut TcpStream,
    peer: SocketAddr,
    command_tx: &Sender<MascotControlCommand>,
    status_store: &ServerStatusStore,
    notify: Option<&Arc<dyn Fn() + Send + Sync>>,
) -> Result<()> {
    let response = match read_http_request(stream) {
        Ok(request) => match route_request(peer, request, command_tx, status_store, notify) {
            Ok(response) => response,
            Err(error) => {
                log_control_error(format!(
                    "event=control_request stage=route peer={peer} error={error:#}"
                ));
                HttpResponse::internal_server_error(error.to_string())
            }
        },
        Err(error) => {
            log_control_error(format!(
                "event=control_request stage=parse peer={peer} error={error:#}"
            ));
            HttpResponse::bad_request(error.to_string())
        }
    };

    write_http_response(stream, &response)
}

fn route_request(
    peer: SocketAddr,
    request: HttpRequest,
    command_tx: &Sender<MascotControlCommand>,
    status_store: &ServerStatusStore,
    notify: Option<&Arc<dyn Fn() + Send + Sync>>,
) -> Result<HttpResponse> {
    let path = canonical_path(&request.path);
    match (request.method.as_str(), path.as_str()) {
        ("GET", "/health") => Ok(HttpResponse::ok_text("ok")),
        ("GET", "/status") => {
            let body = serde_json::to_vec(&status_store.snapshot()?)
                .context("failed to serialize mascot server status")?;
            Ok(HttpResponse::ok_json(body))
        }
        ("POST", "/show") => {
            let status = ServerCommandStatus::queued(ServerCommandKind::Show, "show");
            enqueue_command(
                peer,
                "show",
                MascotControlCommand::show_with_status(status),
                command_tx,
                status_store,
                notify,
            )
        }
        ("POST", "/hide") => {
            let status = ServerCommandStatus::queued(ServerCommandKind::Hide, "hide");
            enqueue_command(
                peer,
                "hide",
                MascotControlCommand::hide_with_status(status),
                command_tx,
                status_store,
                notify,
            )
        }
        ("POST", "/change-character") => {
            let request: ChangeCharacterRequest = serde_json::from_slice(&request.body)
                .context("failed to parse mascot change-character request JSON")?;
            let character_name = request.character_name.clone();
            let status = ServerCommandStatus::queued(
                ServerCommandKind::ChangeCharacter,
                change_character_summary(&character_name),
            );
            log_request_payload(peer, "change_character", &request);
            let response = enqueue_apply_command(
                peer,
                "change_character",
                command_tx,
                status_store,
                notify,
                |completion| {
                    MascotControlCommand::change_character_with_completion(
                        character_name.clone(),
                        completion,
                        status,
                    )
                },
            )?;
            log_control_info(format!(
                "event=control_request stage=applied peer={peer} action=change_character character_name={character_name}"
            ));
            Ok(response)
        }
        ("POST", "/timeline") => {
            let request: MotionTimelineRequest = serde_json::from_slice(&request.body)
                .context("failed to parse mascot motion timeline request JSON")?;
            validate_motion_timeline_request(&request)?;
            let status = ServerCommandStatus::queued(
                ServerCommandKind::Timeline,
                timeline_summary(&request),
            );
            log_request_payload(peer, "timeline", &request);
            let response = enqueue_apply_command(
                peer,
                "timeline",
                command_tx,
                status_store,
                notify,
                |completion| {
                    MascotControlCommand::play_timeline_with_completion(request, completion, status)
                },
            )?;
            log_control_info(format!(
                "event=control_request stage=applied peer={peer} action=timeline"
            ));
            Ok(response)
        }
        _ => Ok(HttpResponse::not_found("not found")),
    }
}

fn enqueue_command(
    peer: SocketAddr,
    action: &str,
    command: MascotControlCommand,
    command_tx: &Sender<MascotControlCommand>,
    status_store: &ServerStatusStore,
    notify: Option<&Arc<dyn Fn() + Send + Sync>>,
) -> Result<HttpResponse> {
    log_control_info(format!(
        "event=control_request stage=received peer={peer} action={action}"
    ));
    let command_status = command.status().clone();
    status_store.update(|snapshot| {
        snapshot.current_command = Some(command_status.clone());
        snapshot.last_error = None;
    })?;
    command_tx
        .send(command)
        .inspect_err(|error| {
            let failed_status = command_status.with_stage(
                ServerCommandStage::Failed,
                mascot_render_protocol::now_unix_ms(),
                Some(error.to_string()),
            );
            let _ = status_store.update(|snapshot| {
                snapshot.current_command = None;
                snapshot.last_failed_command = Some(failed_status);
                snapshot.last_error = Some(error.to_string());
            });
        })
        .with_context(|| format!("failed to enqueue mascot {action} command"))?;
    log_control_info(format!(
        "event=control_request stage=queued peer={peer} action={action}"
    ));
    notify_ui(notify);
    Ok(HttpResponse::ok_text("queued"))
}

fn enqueue_apply_command(
    peer: SocketAddr,
    action: &str,
    command_tx: &Sender<MascotControlCommand>,
    status_store: &ServerStatusStore,
    notify: Option<&Arc<dyn Fn() + Send + Sync>>,
    build_command: impl FnOnce(ControlCommandCompletion) -> MascotControlCommand,
) -> Result<HttpResponse> {
    let (completion, waiter) = ControlCommandCompletion::pair();
    enqueue_command(
        peer,
        action,
        build_command(completion),
        command_tx,
        status_store,
        notify,
    )?;
    match waiter.wait(APPLY_WAIT_TIMEOUT) {
        Ok(()) => Ok(HttpResponse::ok_text("applied")),
        Err(ControlCommandWaitError::TimedOut(timeout)) => {
            let error = ControlCommandWaitError::TimedOut(timeout).into_anyhow(action);
            log_control_error(format!(
                "event=control_request stage=apply_timeout peer={peer} action={action} error={error:#}"
            ));
            Ok(HttpResponse::gateway_timeout(error.to_string()))
        }
        Err(error) => {
            let error = error.into_anyhow(action);
            log_control_error(format!(
                "event=control_request stage=apply_failed peer={peer} action={action} error={error:#}"
            ));
            Ok(HttpResponse::internal_server_error(error.to_string()))
        }
    }
}

fn notify_ui(notify: Option<&Arc<dyn Fn() + Send + Sync>>) {
    if let Some(notify) = notify {
        notify();
    }
}

fn log_request_payload<T: Serialize>(peer: SocketAddr, action: &str, request: &T) {
    match serde_json::to_string_pretty(request) {
        Ok(request_json) => log_control_info(format!(
            "event=control_request stage=received peer={peer} action={action}\nrequest:\n{request_json}"
        )),
        Err(error) => log_control_error(format!(
            "event=control_request stage=serialize peer={peer} action={action} error={error:#}"
        )),
    }
}

fn read_http_request(stream: &mut TcpStream) -> Result<HttpRequest> {
    let mut reader = BufReader::new(stream);
    let mut request_line = String::new();
    reader
        .read_line(&mut request_line)
        .context("failed to read HTTP request line")?;
    if request_line.trim().is_empty() {
        bail!("empty HTTP request");
    }

    let mut parts = request_line.split_whitespace();
    let method = parts
        .next()
        .ok_or_else(|| anyhow!("missing HTTP method"))?
        .to_string();
    let path = parts
        .next()
        .ok_or_else(|| anyhow!("missing HTTP path"))?
        .to_string();
    let _version = parts
        .next()
        .ok_or_else(|| anyhow!("missing HTTP version"))?;

    let mut content_length = 0usize;
    let mut line = String::new();
    loop {
        line.clear();
        reader
            .read_line(&mut line)
            .context("failed to read HTTP header line")?;
        if line == "\r\n" || line == "\n" {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            if name.trim().eq_ignore_ascii_case("content-length") {
                content_length = value
                    .trim()
                    .parse::<usize>()
                    .context("invalid Content-Length header")?;
            }
        }
    }

    let mut body = vec![0; content_length];
    reader
        .read_exact(&mut body)
        .context("failed to read HTTP request body")?;

    Ok(HttpRequest { method, path, body })
}

fn write_http_response(stream: &mut TcpStream, response: &HttpResponse) -> Result<()> {
    let header = format!(
        "HTTP/1.1 {} {}\r\nContent-Length: {}\r\nContent-Type: {}\r\nConnection: close\r\n\r\n",
        response.status_code,
        response.status_text,
        response.body.len(),
        response.content_type
    );
    stream
        .write_all(header.as_bytes())
        .context("failed to write HTTP response header")?;
    stream
        .write_all(&response.body)
        .context("failed to write HTTP response body")?;
    stream.flush().context("failed to flush HTTP response")
}

fn canonical_path(path: &str) -> String {
    if path.len() > 1 {
        path.trim_end_matches('/').to_string()
    } else {
        path.to_string()
    }
}

impl HttpResponse {
    fn ok_text(body: &str) -> Self {
        Self {
            status_code: 200,
            status_text: "OK",
            content_type: "text/plain; charset=utf-8",
            body: body.as_bytes().to_vec(),
        }
    }

    fn ok_json(body: Vec<u8>) -> Self {
        Self {
            status_code: 200,
            status_text: "OK",
            content_type: "application/json; charset=utf-8",
            body,
        }
    }

    fn bad_request(body: String) -> Self {
        Self {
            status_code: 400,
            status_text: "Bad Request",
            content_type: "text/plain; charset=utf-8",
            body: body.into_bytes(),
        }
    }

    fn internal_server_error(body: String) -> Self {
        Self {
            status_code: 500,
            status_text: "Internal Server Error",
            content_type: "text/plain; charset=utf-8",
            body: body.into_bytes(),
        }
    }

    fn gateway_timeout(body: String) -> Self {
        Self {
            status_code: 504,
            status_text: "Gateway Timeout",
            content_type: "text/plain; charset=utf-8",
            body: body.into_bytes(),
        }
    }

    fn not_found(body: &str) -> Self {
        Self {
            status_code: 404,
            status_text: "Not Found",
            content_type: "text/plain; charset=utf-8",
            body: body.as_bytes().to_vec(),
        }
    }
}
