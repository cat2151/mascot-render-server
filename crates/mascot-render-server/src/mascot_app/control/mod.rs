use std::time::Instant;

use anyhow::{Context, Result};
use eframe::egui;
use mascot_render_control::{log_server_info, MascotControlCommand};
use mascot_render_core::MascotConfig;
use mascot_render_protocol::{MotionTimelineKind, MotionTimelineRequest};
use mascot_render_server::apply_motion_timeline_request;

use super::{CachedSkin, MascotApp};

mod character_change;
mod commit;
mod favorite_ensemble;
mod preview_target;

pub(super) struct PreparedSkinChange {
    next_config: MascotConfig,
    open_skin: CachedSkin,
    closed_skin: Option<CachedSkin>,
    mouth_open_skin: Option<CachedSkin>,
    mouth_closed_skin: Option<CachedSkin>,
    placement: super::placement::PreparedPlacementChange,
    persisted_png_path: std::path::PathBuf,
}

impl MascotApp {
    pub(super) fn apply_control_commands(&mut self, ctx: &egui::Context) -> Result<()> {
        let mut first_error = None;

        while let Ok(command) = self.control_rx.try_recv() {
            self.record_command_applying(&command);
            let result = self.apply_control_command(ctx, &command);
            match &result {
                Ok(()) => self.record_command_applied(&command),
                Err(error) => self.record_command_failed(&command, format!("{error:#}")),
            }
            command.finish(
                result
                    .as_ref()
                    .map(|_| ())
                    .map_err(|error| format!("{error:#}")),
            );
            if first_error.is_none() {
                if let Err(error) = result {
                    first_error = Some(error);
                }
            }
            ctx.request_repaint();
        }

        match first_error {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }

    fn apply_control_command(
        &mut self,
        ctx: &egui::Context,
        command: &MascotControlCommand,
    ) -> Result<()> {
        match command {
            MascotControlCommand::Show { .. } => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                log_server_info("trigger=control_command action=show サーバウィンドウを表示しました");
                Ok(())
            }
            MascotControlCommand::Hide { .. } => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                log_server_info("trigger=control_command action=hide サーバウィンドウを非表示にしました");
                Ok(())
            }
            MascotControlCommand::ChangeCharacter { character_name, .. } => {
                self.change_character(ctx, character_name).with_context(|| {
                    format!(
                        "failed to apply mascot change-character command: requested_character={}",
                        character_name
                    )
                })
            }
            MascotControlCommand::PreviewTarget { request, .. } => {
                self.apply_preview_target(ctx, request).with_context(|| {
                    format!(
                        "failed to apply mascot preview-target command: png_path={} zip_path={} psd_path_in_zip={}",
                        request.png_path.display(),
                        request.zip_path.display(),
                        request.psd_path_in_zip.display()
                    )
                })
            }
            MascotControlCommand::PlayTimeline { request, .. } => {
                let timeline_summary = super::config::describe_motion_timeline_request(request);
                if request_contains_mouth_flap(request) {
                    let refresh_started_at = Instant::now();
                    self.ensure_mouth_flap_skins_for_timeline(ctx)
                        .with_context(|| {
                            format!(
                                "failed to prepare mouth-flap skins before applying timeline: {}",
                                timeline_summary
                            )
                        })?;
                    self.record_performance_stage(
                        "ensure_mouth_flap_skins_for_timeline",
                        elapsed_ms_since(refresh_started_at),
                    );
                }
                let stage_started_at = Instant::now();
                let result = apply_motion_timeline_request(
                    &mut self.motion,
                    self.window_layout,
                    Instant::now(),
                    request.clone(),
                )
                .with_context(|| {
                    format!(
                        "failed to apply mascot motion timeline command: {}",
                        timeline_summary
                    )
                });
                self.record_performance_stage(
                    "apply_motion_timeline_request",
                    elapsed_ms_since(stage_started_at),
                );
                result?;
                log_server_info(format!(
                    "trigger=control_command action=timeline {}",
                    timeline_summary
                ));
                Ok(())
            }
            MascotControlCommand::DisableFavoriteEnsemble { .. } => {
                self.disable_favorite_ensemble(ctx)
            }
        }
    }

    fn ensure_mouth_flap_skins_for_timeline(&mut self, ctx: &egui::Context) -> Result<()> {
        if self.config.favorite_ensemble_enabled
            || (self.mouth_open_skin.is_some() && self.mouth_closed_skin.is_some())
        {
            return Ok(());
        }

        let config = self.config.clone();
        let (mouth_open_skin, mouth_closed_skin) =
            self.load_mouth_flap_skins_for_config(ctx, &config)?;
        self.mouth_open_skin = mouth_open_skin;
        self.mouth_closed_skin = mouth_closed_skin;
        Ok(())
    }
}

fn request_contains_mouth_flap(request: &MotionTimelineRequest) -> bool {
    request
        .steps
        .iter()
        .any(|step| step.kind == MotionTimelineKind::MouthFlap)
}

fn elapsed_ms_since(started_at: Instant) -> u64 {
    u64::try_from(started_at.elapsed().as_millis()).unwrap_or(u64::MAX)
}

fn optional_path_text(path: Option<&std::path::Path>) -> String {
    path.map(|path| path.display().to_string())
        .unwrap_or_else(|| "-".to_string())
}
