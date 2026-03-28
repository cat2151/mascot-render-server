use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};

pub const MASCOT_RENDER_SERVER_PORT: u16 = 62152;

const MASCOT_RENDER_SERVER_HOST: &str = "127.0.0.1";
const IO_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeSkinRequest {
    pub png_path: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MotionTimelineKind {
    Shake,
    MouthFlap,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MotionTimelineStep {
    pub kind: MotionTimelineKind,
    pub duration_ms: u64,
    pub fps: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MotionTimelineRequest {
    pub steps: Vec<MotionTimelineStep>,
}

pub fn mascot_render_server_address() -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], MASCOT_RENDER_SERVER_PORT))
}

pub fn mascot_render_server_healthcheck() -> Result<()> {
    mascot_render_server_healthcheck_at(mascot_render_server_address())
}

pub fn show_mascot_render_server() -> Result<()> {
    show_mascot_render_server_at(mascot_render_server_address())
}

pub fn hide_mascot_render_server() -> Result<()> {
    hide_mascot_render_server_at(mascot_render_server_address())
}

pub fn change_skin_mascot_render_server(png_path: &Path) -> Result<()> {
    change_skin_mascot_render_server_at(mascot_render_server_address(), png_path)
}

pub fn play_timeline_mascot_render_server(request: &MotionTimelineRequest) -> Result<()> {
    play_timeline_mascot_render_server_at(mascot_render_server_address(), request)
}

pub fn mascot_render_server_healthcheck_at(address: SocketAddr) -> Result<()> {
    send_http_request(address, "GET", "/health", None)
}

pub fn show_mascot_render_server_at(address: SocketAddr) -> Result<()> {
    send_http_request(address, "POST", "/show", None)
}

pub fn hide_mascot_render_server_at(address: SocketAddr) -> Result<()> {
    send_http_request(address, "POST", "/hide", None)
}

pub fn change_skin_mascot_render_server_at(address: SocketAddr, png_path: &Path) -> Result<()> {
    let body = serde_json::to_vec(&ChangeSkinRequest {
        png_path: png_path.to_path_buf(),
    })
    .context("failed to serialize mascot change-skin request")?;
    send_http_request(address, "POST", "/change-skin", Some(&body))
}

pub fn play_timeline_mascot_render_server_at(
    address: SocketAddr,
    request: &MotionTimelineRequest,
) -> Result<()> {
    let body = serde_json::to_vec(request)
        .context("failed to serialize mascot motion timeline request")?;
    send_http_request(address, "POST", "/timeline", Some(&body))
}

pub fn wait_for_mascot_render_server_healthcheck_at(
    address: SocketAddr,
    timeout: Duration,
) -> Result<()> {
    let deadline = Instant::now() + timeout;
    let mut last_error = None;

    while Instant::now() < deadline {
        match mascot_render_server_healthcheck_at(address) {
            Ok(()) => return Ok(()),
            Err(error) => {
                last_error = Some(error);
                thread::sleep(Duration::from_millis(100));
            }
        }
    }

    Err(last_error
        .unwrap_or_else(|| anyhow!("timed out waiting for mascot-render-server at {address}")))
}

fn send_http_request(
    address: SocketAddr,
    method: &str,
    path: &str,
    body: Option<&[u8]>,
) -> Result<()> {
    let mut stream = TcpStream::connect_timeout(&address, IO_TIMEOUT)
        .with_context(|| format!("failed to connect to mascot-render-server at {address}"))?;
    stream
        .set_read_timeout(Some(IO_TIMEOUT))
        .with_context(|| format!("failed to set read timeout for {address}"))?;
    stream
        .set_write_timeout(Some(IO_TIMEOUT))
        .with_context(|| format!("failed to set write timeout for {address}"))?;

    let body = body.unwrap_or_default();
    let mut request = format!(
        "{method} {path} HTTP/1.1\r\nHost: {MASCOT_RENDER_SERVER_HOST}:{port}\r\nConnection: close\r\nContent-Length: {}\r\n",
        body.len(),
        port = address.port()
    );
    if !body.is_empty() {
        request.push_str("Content-Type: application/json\r\n");
    }
    request.push_str("\r\n");

    stream
        .write_all(request.as_bytes())
        .with_context(|| format!("failed to write HTTP request to {address}"))?;
    if !body.is_empty() {
        stream
            .write_all(body)
            .with_context(|| format!("failed to write HTTP body to {address}"))?;
    }
    stream
        .flush()
        .with_context(|| format!("failed to flush HTTP request to {address}"))?;

    read_http_response(&mut stream)
}

fn read_http_response(stream: &mut TcpStream) -> Result<()> {
    let mut reader = BufReader::new(stream);
    let mut status_line = String::new();
    reader
        .read_line(&mut status_line)
        .context("failed to read HTTP response status line")?;
    if status_line.trim().is_empty() {
        bail!("empty HTTP response");
    }

    let status_code = parse_status_code(&status_line)?;
    let mut content_length = 0usize;
    let mut line = String::new();
    loop {
        line.clear();
        reader
            .read_line(&mut line)
            .context("failed to read HTTP response header")?;
        if line == "\r\n" || line == "\n" {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            if name.trim().eq_ignore_ascii_case("content-length") {
                content_length = value
                    .trim()
                    .parse::<usize>()
                    .context("invalid HTTP response Content-Length header")?;
            }
        }
    }

    let mut body = vec![0; content_length];
    reader
        .read_exact(&mut body)
        .context("failed to read HTTP response body")?;

    if status_code == 200 {
        return Ok(());
    }

    bail!(
        "mascot-render-server request failed with HTTP {}: {}",
        status_code,
        String::from_utf8_lossy(&body).trim()
    )
}

fn parse_status_code(status_line: &str) -> Result<u16> {
    status_line
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| anyhow!("missing HTTP status code"))?
        .parse::<u16>()
        .context("invalid HTTP status code")
}
