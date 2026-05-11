use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpListener};
use std::thread::{self, JoinHandle};

use mascot_render_protocol::{MotionTimelineKind, MotionTimelineRequest, MotionTimelineStep};

use crate::{
    mascot_render_server_address, mascot_render_server_psd_file_names_at,
    preview_mouth_flap_timeline_request, set_single_character_mode_mascot_render_server_at,
    set_vpt_ensemble_mascot_render_server_at, MASCOT_RENDER_SERVER_PORT,
    PREVIEW_MOUTH_FLAP_DURATION_MS, PREVIEW_MOUTH_FLAP_FPS,
};

#[test]
fn default_server_address_uses_expected_port() {
    assert_eq!(
        mascot_render_server_address().port(),
        MASCOT_RENDER_SERVER_PORT
    );
}

#[test]
fn preview_mouth_flap_request_matches_psd_viewer_timing() {
    assert_eq!(
        preview_mouth_flap_timeline_request(),
        MotionTimelineRequest {
            steps: vec![MotionTimelineStep {
                kind: MotionTimelineKind::MouthFlap,
                duration_ms: PREVIEW_MOUTH_FLAP_DURATION_MS,
                fps: PREVIEW_MOUTH_FLAP_FPS,
            }],
        }
    );
    assert_eq!(PREVIEW_MOUTH_FLAP_DURATION_MS, 5_000);
    assert_eq!(PREVIEW_MOUTH_FLAP_FPS, 4);
}

#[test]
fn psd_file_names_request_parses_json_response() {
    let body = r#"["body.psd","face.psd"]"#;
    let (address, handle) = start_mock_server_for_request("GET /psd-filenames HTTP/1.1\r\n", body);

    let psd_file_names = mascot_render_server_psd_file_names_at(address)
        .expect("PSD file names request should return JSON");

    assert_eq!(
        psd_file_names,
        vec!["body.psd".to_string(), "face.psd".to_string()]
    );
    handle.join().expect("mock server thread should finish");
}

#[test]
fn set_single_character_mode_request_posts_expected_endpoint() {
    let (address, handle) = start_mock_server_for_request(
        "POST /ensemble-mode/single-character HTTP/1.1\r\n",
        "applied",
    );

    set_single_character_mode_mascot_render_server_at(address)
        .expect("set single character mode request should succeed");

    handle.join().expect("mock server thread should finish");
}

#[test]
fn set_vpt_ensemble_request_posts_expected_endpoint() {
    let (address, handle) =
        start_mock_server_for_request("POST /vpt-ensemble HTTP/1.1\r\n", "applied");

    set_vpt_ensemble_mascot_render_server_at(
        address,
        &["ずんだもん".to_string(), "四国めたん".to_string()],
    )
    .expect("set vpt ensemble request should succeed");

    handle.join().expect("mock server thread should finish");
}

fn start_mock_server_for_request(
    expected_request_line: &'static str,
    body: &'static str,
) -> (SocketAddr, JoinHandle<()>) {
    let listener =
        TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0))).expect("mock server should bind");
    let address = listener
        .local_addr()
        .expect("mock server should expose local address");
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("mock server should accept client");
        let mut reader = BufReader::new(stream.try_clone().expect("stream clone should succeed"));
        let mut request_line = String::new();
        reader
            .read_line(&mut request_line)
            .expect("request line should be readable");
        assert_eq!(request_line, expected_request_line);
        let mut line = String::new();
        loop {
            line.clear();
            reader
                .read_line(&mut line)
                .expect("header line should be readable");
            if line == "\r\n" || line == "\n" {
                break;
            }
        }
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: application/json; charset=utf-8\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        stream
            .write_all(response.as_bytes())
            .expect("mock response should be writable");
        stream.flush().expect("mock response should flush");
    });
    (address, handle)
}
