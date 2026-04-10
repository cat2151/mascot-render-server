use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use mascot_render_client::{
    mascot_render_server_address, ChangeSkinRequest, MotionTimelineRequest,
};
use serde::Serialize;

use crate::command::MascotControlCommand;
use crate::logging::{log_control_error, log_control_info};
use crate::timeline::validate_motion_timeline_request;

const ACCEPT_POLL_INTERVAL: Duration = Duration::from_millis(50);
const IO_TIMEOUT: Duration = Duration::from_secs(2);

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
    body: Vec<u8>,
}

pub fn start_mascot_control_server(
    command_tx: Sender<MascotControlCommand>,
) -> Result<JoinHandle<()>> {
    start_mascot_control_server_with_notify(command_tx, None)
}

pub fn start_mascot_control_server_with_notify(
    command_tx: Sender<MascotControlCommand>,
    notify: Option<Arc<dyn Fn() + Send + Sync>>,
) -> Result<JoinHandle<()>> {
    let (_address, handle) = start_mascot_control_server_on_with_notify(
        mascot_render_server_address(),
        command_tx,
        notify,
    )?;
    Ok(handle)
}

#[cfg(test)]
pub(crate) fn start_mascot_control_server_on(
    address: SocketAddr,
    command_tx: Sender<MascotControlCommand>,
) -> Result<(SocketAddr, JoinHandle<()>)> {
    start_mascot_control_server_on_with_notify(address, command_tx, None)
}

pub(crate) fn start_mascot_control_server_on_with_notify(
    address: SocketAddr,
    command_tx: Sender<MascotControlCommand>,
    notify: Option<Arc<dyn Fn() + Send + Sync>>,
) -> Result<(SocketAddr, JoinHandle<()>)> {
    let listener = bind_control_listener(address)?;
    let bound_address = listener
        .local_addr()
        .context("failed to read bound mascot control address")?;
    listener
        .set_nonblocking(true)
        .with_context(|| format!("failed to set {bound_address} nonblocking"))?;

    let handle = thread::spawn(move || accept_loop(listener, command_tx, notify));
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
                if let Err(error) =
                    handle_connection(&mut stream, peer, &command_tx, notify.as_ref())
                {
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
    notify: Option<&Arc<dyn Fn() + Send + Sync>>,
) -> Result<()> {
    let response = match read_http_request(stream) {
        Ok(request) => match route_request(peer, request, command_tx, notify) {
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
    notify: Option<&Arc<dyn Fn() + Send + Sync>>,
) -> Result<HttpResponse> {
    let path = canonical_path(&request.path);
    match (request.method.as_str(), path.as_str()) {
        ("GET", "/health") => Ok(HttpResponse::ok_text("ok")),
        ("POST", "/show") => {
            enqueue_command(peer, "show", MascotControlCommand::Show, command_tx, notify)
        }
        ("POST", "/hide") => {
            enqueue_command(peer, "hide", MascotControlCommand::Hide, command_tx, notify)
        }
        ("POST", "/change-skin") => {
            let request: ChangeSkinRequest = serde_json::from_slice(&request.body)
                .context("failed to parse mascot change-skin request JSON")?;
            let png_path = request.png_path.clone();
            log_request_payload(peer, "change_skin", &request);
            enqueue_command(
                peer,
                "change_skin",
                MascotControlCommand::ChangeSkin(request.png_path),
                command_tx,
                notify,
            )?;
            log_control_info(format!(
                "event=control_request stage=enqueued peer={peer} action=change_skin png_path={}",
                png_path.display()
            ));
            Ok(HttpResponse::ok_text("ok"))
        }
        ("POST", "/timeline") => {
            let request: MotionTimelineRequest = serde_json::from_slice(&request.body)
                .context("failed to parse mascot motion timeline request JSON")?;
            validate_motion_timeline_request(&request)?;
            log_request_payload(peer, "timeline", &request);
            enqueue_command(
                peer,
                "timeline",
                MascotControlCommand::PlayTimeline(request),
                command_tx,
                notify,
            )
        }
        _ => Ok(HttpResponse::not_found("not found")),
    }
}

fn enqueue_command(
    peer: SocketAddr,
    action: &str,
    command: MascotControlCommand,
    command_tx: &Sender<MascotControlCommand>,
    notify: Option<&Arc<dyn Fn() + Send + Sync>>,
) -> Result<HttpResponse> {
    log_control_info(format!(
        "event=control_request stage=received peer={peer} action={action}"
    ));
    command_tx
        .send(command)
        .with_context(|| format!("failed to enqueue mascot {action} command"))?;
    notify_ui(notify);
    Ok(HttpResponse::ok_text("ok"))
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
        "HTTP/1.1 {} {}\r\nContent-Length: {}\r\nContent-Type: text/plain; charset=utf-8\r\nConnection: close\r\n\r\n",
        response.status_code,
        response.status_text,
        response.body.len()
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
            body: body.as_bytes().to_vec(),
        }
    }

    fn bad_request(body: String) -> Self {
        Self {
            status_code: 400,
            status_text: "Bad Request",
            body: body.into_bytes(),
        }
    }

    fn internal_server_error(body: String) -> Self {
        Self {
            status_code: 500,
            status_text: "Internal Server Error",
            body: body.into_bytes(),
        }
    }

    fn not_found(body: &str) -> Self {
        Self {
            status_code: 404,
            status_text: "Not Found",
            body: body.as_bytes().to_vec(),
        }
    }
}
