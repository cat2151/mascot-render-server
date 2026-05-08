use std::path::Path;

use mascot_render_control::log_server_skin_info;

use super::super::MascotApp;
use super::control_command::rendered_skin_message;
use super::should_log_rendered_skin;

impl MascotApp {
    pub(in crate::mascot_app) fn log_rendered_skin_if_changed(&mut self, png_path: &Path) {
        if !should_log_rendered_skin(self.last_logged_skin_path.as_deref(), png_path) {
            return;
        }
        self.last_logged_skin_path = Some(png_path.to_path_buf());
        log_server_skin_info(rendered_skin_message(png_path));
    }

    pub(in crate::mascot_app) fn clear_last_logged_skin_path(&mut self) {
        super::clear_rendered_skin_path(&mut self.last_logged_skin_path);
    }
}
