use std::path::Path;
use std::time::Instant;

use anyhow::{Context, Result};
use eframe::egui;
use mascot_render_control::{log_server_info, MascotControlCommand};
use mascot_render_server::apply_motion_timeline_request;

use super::config::describe_motion_timeline_request;
use super::logging::{change_skin_success_message, run_change_skin_stage};
use super::persistence::{persist_requested_skin_change, verify_persisted_skin_change};
use super::MascotApp;
use crate::app_support::{path_modified_at, size_vec};

impl MascotApp {
    pub(super) fn apply_control_commands(&mut self, ctx: &egui::Context) -> Result<()> {
        while let Ok(command) = self.control_rx.try_recv() {
            match command {
                MascotControlCommand::Show => {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                    log_server_info(
                        "trigger=control_command action=show サーバウィンドウを表示しました",
                    );
                }
                MascotControlCommand::Hide => {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                    log_server_info(
                        "trigger=control_command action=hide サーバウィンドウを非表示にしました",
                    );
                }
                MascotControlCommand::ChangeSkin(png_path) => {
                    self.change_skin(ctx, &png_path).with_context(|| {
                        format!(
                            "failed to apply mascot change-skin command: requested_png_path={}",
                            png_path.display()
                        )
                    })?;
                }
                MascotControlCommand::PlayTimeline(request) => {
                    let timeline_summary = describe_motion_timeline_request(&request);
                    apply_motion_timeline_request(
                        &mut self.motion,
                        self.window_layout,
                        Instant::now(),
                        request,
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
                }
            }
            ctx.request_repaint();
        }

        Ok(())
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
        self.open_skin = run_change_skin_stage(&previous_png_path, png_path, "load_skin", || {
            self.load_skin(ctx, png_path).with_context(|| {
                format!(
                    "failed to load requested mascot skin image {}",
                    png_path.display()
                )
            })
        })?;
        self.base_size = size_vec(
            self.open_skin.image_size[0],
            self.open_skin.image_size[1],
            Some(self.scale),
        );
        self.config.png_path = png_path.to_path_buf();
        self.eye_blink.reset(Instant::now());
        run_change_skin_stage(
            &previous_png_path,
            png_path,
            "refresh_closed_eye_skin",
            || {
                self.refresh_closed_eye_skin(ctx).with_context(|| {
                    format!(
                        "failed to refresh closed-eye skin after changing to {}",
                        png_path.display()
                    )
                })
            },
        )?;
        run_change_skin_stage(
            &previous_png_path,
            png_path,
            "refresh_mouth_flap_skins",
            || {
                self.refresh_mouth_flap_skins(ctx).with_context(|| {
                    format!(
                        "failed to refresh mouth-flap skins after changing to {}",
                        png_path.display()
                    )
                })
            },
        )?;
        log_server_info(super::logging::change_skin_stage_message(
            &previous_png_path,
            png_path,
            "refresh_window_layout",
        ));
        self.refresh_window_layout(ctx, previous_layout);
        run_change_skin_stage(
            &previous_png_path,
            png_path,
            "persist_runtime_state",
            || {
                persist_requested_skin_change(&self.config_path, &self.config, png_path)
                    .with_context(|| {
                        format!(
                            "failed to persist requested mascot skin to {}",
                            self.runtime_state_path.display()
                        )
                    })
            },
        )?;
        let persisted_png_path =
            run_change_skin_stage(&previous_png_path, png_path, "verify_runtime_state", || {
                verify_persisted_skin_change(&self.config_path, png_path).with_context(|| {
                    format!(
                        "failed to verify requested mascot skin in {}",
                        self.runtime_state_path.display()
                    )
                })
            })?;
        self.runtime_state_modified_at = path_modified_at(&self.runtime_state_path);
        log_server_info(change_skin_success_message(
            &previous_png_path,
            png_path,
            &self.runtime_state_path,
            &persisted_png_path,
        ));
        Ok(())
    }
}
