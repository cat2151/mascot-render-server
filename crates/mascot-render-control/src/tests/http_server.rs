use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use mascot_render_client::{
    change_character_mascot_render_server_at, hide_mascot_render_server_at,
    mascot_render_server_healthcheck_at, mascot_render_server_psd_file_names_at,
    mascot_render_server_status_at, play_timeline_mascot_render_server_at,
    show_mascot_render_server_at,
};
use mascot_render_protocol::{
    MotionTimelineKind, MotionTimelineRequest, MotionTimelineStep, ServerCommandStage,
    ServerLifecyclePhase, ServerStatusSnapshot, ServerStatusStore,
};

use crate::command::MascotControlCommand;
use crate::http_server::{
    start_mascot_control_server_on, start_mascot_control_server_on_with_notify,
};

#[test]
fn mascot_control_server_accepts_show_hide_change_character_and_timeline() {
    let (tx, rx) = mpsc::channel();
    let status_store = test_status_store();
    let (address, _handle) = start_mascot_control_server_on(
        SocketAddr::from(([127, 0, 0, 1], 0)),
        tx,
        status_store,
        empty_psd_file_names,
    )
    .expect("should start mascot control server");
    wait_for_healthcheck(address);

    show_mascot_render_server_at(address).expect("show request should succeed");
    assert_eq!(
        rx.recv_timeout(Duration::from_secs(1))
            .expect("show command should arrive"),
        MascotControlCommand::show()
    );

    let character_name = "ずんだもん".to_string();
    let preview_request = {
        let character_name = character_name.clone();
        thread::spawn(move || change_character_mascot_render_server_at(address, &character_name))
    };
    let preview_command = rx
        .recv_timeout(Duration::from_secs(1))
        .expect("change-character command should arrive");
    assert_eq!(
        preview_command,
        MascotControlCommand::change_character(character_name.clone())
    );
    preview_command.finish(Ok(()));
    preview_request
        .join()
        .expect("change-character request thread should complete")
        .expect("change-character request should succeed");

    let timeline = MotionTimelineRequest {
        steps: vec![MotionTimelineStep {
            kind: MotionTimelineKind::Shake,
            duration_ms: 5_000,
            fps: 20,
        }],
    };
    let timeline_request = {
        let timeline = timeline.clone();
        thread::spawn(move || play_timeline_mascot_render_server_at(address, &timeline))
    };
    let timeline_command = rx
        .recv_timeout(Duration::from_secs(1))
        .expect("timeline command should arrive");
    assert_eq!(
        timeline_command,
        MascotControlCommand::play_timeline(timeline.clone())
    );
    timeline_command.finish(Ok(()));
    timeline_request
        .join()
        .expect("timeline request thread should complete")
        .expect("timeline request should succeed");

    let mouth_flap_timeline = MotionTimelineRequest {
        steps: vec![MotionTimelineStep {
            kind: MotionTimelineKind::MouthFlap,
            duration_ms: 5_000,
            fps: 20,
        }],
    };
    let mouth_flap_request = {
        let mouth_flap_timeline = mouth_flap_timeline.clone();
        thread::spawn(move || play_timeline_mascot_render_server_at(address, &mouth_flap_timeline))
    };
    let mouth_flap_command = rx
        .recv_timeout(Duration::from_secs(1))
        .expect("mouth flap timeline command should arrive");
    assert_eq!(
        mouth_flap_command,
        MascotControlCommand::play_timeline(mouth_flap_timeline.clone())
    );
    mouth_flap_command.finish(Ok(()));
    mouth_flap_request
        .join()
        .expect("mouth flap request thread should complete")
        .expect("mouth flap timeline request should succeed");

    hide_mascot_render_server_at(address).expect("hide request should succeed");
    assert_eq!(
        rx.recv_timeout(Duration::from_secs(1))
            .expect("hide command should arrive"),
        MascotControlCommand::hide()
    );
}

#[test]
fn mascot_control_server_reports_change_character_apply_failure_to_http_caller() {
    let (tx, rx) = mpsc::channel();
    let (address, _handle) = start_mascot_control_server_on(
        SocketAddr::from(([127, 0, 0, 1], 0)),
        tx,
        test_status_store(),
        empty_psd_file_names,
    )
    .expect("should start mascot control server");
    wait_for_healthcheck(address);

    let character_name = "ずんだもん".to_string();
    let request_thread = {
        let character_name = character_name.clone();
        thread::spawn(move || change_character_mascot_render_server_at(address, &character_name))
    };

    let command = rx
        .recv_timeout(Duration::from_secs(1))
        .expect("change-character command should arrive");
    assert_eq!(
        command,
        MascotControlCommand::change_character(character_name.clone())
    );
    command.finish(Err("failed to load requested skin".to_string()));

    let error = request_thread
        .join()
        .expect("request thread should complete")
        .expect_err("change-character request should report apply failure");
    assert!(
        error.to_string().contains("HTTP 500"),
        "unexpected error: {error:#}"
    );
    assert!(
        error.to_string().contains("failed to load requested skin"),
        "unexpected error: {error:#}"
    );
}

#[test]
fn mascot_control_server_reports_health() {
    let (tx, _rx) = mpsc::channel();
    let (address, _handle) = start_mascot_control_server_on(
        SocketAddr::from(([127, 0, 0, 1], 0)),
        tx,
        test_status_store(),
        empty_psd_file_names,
    )
    .expect("should start mascot control server");
    wait_for_healthcheck(address);

    mascot_render_server_healthcheck_at(address).expect("healthcheck should succeed");
}

#[test]
fn mascot_control_server_bind_error_mentions_existing_server() {
    let (tx, _rx) = mpsc::channel();
    let (address, _handle) = start_mascot_control_server_on(
        SocketAddr::from(([127, 0, 0, 1], 0)),
        tx,
        test_status_store(),
        empty_psd_file_names,
    )
    .expect("should start mascot control server");

    let (tx2, _rx2) = mpsc::channel();
    let error =
        start_mascot_control_server_on(address, tx2, test_status_store(), empty_psd_file_names)
            .expect_err("second server on the same address should fail");

    assert!(
        error
            .to_string()
            .contains("mascot-render-server may already be running"),
        "unexpected bind error: {error:#}"
    );
}

#[test]
fn mascot_control_server_notifies_ui_when_commands_arrive() {
    let (tx, _rx) = mpsc::channel();
    let notified = Arc::new(AtomicUsize::new(0));
    let notify_counter = Arc::clone(&notified);
    let notify = Arc::new(move || {
        notify_counter.fetch_add(1, Ordering::SeqCst);
    });
    let (address, _handle) = start_mascot_control_server_on_with_notify(
        SocketAddr::from(([127, 0, 0, 1], 0)),
        tx,
        test_status_store(),
        Arc::new(empty_psd_file_names),
        Some(notify),
    )
    .expect("should start mascot control server");
    wait_for_healthcheck(address);

    show_mascot_render_server_at(address).expect("show request should succeed");
    wait_for_notify(&notified);
}

#[test]
fn mascot_control_server_reports_status_snapshot() {
    let (tx, _rx) = mpsc::channel();
    let status_store = test_status_store();
    status_store
        .update(|snapshot| snapshot.lifecycle = ServerLifecyclePhase::Running)
        .expect("status update should succeed");
    let (address, _handle) = start_mascot_control_server_on(
        SocketAddr::from(([127, 0, 0, 1], 0)),
        tx,
        status_store,
        empty_psd_file_names,
    )
    .expect("should start mascot control server");
    wait_for_healthcheck(address);

    let status =
        mascot_render_server_status_at(address).expect("status request should return JSON");

    assert_eq!(status.lifecycle, ServerLifecyclePhase::Running);
    assert_eq!(
        status.displayed_png_path,
        PathBuf::from("cache/demo/open.png")
    );
}

#[test]
fn mascot_control_server_reports_psd_file_names() {
    let (tx, _rx) = mpsc::channel();
    let expected = vec!["body.psd".to_string(), "face.psd".to_string()];
    let (address, _handle) = start_mascot_control_server_on(
        SocketAddr::from(([127, 0, 0, 1], 0)),
        tx,
        test_status_store(),
        {
            let expected = expected.clone();
            move || Ok(expected.clone())
        },
    )
    .expect("should start mascot control server");
    wait_for_healthcheck(address);

    let psd_file_names = mascot_render_server_psd_file_names_at(address)
        .expect("PSD file names request should return JSON");

    assert_eq!(psd_file_names, expected);
}

#[test]
fn mascot_control_server_records_queued_command_status() {
    let (tx, _rx) = mpsc::channel();
    let status_store = test_status_store();
    let (address, _handle) = start_mascot_control_server_on(
        SocketAddr::from(([127, 0, 0, 1], 0)),
        tx,
        status_store.clone(),
        empty_psd_file_names,
    )
    .expect("should start mascot control server");
    wait_for_healthcheck(address);

    show_mascot_render_server_at(address).expect("show request should succeed");
    let status = status_store.snapshot().expect("status should be readable");

    assert_eq!(
        status
            .current_command
            .expect("current command should be recorded")
            .stage,
        ServerCommandStage::Queued
    );
}

fn wait_for_healthcheck(address: SocketAddr) {
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
        if mascot_render_server_healthcheck_at(address).is_ok() {
            return;
        }
        std::thread::sleep(Duration::from_millis(20));
    }

    panic!("mascot control server did not become healthy at {address}");
}

fn wait_for_notify(notified: &AtomicUsize) {
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
        if notified.load(Ordering::SeqCst) > 0 {
            return;
        }
        std::thread::sleep(Duration::from_millis(20));
    }

    panic!("mascot control server did not notify ui");
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
