use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use eframe::egui;
use mascot_render_control::{log_server_info, MascotControlCommand};
use mascot_render_core::{MascotConfig, MascotEnsembleMode};
use mascot_render_protocol::{MotionTimelineKind, MotionTimelineRequest};
use mascot_render_server::apply_motion_timeline_request;

use super::{CachedSkin, MascotApp};

mod character_change;
mod commit;
mod ensemble_mode;
mod preview_target;
mod vpt_ensemble;

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
                let now = Instant::now();
                if let Some(target_character_name) = request.target_character_name.as_deref() {
                    let stage_started_at = Instant::now();
                    if self.apply_targeted_mouth_flap_timeline(
                        request,
                        target_character_name,
                        now,
                    ) {
                        self.record_performance_stage(
                            "apply_targeted_mouth_flap_timeline",
                            elapsed_ms_since(stage_started_at),
                        );
                        log_server_info(format!(
                            "trigger=control_command action=timeline {}",
                            timeline_summary
                        ));
                        return Ok(());
                    }
                }
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
                    now,
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
            MascotControlCommand::SetEnsembleMode { mode, .. } => {
                self.set_ensemble_mode_from_control(ctx, *mode)
            }
            MascotControlCommand::SetVptEnsemble { request, .. } => {
                self.set_vpt_ensemble(ctx, request)
            }
            MascotControlCommand::SetVptEnsembleMembers { request, .. } => {
                self.set_vpt_ensemble_members(ctx, request)
            }
        }
    }

    fn apply_targeted_mouth_flap_timeline(
        &mut self,
        request: &MotionTimelineRequest,
        target_character_name: &str,
        now: Instant,
    ) -> bool {
        let Some(step) = request.steps.first() else {
            return false;
        };
        if step.kind != MotionTimelineKind::MouthFlap {
            return false;
        }
        if !should_consume_targeted_mouth_flap_timeline(request, self.config.ensemble_mode) {
            return false;
        }

        let Some(ensemble_scene) = self.ensemble_scene.as_mut() else {
            log_server_info(format!(
                "trigger=control_command action=timeline target_character_name={} result=noop reason=vpt_member_not_found ensemble_mode={:?} member_count=0",
                target_character_name,
                self.config.ensemble_mode
            ));
            return true;
        };
        let member_count = ensemble_scene.members.len();
        let triggered = ensemble_scene.trigger_mouth_flap_for_character(
            target_character_name,
            now,
            Duration::from_millis(step.duration_ms),
            step.fps,
        );
        if triggered {
            log_server_info(format!(
                "trigger=control_command action=timeline target_character_name={} result=targeted_mouth_flap member_count={member_count}",
                target_character_name
            ));
        } else {
            log_server_info(format!(
                "trigger=control_command action=timeline target_character_name={} result=noop reason=vpt_member_not_found ensemble_mode={:?} member_count={member_count}",
                target_character_name,
                self.config.ensemble_mode
            ));
        }
        true
    }

    fn ensure_mouth_flap_skins_for_timeline(&mut self, ctx: &egui::Context) -> Result<()> {
        if self.config.ensemble_mode.is_ensemble()
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

fn should_consume_targeted_mouth_flap_timeline(
    request: &MotionTimelineRequest,
    ensemble_mode: MascotEnsembleMode,
) -> bool {
    request.target_character_name.is_some()
        && matches!(ensemble_mode, MascotEnsembleMode::Vpt)
        && request
            .steps
            .first()
            .is_some_and(|step| step.kind == MotionTimelineKind::MouthFlap)
}

#[cfg(test)]
pub(crate) fn should_consume_targeted_mouth_flap_timeline_for_test(
    request: &MotionTimelineRequest,
    ensemble_mode: MascotEnsembleMode,
) -> bool {
    should_consume_targeted_mouth_flap_timeline(request, ensemble_mode)
}

fn elapsed_ms_since(started_at: Instant) -> u64 {
    u64::try_from(started_at.elapsed().as_millis()).unwrap_or(u64::MAX)
}

fn optional_path_text(path: Option<&std::path::Path>) -> String {
    path.map(|path| path.display().to_string())
        .unwrap_or_else(|| "-".to_string())
}
