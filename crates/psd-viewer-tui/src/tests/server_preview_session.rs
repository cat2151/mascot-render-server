use std::fs;
use std::path::PathBuf;

use mascot_render_core::{
    load_mascot_config, mascot_runtime_state_path, workspace_cache_root, MascotEnsembleMode,
};
use mascot_render_protocol::PreviewTargetRequest;

use crate::server_preview_session::prepare_preview_startup;
use crate::server_preview_session::sync_running_preview_session;

#[test]
fn startup_seeds_selected_psd_and_replaces_saved_ensemble_mode() {
    let root = workspace_cache_root().join("test-preview-session-ensemble");
    fs::create_dir_all(&root).unwrap();
    let config_path = root.join("server.toml");
    fs::write(&config_path, "ensemble_mode = 'vpt'\nalways_bend = false\n").unwrap();
    let runtime_path = mascot_runtime_state_path(&config_path);
    let _ = fs::remove_file(&runtime_path);
    let target = PreviewTargetRequest {
        png_path: PathBuf::from("X:/cache/selected.png"),
        scale: Some(0.25),
        zip_path: PathBuf::from("X:/assets/selected.zip"),
        psd_path_in_zip: PathBuf::from("selected.psd"),
        display_diff_path: Some(PathBuf::from("X:/cache/selected.json")),
    };

    prepare_preview_startup(&config_path, &target).unwrap();

    let config = load_mascot_config(&config_path).unwrap();
    assert_eq!(config.ensemble_mode, MascotEnsembleMode::SingleCharacter);
    assert!(!config.always_bend.enabled);
    assert_eq!(config.png_path, target.png_path);
    assert_eq!(config.zip_path, target.zip_path);
    assert_eq!(config.psd_path_in_zip, target.psd_path_in_zip);
    assert_eq!(config.display_diff_path, target.display_diff_path);
    assert_eq!(config.scale, target.scale);
    assert_eq!(config.ensemble_scale, None);
}

#[test]
fn startup_creates_config_and_runtime_state_for_first_use() {
    let root = workspace_cache_root().join("test-preview-session-first-use");
    let config_path = root.join("server.toml");
    let _ = fs::remove_file(&config_path);
    let _ = fs::remove_file(mascot_runtime_state_path(&config_path));
    let target = PreviewTargetRequest {
        png_path: PathBuf::from("X:/cache/first.png"),
        scale: None,
        zip_path: PathBuf::from("X:/assets/first.zip"),
        psd_path_in_zip: PathBuf::from("first.psd"),
        display_diff_path: None,
    };
    prepare_preview_startup(&config_path, &target).unwrap();
    let config = load_mascot_config(&config_path).unwrap();
    assert_eq!(config.png_path, target.png_path);
    assert_eq!(config.ensemble_mode, MascotEnsembleMode::SingleCharacter);
}

#[test]
fn existing_ensemble_is_switched_before_preview_and_show() {
    verify_http_session(true, false);
}

#[test]
fn later_selections_do_not_override_explicit_mode_changes() {
    verify_http_session(false, false);
}

#[test]
fn failed_mode_change_does_not_send_preview_or_show() {
    verify_http_session(true, true);
}

fn verify_http_session(initialize: bool, reject_mode: bool) {
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;
    use std::thread;
    use std::time::Duration;

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let mut routes = Vec::new();
        if initialize {
            routes.push("/ensemble-mode/single-character");
        }
        if !reject_mode {
            routes.extend(["/preview-target", "/show"]);
        }
        for route in routes {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut reader = BufReader::new(&stream);
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            assert_eq!(line.trim(), format!("POST {route} HTTP/1.1"));
            let mut content_length = 0;
            loop {
                line.clear();
                reader.read_line(&mut line).unwrap();
                if line == "\r\n" {
                    break;
                }
                if let Some(length) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                    content_length = length.trim().parse::<usize>().unwrap();
                }
            }
            let mut body = vec![0; content_length];
            reader.read_exact(&mut body).unwrap();
            if route == "/preview-target" {
                let request: PreviewTargetRequest = serde_json::from_slice(&body).unwrap();
                assert_eq!(request.png_path, PathBuf::from("X:/cache/selected.png"));
            }
            let status = if reject_mode { "500 Failed" } else { "200 OK" };
            write!(
                stream,
                "HTTP/1.1 {status}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            )
            .unwrap();
        }
    });
    let target = PreviewTargetRequest {
        png_path: PathBuf::from("X:/cache/selected.png"),
        scale: None,
        zip_path: PathBuf::from("X:/assets/selected.zip"),
        psd_path_in_zip: PathBuf::from("selected.psd"),
        display_diff_path: None,
    };
    let result = sync_running_preview_session(address, &target, initialize);
    assert_eq!(result.is_err(), reject_mode);
    server.join().unwrap();
}
