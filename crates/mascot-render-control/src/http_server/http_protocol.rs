use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;

use anyhow::{anyhow, Context, Result};

const MAX_REQUEST_BODY_BYTES: usize = 1024 * 1024;

#[derive(Debug)]
pub(super) struct HttpRequest {
    pub(super) method: String,
    pub(super) path: String,
    pub(super) body: Vec<u8>,
}

#[derive(Debug)]
pub(super) struct HttpResponse {
    status_code: u16,
    status_text: &'static str,
    content_type: &'static str,
    body: Vec<u8>,
}

#[derive(Debug)]
pub(super) enum HttpRequestReadError {
    BadRequest(anyhow::Error),
    PayloadTooLarge {
        content_length: usize,
        max_length: usize,
    },
}

impl HttpRequestReadError {
    fn bad_request(error: anyhow::Error) -> Self {
        Self::BadRequest(error)
    }

    pub(super) fn is_payload_too_large(&self) -> bool {
        matches!(self, Self::PayloadTooLarge { .. })
    }
}

impl std::fmt::Display for HttpRequestReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadRequest(error) => write!(f, "{error}"),
            Self::PayloadTooLarge {
                content_length,
                max_length,
            } => write!(
                f,
                "Content-Length {content_length} exceeds max request body size {max_length}"
            ),
        }
    }
}

impl std::error::Error for HttpRequestReadError {}

pub(super) fn read_http_request(
    stream: &mut TcpStream,
) -> Result<HttpRequest, HttpRequestReadError> {
    let mut reader = BufReader::new(stream);
    let mut request_line = String::new();
    reader
        .read_line(&mut request_line)
        .context("failed to read HTTP request line")
        .map_err(HttpRequestReadError::bad_request)?;
    if request_line.trim().is_empty() {
        return Err(HttpRequestReadError::bad_request(anyhow!(
            "empty HTTP request"
        )));
    }

    let mut parts = request_line.split_whitespace();
    let method = parts
        .next()
        .ok_or_else(|| HttpRequestReadError::bad_request(anyhow!("missing HTTP method")))?
        .to_string();
    let path = parts
        .next()
        .ok_or_else(|| HttpRequestReadError::bad_request(anyhow!("missing HTTP path")))?
        .to_string();
    let _version = parts
        .next()
        .ok_or_else(|| HttpRequestReadError::bad_request(anyhow!("missing HTTP version")))?;

    let mut content_length = 0usize;
    let mut line = String::new();
    loop {
        line.clear();
        reader
            .read_line(&mut line)
            .context("failed to read HTTP header line")
            .map_err(HttpRequestReadError::bad_request)?;
        if line == "\r\n" || line == "\n" {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            if name.trim().eq_ignore_ascii_case("content-length") {
                content_length = value
                    .trim()
                    .parse::<usize>()
                    .context("invalid Content-Length header")
                    .map_err(HttpRequestReadError::bad_request)?;
                if content_length > MAX_REQUEST_BODY_BYTES {
                    return Err(HttpRequestReadError::PayloadTooLarge {
                        content_length,
                        max_length: MAX_REQUEST_BODY_BYTES,
                    });
                }
            }
        }
    }

    let mut body = vec![0; content_length];
    reader
        .read_exact(&mut body)
        .context("failed to read HTTP request body")
        .map_err(HttpRequestReadError::bad_request)?;

    Ok(HttpRequest { method, path, body })
}

pub(super) fn write_http_response(stream: &mut TcpStream, response: &HttpResponse) -> Result<()> {
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

pub(super) fn canonical_path(path: &str) -> String {
    if path.len() > 1 {
        path.trim_end_matches('/').to_string()
    } else {
        path.to_string()
    }
}

impl HttpResponse {
    pub(super) fn ok_text(body: &str) -> Self {
        Self {
            status_code: 200,
            status_text: "OK",
            content_type: "text/plain; charset=utf-8",
            body: body.as_bytes().to_vec(),
        }
    }

    pub(super) fn ok_json(body: Vec<u8>) -> Self {
        Self {
            status_code: 200,
            status_text: "OK",
            content_type: "application/json; charset=utf-8",
            body,
        }
    }

    pub(super) fn bad_request(body: String) -> Self {
        Self {
            status_code: 400,
            status_text: "Bad Request",
            content_type: "text/plain; charset=utf-8",
            body: body.into_bytes(),
        }
    }

    pub(super) fn payload_too_large(body: String) -> Self {
        Self {
            status_code: 413,
            status_text: "Payload Too Large",
            content_type: "text/plain; charset=utf-8",
            body: body.into_bytes(),
        }
    }

    pub(super) fn internal_server_error(body: String) -> Self {
        Self {
            status_code: 500,
            status_text: "Internal Server Error",
            content_type: "text/plain; charset=utf-8",
            body: body.into_bytes(),
        }
    }

    pub(super) fn gateway_timeout(body: String) -> Self {
        Self {
            status_code: 504,
            status_text: "Gateway Timeout",
            content_type: "text/plain; charset=utf-8",
            body: body.into_bytes(),
        }
    }

    pub(super) fn not_found(body: &str) -> Self {
        Self {
            status_code: 404,
            status_text: "Not Found",
            content_type: "text/plain; charset=utf-8",
            body: body.as_bytes().to_vec(),
        }
    }
}
