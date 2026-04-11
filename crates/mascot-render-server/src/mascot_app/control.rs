use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{Context, Result};
use eframe::egui;
use mascot_render_control::{log_server_info, MascotControlCommand};
use mascot_render_core::MascotConfig;
use mascot_render_server::apply_motion_timeline_request;

use super::config::describe_motion_timeline_request;
use super::logging::{
    change_skin_stage_message, change_skin_success_message, run_change_skin_stage,
};
use super::persistence::{persist_requested_skin_change, verify_persisted_skin_change};
use super::{CachedSkin, MascotApp};
use crate::app_support::{path_modified_at, size_vec};

struct PreparedSkinChange {
    next_config: MascotConfig,
    open_skin: CachedSkin,
    closed_skin: Option<CachedSkin>,
    mouth_open_skin: Option<CachedSkin>,
    mouth_closed_skin: Option<CachedSkin>,
    base_size: egui::Vec2,
    persisted_png_path: PathBuf,
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
                log_server_info(
                    "trigger=control_command action=show サーバウィンドウを表示しました",
                );
                Ok(())
            }
            MascotControlCommand::Hide { .. } => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                log_server_info(
                    "trigger=control_command action=hide サーバウィンドウを非表示にしました",
                );
                Ok(())
            }
            MascotControlCommand::ChangeSkin { png_path, .. } => {
                self.change_skin(ctx, png_path).with_context(|| {
                    format!(
                        "failed to apply mascot change-skin command: requested_png_path={}",
                        png_path.display()
                    )
                })
            }
            MascotControlCommand::PlayTimeline { request, .. } => {
                let timeline_summary = describe_motion_timeline_request(request);
                apply_motion_timeline_request(
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
                })?;
                log_server_info(format!(
                    "trigger=control_command action=timeline {}",
                    timeline_summary
                ));
                Ok(())
            }
        }
    }

    fn change_skin(&mut self, ctx: &egui::Context, png_path: &Path) -> Result<()> {
        if self.config.favorite_ensemble_enabled {
            log_server_info(format!(
                "trigger=control_command action=change_skin skin変更をスキップしました: favorite_ensemble_enabled=true requested_png_path={}",
                png_path.display()
            ));
            return Ok(());
        }
        if self.config.png_path == png_path {
            match verify_persisted_skin_change(&self.config_path, png_path) {
                Ok(persisted_png_path) => {
                    log_server_info(format!(
                        "trigger=control_command action=change_skin skin変更をスキップしました: requested_png_path={} は現在の skin と同じで runtime state も一致しています runtime_state_path={} persisted_png_path={}",
                        png_path.display(),
                        self.runtime_state_path.display(),
                        persisted_png_path.display()
                    ));
                    return Ok(());
                }
                Err(error) => {
                    log_server_info(format!(
                        "trigger=control_command action=change_skin requested_png_path={} は現在の skin と同じですが runtime state の検証に失敗したため再試行します: runtime_state_path={} error={error:#}",
                        png_path.display(),
                        self.runtime_state_path.display()
                    ));
                }
            }
        }

        let previous_png_path = self.config.png_path.clone();
        log_server_info(format!(
            "trigger=control_command action=change_skin skin変更を開始しました: from={} to={}",
            previous_png_path.display(),
            png_path.display()
        ));
        let previous_layout = self.window_layout;
        let prepared = self.prepare_skin_change(ctx, &previous_png_path, png_path)?;
        let persisted_png_path = prepared.persisted_png_path.clone();
        self.commit_skin_change(ctx, previous_layout, &previous_png_path, prepared);
        log_server_info(change_skin_success_message(
            &previous_png_path,
            png_path,
            &self.runtime_state_path,
            &persisted_png_path,
        ));
        Ok(())
    }

    fn prepare_skin_change(
        &mut self,
        ctx: &egui::Context,
        previous_png_path: &Path,
        png_path: &Path,
    ) -> Result<PreparedSkinChange> {
        let open_skin = run_change_skin_stage(previous_png_path, png_path, "load_skin", || {
            self.load_skin(ctx, png_path).with_context(|| {
                format!(
                    "failed to load requested mascot skin image {}",
                    png_path.display()
                )
            })
        })?;
        let mut next_config = self.config.clone();
        next_config.png_path = png_path.to_path_buf();
        let closed_skin = run_change_skin_stage(
            previous_png_path,
            png_path,
            "refresh_closed_eye_skin",
            || {
                self.load_closed_eye_skin_for_config(ctx, &next_config)
                    .with_context(|| {
                        format!(
                            "failed to refresh closed-eye skin after changing to {}",
                            png_path.display()
                        )
                    })
            },
        )?;
        let (mouth_open_skin, mouth_closed_skin) = run_change_skin_stage(
            previous_png_path,
            png_path,
            "refresh_mouth_flap_skins",
            || {
                self.load_mouth_flap_skins_for_config(ctx, &next_config)
                    .with_context(|| {
                        format!(
                            "failed to refresh mouth-flap skins after changing to {}",
                            png_path.display()
                        )
                    })
            },
        )?;
        run_change_skin_stage(previous_png_path, png_path, "persist_runtime_state", || {
            persist_requested_skin_change(&self.config_path, &next_config, png_path).with_context(
                || {
                    format!(
                        "failed to persist requested mascot skin to {}",
                        self.runtime_state_path.display()
                    )
                },
            )
        })?;
        let persisted_png_path =
            run_change_skin_stage(previous_png_path, png_path, "verify_runtime_state", || {
                verify_persisted_skin_change(&self.config_path, png_path).with_context(|| {
                    format!(
                        "failed to verify requested mascot skin in {}",
                        self.runtime_state_path.display()
                    )
                })
            })?;

        Ok(PreparedSkinChange {
            base_size: size_vec(
                open_skin.image_size[0],
                open_skin.image_size[1],
                Some(self.scale),
            ),
            next_config,
            open_skin,
            closed_skin,
            mouth_open_skin,
            mouth_closed_skin,
            persisted_png_path,
        })
    }

    fn commit_skin_change(
        &mut self,
        ctx: &egui::Context,
        previous_layout: mascot_render_server::MascotWindowLayout,
        previous_png_path: &Path,
        prepared: PreparedSkinChange,
    ) {
        let next_png_path = prepared.next_config.png_path.clone();
        self.config = prepared.next_config;
        self.open_skin = prepared.open_skin;
        self.closed_skin = prepared.closed_skin;
        self.mouth_open_skin = prepared.mouth_open_skin;
        self.mouth_closed_skin = prepared.mouth_closed_skin;
        self.base_size = prepared.base_size;
        self.eye_blink.reset(Instant::now());
        self.runtime_state_modified_at = path_modified_at(&self.runtime_state_path);
        log_server_info(change_skin_stage_message(
            previous_png_path,
            &next_png_path,
            "refresh_window_layout",
        ));
        self.refresh_window_layout(ctx, previous_layout);
    }
}
