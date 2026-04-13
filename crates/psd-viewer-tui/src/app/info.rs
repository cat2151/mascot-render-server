use ratatui::style::Style;
use ratatui::text::{Line, Span};

use mascot_render_core::{display_path, mascot_config_path, mascot_runtime_state_path};

use crate::app::{App, PreviewBackend, MONOKAI_PINK, MONOKAI_YELLOW};
use crate::favorites::favorites_path;
use crate::tui_config::{tui_config_path, tui_runtime_state_path};

impl App {
    fn active_overlay_action_text(&self) -> &'static str {
        if self.help_overlay_visible {
            " | Esc: close help"
        } else if self.log_overlay.is_some() {
            " | Enter/Esc: close overlay"
        } else if self.favorites_visible {
            " | Esc: close favorites"
        } else {
            ""
        }
    }

    pub(crate) fn info_lines(&self) -> Vec<Line<'static>> {
        let mut lines = Vec::new();
        if let Some(zip) = self.selected_zip_entry() {
            lines.push(Line::from(format!(
                "Selected ZIP: {} | cache_key={}",
                display_path(&zip.zip_path),
                zip.zip_cache_key
            )));
        }

        if let Some(psd) = self.selected_psd_entry() {
            lines.push(Line::from(format!(
                "Selected PSD: {} | {}",
                psd.file_name, psd.metadata
            )));

            if let Some(rendered_png_path) = &psd.rendered_png_path {
                lines.push(Line::from(format!(
                    "Default PNG: {}",
                    display_path(rendered_png_path)
                )));
            }

            if let Some(error) = &psd.error {
                lines.push(Line::from(vec![
                    Span::styled("PSD Parse: ", Style::default().fg(MONOKAI_PINK)),
                    Span::raw(error.clone()),
                ]));
            } else if let Some(warning) = psd.render_warnings.first() {
                lines.push(Line::from(vec![
                    Span::styled("Render Warning: ", Style::default().fg(MONOKAI_YELLOW)),
                    Span::raw(warning.clone()),
                ]));
            }

            if let Some(log_path) = &psd.log_path {
                lines.push(Line::from(format!(
                    "Failure Log: {}",
                    display_path(log_path)
                )));
            }
        }

        if let Some(preview_png_path) = &self.current_preview_png_path {
            lines.push(Line::from(format!(
                "Preview PNG: {}",
                display_path(preview_png_path)
            )));
        }

        lines.push(Line::from(match self.preview_backend {
            PreviewBackend::MascotServer => {
                "Preview Backend: mascot-render-server window".to_string()
            }
            PreviewBackend::Sixel => "Preview Backend: sixel in TUI".to_string(),
        }));
        lines.push(Line::from(format!(
            "Mascot Scale: {} of original | TUI TOML: {}",
            self.mascot_scale
                .map(|scale| format!("{:.1}%", scale * 100.0))
                .unwrap_or_else(|| "auto".to_string()),
            display_path(&tui_config_path())
        )));
        lines.push(Line::from(format!(
            "TUI Runtime State: {}",
            display_path(&tui_runtime_state_path(&tui_config_path()))
        )));
        lines.push(Line::from(format!(
            "Favorites TOML: {}",
            display_path(&favorites_path())
        )));

        if let Some(variation_spec_path) = &self.current_variation_spec_path {
            lines.push(Line::from(format!(
                "Variation JSON: {}",
                display_path(variation_spec_path)
            )));
        }

        if let Some(node) = self.selected_layer_row() {
            lines.push(Line::from(format!(
                "Selected Node: {}",
                node.display_label().trim_start()
            )));
        }

        lines.push(Line::from(format!(
            "Mascot Render Server TOML: {}",
            display_path(&mascot_config_path())
        )));
        lines.push(Line::from(format!(
            "Mascot Runtime State: {}",
            display_path(&mascot_runtime_state_path(&mascot_config_path()))
        )));

        lines
    }

    pub(crate) fn log_lines(&self) -> Vec<Line<'static>> {
        vec![Line::from(vec![
            Span::styled("Message: ", Style::default().fg(MONOKAI_YELLOW)),
            Span::raw(self.status.clone()),
        ])]
    }

    pub(crate) fn help_line(&self) -> Line<'static> {
        let help_action = if self.help_overlay_visible {
            "close help"
        } else {
            "help"
        };
        Line::from(format!(
            "q: quit | ?: {help_action}{} | j/k: move | h/l: pane | PageUp/PageDown: scroll | Space/Enter: toggle | f: favorite | v: favorites | e: ensemble | -/+: mascot scale | t: mouth flap | m: eye blink | s: shake mascot",
            self.active_overlay_action_text(),
        ))
    }

    pub(crate) fn help_overlay_lines(&self) -> Vec<Line<'static>> {
        vec![
            Line::from("Press ? or Esc to close help."),
            Line::from(""),
            Line::from("q: quit"),
            Line::from("j/k or Up/Down: move selection"),
            Line::from("h/l or Left/Right: switch pane"),
            Line::from("PageUp/PageDown: page scroll"),
            Line::from("Space/Enter: toggle selected layer"),
            Line::from("f: save current PSD to favorites (ZIP / PSD or layer pane)"),
            Line::from(
                "v: open/close favorites list, Esc: close favorites list, Enter/Esc: close overlay",
            ),
            Line::from("e: toggle favorite ensemble true/false"),
            Line::from("Enter on favorites list: select PSD"),
            Line::from("-/+: mascot scale"),
            Line::from("t: mouth flap preview"),
            Line::from("m: eye blink preview"),
            Line::from("s: shake mascot"),
        ]
    }
}
