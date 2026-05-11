use std::net::SocketAddr;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use mascot_render_client::{
    mascot_render_server_healthcheck_at, set_single_character_mode_mascot_render_server_at,
    set_vpt_ensemble_mascot_render_server_at,
};
use mascot_render_protocol::{
    ServerCommandKind, ServerCommandStage, ServerEnsembleMode, ServerStatusSnapshot,
    ServerStatusStore, VptEnsembleRequest,
};

use crate::command::MascotControlCommand;
use crate::http_server::start_mascot_control_server_on;

#[test]
fn mascot_control_server_accepts_set_single_character_mode() {
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
        thread::spawn(move || set_single_character_mode_mascot_render_server_at(address));
    let command = rx
        .recv_timeout(Duration::from_secs(1))
        .expect("set ensemble mode command should arrive");

    assert_eq!(
        command,
        MascotControlCommand::set_ensemble_mode(ServerEnsembleMode::SingleCharacter)
    );
    let status = status_store.snapshot().expect("status should be readable");
    let current = status
        .current_command
        .expect("set ensemble mode should be current command");
    assert_eq!(current.kind, ServerCommandKind::SetEnsembleMode);
    assert_eq!(current.stage, ServerCommandStage::Queued);

    command.finish(Ok(()));
    request_thread
        .join()
        .expect("set mode request thread should complete")
        .expect("set single character mode request should succeed");
}

#[test]
fn mascot_control_server_accepts_set_vpt_ensemble() {
    let (tx, rx) = mpsc::channel();
    let (address, _handle) = start_mascot_control_server_on(
        SocketAddr::from(([127, 0, 0, 1], 0)),
        tx,
        test_status_store(),
        empty_psd_file_names,
    )
    .expect("should start mascot control server");
    wait_for_healthcheck(address);

    let character_names = vec!["ずんだもん".to_string(), "四国めたん".to_string()];
    let request_thread = {
        let character_names = character_names.clone();
        thread::spawn(move || set_vpt_ensemble_mascot_render_server_at(address, &character_names))
    };
    let command = rx
        .recv_timeout(Duration::from_secs(1))
        .expect("set vpt ensemble command should arrive");

    assert_eq!(
        command,
        MascotControlCommand::set_vpt_ensemble(VptEnsembleRequest { character_names })
    );

    command.finish(Ok(()));
    request_thread
        .join()
        .expect("set vpt ensemble request thread should complete")
        .expect("set vpt ensemble request should succeed");
}

#[test]
fn mascot_control_server_reports_set_ensemble_mode_apply_failure_to_http_caller() {
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
        thread::spawn(move || set_single_character_mode_mascot_render_server_at(address));
    let command = rx
        .recv_timeout(Duration::from_secs(1))
        .expect("set ensemble mode command should arrive");

    assert_eq!(
        command,
        MascotControlCommand::set_ensemble_mode(ServerEnsembleMode::SingleCharacter)
    );
    command.finish(Err("failed to set ensemble mode for test".to_string()));

    let error = request_thread
        .join()
        .expect("set mode request thread should complete")
        .expect_err("set ensemble mode request should report apply failure");
    assert!(
        error.to_string().contains("HTTP 500"),
        "unexpected error: {error:#}"
    );
    assert!(
        error
            .to_string()
            .contains("failed to set ensemble mode for test"),
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
