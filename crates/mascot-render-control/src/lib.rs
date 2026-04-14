mod command;
mod http_server;
mod logging;
mod orchestration;
mod paths;
mod spawn;

#[cfg(test)]
mascot_render_test_support::install_test_data_root!();

#[cfg(test)]
mod tests;

pub use command::MascotControlCommand;
pub use http_server::{start_mascot_control_server, start_mascot_control_server_with_notify};
pub use logging::{
    init_server_log, log_psd_viewer_tui_info, log_server_error, log_server_info,
    log_server_performance_error, log_server_performance_info, log_server_skin_info,
    server_performance_log_path,
};
pub use orchestration::{
    ensure_mascot_render_server_running, ensure_mascot_render_server_visible,
    play_mascot_render_server_timeline, sync_mascot_render_server_preview,
};
