use crate::http_server::start_mascot_control_server_on;
use mascot_render_protocol::{ServerStatusSnapshot, ServerStatusStore};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::mpsc;

#[test]
fn playback_updates_latest_slot_without_queueing_gui_commands() {
    let (tx, rx) = mpsc::channel();
    let store = test_status_store();
    let (address, _) = start_mascot_control_server_on(
        SocketAddr::from(([127, 0, 0, 1], 0)),
        tx,
        store.clone(),
        empty_psd_file_names,
    )
    .unwrap();
    let mut request = mascot_render_protocol::PlaybackRequest {
        sent_unix_ms: mascot_render_protocol::now_unix_ms(),
        playing: true,
        beat: Some(1.0),
        beats_per_second: 2.0,
        singing: true,
    };
    mascot_render_client::sync_playback_at(address, &request).unwrap();
    assert!(store.playback().unwrap().unwrap().request.singing);
    request.playing = false;
    request.singing = false;
    request.sent_unix_ms = mascot_render_protocol::now_unix_ms();
    mascot_render_client::sync_playback_at(address, &request).unwrap();
    assert!(!store.playback().unwrap().unwrap().request.playing);
    assert!(rx.try_recv().is_err());
}

fn test_status_store() -> ServerStatusStore {
    ServerStatusStore::new(ServerStatusSnapshot::starting(
        PathBuf::from("config/mascot-render-server.toml"),
        PathBuf::from("config/mascot-render-server.runtime.json"),
        PathBuf::from("cache/demo/open.png"),
        PathBuf::from("assets/zip/demo.zip"),
        PathBuf::from("demo/basic.psd"),
    ))
}

fn empty_psd_file_names() -> anyhow::Result<Vec<String>> {
    Ok(Vec::new())
}
