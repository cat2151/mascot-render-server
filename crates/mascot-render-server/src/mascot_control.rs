use std::ffi::OsString;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use mascot_render_client::{
    change_skin_mascot_render_server, mascot_render_server_address,
    mascot_render_server_healthcheck_at, play_timeline_mascot_render_server,
    show_mascot_render_server, wait_for_mascot_render_server_healthcheck_at, ChangeSkinRequest,
    MotionTimelineRequest,
};
use serde::Serialize;

use crate::{log_server_error, log_server_info, validate_motion_timeline_request};

const ACCEPT_POLL_INTERVAL: Duration = Duration::from_millis(50);
const IO_TIMEOUT: Duration = Duration::from_secs(2);
const STARTUP_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MascotControlCommand {
    Show,
    Hide,
    ChangeSkin(PathBuf),
    PlayTimeline(MotionTimelineRequest),
}

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

pub fn ensure_mascot_render_server_visible(config_path: &Path) -> Result<()> {
    let address = mascot_render_server_address();
    if mascot_render_server_healthcheck_at(address).is_err() {
        spawn_mascot_render_server(config_path)?;
        wait_for_mascot_render_server_healthcheck_at(address, STARTUP_TIMEOUT)?;
    }

    show_mascot_render_server()
}

pub fn sync_mascot_render_server_preview(
    config_path: &Path,
    png_path: Option<&Path>,
) -> Result<()> {
    let Some(png_path) = png_path else {
        return Ok(());
    };

    ensure_mascot_render_server_visible(config_path)?;
    change_skin_mascot_render_server(png_path)
}

pub fn play_mascot_render_server_timeline(
    config_path: &Path,
    request: &MotionTimelineRequest,
) -> Result<()> {
    ensure_mascot_render_server_visible(config_path)?;
    play_timeline_mascot_render_server(request)
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
                    log_server_error(format!(
                        "trigger=http_request peer={peer} failed to set mascot control read timeout: {error}"
                    ));
                    continue;
                }
                if let Err(error) = stream.set_write_timeout(Some(IO_TIMEOUT)) {
                    log_server_error(format!(
                        "trigger=http_request peer={peer} failed to set mascot control write timeout: {error}"
                    ));
                    continue;
                }
                if let Err(error) =
                    handle_connection(&mut stream, peer, &command_tx, notify.as_ref())
                {
                    log_server_error(format!(
                        "trigger=http_request peer={peer} mascot control connection error: {error:#}"
                    ));
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(ACCEPT_POLL_INTERVAL);
            }
            Err(error) => {
                log_server_error(format!(
                    "trigger=http_accept mascot control accept error: {error}"
                ));
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
                log_server_error(format!(
                    "trigger=http_request peer={peer} request handling failed: {error:#}"
                ));
                HttpResponse::internal_server_error(error.to_string())
            }
        },
        Err(error) => {
            log_server_error(format!(
                "trigger=http_request peer={peer} failed to read HTTP request: {error:#}"
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
            log_server_info(format!(
                "trigger=http_request peer={peer} action=show show request を受け取りました"
            ));
            command_tx
                .send(MascotControlCommand::Show)
                .context("failed to enqueue mascot show command")?;
            log_server_info(format!(
                "trigger=http_request peer={peer} action=show show command を UI に送信しました"
            ));
            notify_ui(notify);
            Ok(HttpResponse::ok_text("ok"))
        }
        ("POST", "/hide") => {
            log_server_info(format!(
                "trigger=http_request peer={peer} action=hide hide request を受け取りました"
            ));
            command_tx
                .send(MascotControlCommand::Hide)
                .context("failed to enqueue mascot hide command")?;
            log_server_info(format!(
                "trigger=http_request peer={peer} action=hide hide command を UI に送信しました"
            ));
            notify_ui(notify);
            Ok(HttpResponse::ok_text("ok"))
        }
        ("POST", "/change-skin") => {
            let request: ChangeSkinRequest = serde_json::from_slice(&request.body)
                .context("failed to parse mascot change-skin request JSON")?;
            log_received_request(peer, "change_skin", &request);
            let png_path = request.png_path.clone();
            command_tx
                .send(MascotControlCommand::ChangeSkin(request.png_path))
                .context("failed to enqueue mascot change-skin command")?;
            log_server_info(format!(
                "trigger=http_request peer={peer} action=change_skin skin変更 command を UI に送信しました: png_path={}",
                png_path.display()
            ));
            notify_ui(notify);
            Ok(HttpResponse::ok_text("ok"))
        }
        ("POST", "/timeline") => {
            let request: MotionTimelineRequest = serde_json::from_slice(&request.body)
                .context("failed to parse mascot motion timeline request JSON")?;
            validate_motion_timeline_request(&request)?;
            log_received_request(peer, "timeline", &request);
            command_tx
                .send(MascotControlCommand::PlayTimeline(request))
                .context("failed to enqueue mascot motion timeline command")?;
            log_server_info(format!(
                "trigger=http_request peer={peer} action=timeline motion timeline command を UI に送信しました"
            ));
            notify_ui(notify);
            Ok(HttpResponse::ok_text("ok"))
        }
        _ => Ok(HttpResponse::not_found("not found")),
    }
}

fn notify_ui(notify: Option<&Arc<dyn Fn() + Send + Sync>>) {
    if let Some(notify) = notify {
        notify();
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

fn spawn_mascot_render_server(config_path: &Path) -> Result<()> {
    let candidates = spawn_command_candidates(config_path)?;
    let mut last_error = None;

    for (program, args) in candidates {
        let mut command = Command::new(&program);
        command
            .args(&args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        match command.spawn() {
            Ok(_child) => return Ok(()),
            Err(error) => {
                last_error = Some(anyhow!(
                    "failed to spawn {:?} {:?}: {}",
                    program,
                    args,
                    error
                ));
            }
        }
    }

    Err(last_error.unwrap_or_else(|| anyhow!("no mascot-render-server spawn command available")))
}

fn spawn_command_candidates(config_path: &Path) -> Result<Vec<(OsString, Vec<OsString>)>> {
    let mut candidates = Vec::new();
    let sibling_binary = std::env::current_exe()
        .context("failed to resolve current executable path")?
        .with_file_name(mascot_render_server_binary_name());

    if sibling_binary.exists() {
        candidates.push((
            sibling_binary.into_os_string(),
            vec![
                OsString::from("--config"),
                config_path.as_os_str().to_os_string(),
            ],
        ));
    }

    candidates.push((
        OsString::from("cargo"),
        vec![
            OsString::from("run"),
            OsString::from("-p"),
            OsString::from("mascot-render-server"),
            OsString::from("--bin"),
            OsString::from("mascot-render-server"),
            OsString::from("--"),
            OsString::from("--config"),
            config_path.as_os_str().to_os_string(),
        ],
    ));

    Ok(candidates)
}

fn mascot_render_server_binary_name() -> &'static str {
    if cfg!(windows) {
        "mascot-render-server.exe"
    } else {
        "mascot-render-server"
    }
}

fn log_received_request<T: Serialize>(peer: SocketAddr, action: &str, request: &T) {
    match serde_json::to_string_pretty(request) {
        Ok(request_json) => log_server_info(format!(
            "trigger=http_request peer={peer} action={action} request を受け取りました\nrequest:\n{request_json}"
        )),
        Err(error) => {
            log_server_error(format!(
                "trigger=http_request peer={peer} action={action} request の pretty JSON 整形に失敗しました: {error:#}"
            ));
            log_server_info(format!(
                "trigger=http_request peer={peer} action={action} request を受け取りました"
            ));
        }
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
