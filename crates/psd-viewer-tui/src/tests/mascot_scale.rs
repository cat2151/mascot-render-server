use std::path::{Path, PathBuf};

use mascot_render_core::MascotTarget;

use crate::app::sync_current_mascot_config_log_message_for_test;

#[test]
fn sync_current_mascot_config_log_message_reports_runtime_state_inputs() {
    let message = sync_current_mascot_config_log_message_for_test(
        Path::new("config/mascot-render-server.toml"),
        &MascotTarget {
            png_path: PathBuf::from("cache/demo.png"),
            scale: Some(0.145),
            ensemble_scale: None,
            zip_path: PathBuf::from("assets/demo.zip"),
            psd_path_in_zip: PathBuf::from("demo/body.psd"),
            display_diff_path: Some(PathBuf::from("cache/demo.json")),
        },
    );

    assert_eq!(
        message,
        "trigger=selection_sync action=sync_current_mascot_config mascot runtime stateを書き込みました: config_path=config/mascot-render-server.toml png_path=cache/demo.png zip_path=assets/demo.zip psd_path_in_zip=demo/body.psd display_diff_path=cache/demo.json scale=0.145"
    );
}
