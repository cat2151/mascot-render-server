use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::mascot_control::{
    start_mascot_control_server_on, start_mascot_control_server_on_with_notify,
    MascotControlCommand,
};
use mascot_render_client::{
    change_skin_mascot_render_server_at, hide_mascot_render_server_at,
    mascot_render_server_healthcheck_at, play_timeline_mascot_render_server_at,
    show_mascot_render_server_at, MotionTimelineKind, MotionTimelineRequest, MotionTimelineStep,
};

#[test]
fn mascot_control_server_accepts_show_hide_change_skin_and_timeline() {
    let (tx, rx) = mpsc::channel();
    let (address, _handle) =
        start_mascot_control_server_on(SocketAddr::from(([127, 0, 0, 1], 0)), tx)
            .expect("should start mascot control server");
    wait_for_healthcheck(address);

    show_mascot_render_server_at(address).expect("show request should succeed");
    assert_eq!(
        rx.recv_timeout(Duration::from_secs(1))
            .expect("show command should arrive"),
        MascotControlCommand::Show
    );

    let preview_path = PathBuf::from("cache/demo/variation.png");
    change_skin_mascot_render_server_at(address, &preview_path)
        .expect("change-skin request should succeed");
    assert_eq!(
        rx.recv_timeout(Duration::from_secs(1))
            .expect("change-skin command should arrive"),
        MascotControlCommand::ChangeSkin(preview_path)
    );

    let timeline = MotionTimelineRequest {
        steps: vec![MotionTimelineStep {
            kind: MotionTimelineKind::Shake,
            duration_ms: 5_000,
            fps: 20,
        }],
    };
    play_timeline_mascot_render_server_at(address, &timeline)
        .expect("timeline request should succeed");
    assert_eq!(
        rx.recv_timeout(Duration::from_secs(1))
            .expect("timeline command should arrive"),
        MascotControlCommand::PlayTimeline(timeline)
    );

    hide_mascot_render_server_at(address).expect("hide request should succeed");
    assert_eq!(
        rx.recv_timeout(Duration::from_secs(1))
            .expect("hide command should arrive"),
        MascotControlCommand::Hide
    );
}

#[test]
fn mascot_control_server_reports_health() {
    let (tx, _rx) = mpsc::channel();
    let (address, _handle) =
        start_mascot_control_server_on(SocketAddr::from(([127, 0, 0, 1], 0)), tx)
            .expect("should start mascot control server");
    wait_for_healthcheck(address);

    mascot_render_server_healthcheck_at(address).expect("healthcheck should succeed");
}

#[test]
fn mascot_control_server_bind_error_mentions_existing_server() {
    let (tx, _rx) = mpsc::channel();
    let (address, _handle) =
        start_mascot_control_server_on(SocketAddr::from(([127, 0, 0, 1], 0)), tx)
            .expect("should start mascot control server");

    let (tx2, _rx2) = mpsc::channel();
    let error = start_mascot_control_server_on(address, tx2)
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
        Some(notify),
    )
    .expect("should start mascot control server");
    wait_for_healthcheck(address);

    show_mascot_render_server_at(address).expect("show request should succeed");
    wait_for_notify(&notified);
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

