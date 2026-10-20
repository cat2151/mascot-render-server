use std::net::SocketAddr;
use std::path::Path;

use anyhow::Result;
use mascot_render_client::{
    mascot_render_server_address, mascot_render_server_healthcheck,
    preview_target_mascot_render_server_at, set_single_character_mode_mascot_render_server_at,
    show_mascot_render_server_at,
};
use mascot_render_control::ensure_mascot_render_server_running;
use mascot_render_core::{
    set_mascot_ensemble_mode, write_mascot_config, MascotEnsembleMode, MascotTarget,
};
use mascot_render_protocol::PreviewTargetRequest;

pub(crate) fn sync_preview_session(
    config_path: &Path,
    target: &PreviewTargetRequest,
    initialize: bool,
) -> Result<()> {
    let needs_startup = mascot_render_server_healthcheck().is_err();
    if needs_startup {
        prepare_preview_startup(config_path, target)?;
    }
    ensure_mascot_render_server_running(config_path)?;
    sync_running_preview_session(
        mascot_render_server_address(),
        target,
        initialize || needs_startup,
    )
}

pub(crate) fn sync_running_preview_session(
    address: SocketAddr,
    target: &PreviewTargetRequest,
    initialize: bool,
) -> Result<()> {
    if initialize {
        // Wait for application, including when an existing VPT server is reused.
        set_single_character_mode_mascot_render_server_at(address)?;
    }
    preview_target_mascot_render_server_at(address, target)?;
    show_mascot_render_server_at(address)
}

pub(crate) fn prepare_preview_startup(
    config_path: &Path,
    target: &PreviewTargetRequest,
) -> Result<()> {
    // The server loads both files before opening its control endpoint. Seed them
    // from the selected PSD rather than requiring a previous server session.
    write_mascot_config(
        config_path,
        &MascotTarget {
            png_path: target.png_path.clone(),
            scale: target.scale,
            ensemble_scale: None,
            zip_path: target.zip_path.clone(),
            psd_path_in_zip: target.psd_path_in_zip.clone(),
            display_diff_path: target.display_diff_path.clone(),
        },
    )?;
    set_mascot_ensemble_mode(config_path, MascotEnsembleMode::SingleCharacter)
}
