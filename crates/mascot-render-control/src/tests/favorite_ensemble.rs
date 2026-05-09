use std::net::SocketAddr;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use mascot_render_client::{
    disable_favorite_ensemble_mascot_render_server_at, mascot_render_server_healthcheck_at,
};
use mascot_render_protocol::{
    ServerCommandKind, ServerCommandStage, ServerStatusSnapshot, ServerStatusStore,
};

use crate::command::MascotControlCommand;
use crate::http_server::start_mascot_control_server_on;

#[test]
fn mascot_control_server_accepts_disable_favorite_ensemble() {
    let (tx, rx) = mpsc::channel();
    let status_store = test_status_store();
    let (address, _handle) = start_mascot_control_server_on(
        SocketAddr::from(([127, 0, 0, 1], 0)),
        tx,
        status_store.clone(),
        empty_psd_file_names,
    )
    .expect("should start mascot control server");
    wait_for_healthcheck(address);

    let request_thread =
        thread::spawn(move || disable_favorite_ensemble_mascot_render_server_at(address));
    let command = rx
        .recv_timeout(Duration::from_secs(1))
        .expect("disable favorite ensemble command should arrive");

    assert_eq!(command, MascotControlCommand::disable_favorite_ensemble());
    let status = status_store.snapshot().expect("status should be readable");
    let current = status
        .current_command
        .expect("disable favorite ensemble should be current command");
    assert_eq!(current.kind, ServerCommandKind::DisableFavoriteEnsemble);
    assert_eq!(current.stage, ServerCommandStage::Queued);

    command.finish(Ok(()));
    request_thread
        .join()
        .expect("disable request thread should complete")
        .expect("disable favorite ensemble request should succeed");
}

#[test]
fn mascot_control_server_reports_disable_favorite_ensemble_apply_failure_to_http_caller() {
    let (tx, rx) = mpsc::channel();
    let (address, _handle) = start_mascot_control_server_on(
        SocketAddr::from(([127, 0, 0, 1], 0)),
        tx,
        test_status_store(),
        empty_psd_file_names,
    )
    .expect("should start mascot control server");
    wait_for_healthcheck(address);

    let request_thread =
        thread::spawn(move || disable_favorite_ensemble_mascot_render_server_at(address));
    let command = rx
        .recv_timeout(Duration::from_secs(1))
        .expect("disable favorite ensemble command should arrive");

    assert_eq!(command, MascotControlCommand::disable_favorite_ensemble());
    command.finish(Err(
        "failed to disable favorite ensemble for test".to_string()
    ));

    let error = request_thread
        .join()
        .expect("disable request thread should complete")
        .expect_err("disable favorite ensemble request should report apply failure");
    assert!(
        error.to_string().contains("HTTP 500"),
        "unexpected error: {error:#}"
    );
    assert!(
        error
            .to_string()
            .contains("failed to disable favorite ensemble for test"),
        "unexpected error: {error:#}"
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

fn test_status_store() -> ServerStatusStore {
    ServerStatusStore::new(ServerStatusSnapshot::starting(
        "config/mascot-render-server.toml".into(),
        "config/mascot-render-server.runtime.json".into(),
        "cache/demo/open.png".into(),
        "assets/zip/demo.zip".into(),
        "demo/basic.psd".into(),
    ))
}

fn empty_psd_file_names() -> anyhow::Result<Vec<String>> {
    Ok(Vec::new())
}
