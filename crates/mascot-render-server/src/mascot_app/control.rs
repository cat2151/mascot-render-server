use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{bail, Context, Result};
use eframe::egui;
use mascot_render_control::{log_server_info, MascotControlCommand};
use mascot_render_core::MascotConfig;
use mascot_render_server::apply_motion_timeline_request;

use super::character::{resolve_character_skin, ResolvedCharacterSkin};
use super::config::describe_motion_timeline_request;
use super::logging::{
    change_character_stage_message, change_character_success_message, run_change_character_stage,
};
use super::persistence::{persist_requested_character_change, verify_persisted_character_change};
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
            MascotControlCommand::ChangeCharacter { character_name, .. } => {
                self.change_character(ctx, character_name).with_context(|| {
                    format!(
                        "failed to apply mascot change-character command: requested_character={}",
                        character_name
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

    fn change_character(&mut self, ctx: &egui::Context, character_name: &str) -> Result<()> {
        if self.config.favorite_ensemble_enabled {
            let message = format!(
                "trigger=control_command action=change_character favorite_ensemble_enabled=true requested_character={} のため character変更できません",
                character_name
            );
            log_server_info(message);
            bail!("favorite_ensemble_enabled=true; cannot change character while favorite ensemble is active");
        }

        let resolved = resolve_character_skin(&self.core, character_name).with_context(|| {
            format!(
                "failed to resolve requested character: requested_character={} current_png={} current_zip={} current_psd={} current_display_diff={}",
                character_name,
                self.config.png_path.display(),
                self.config.zip_path.display(),
                self.config.psd_path_in_zip.display(),
                optional_path_text(self.config.display_diff_path.as_deref())
            )
        })?;
        log_server_info(format!(
            "trigger=control_command action=change_character requested_character={} candidate_count={} selected_zip={} selected_psd={} selected_png={} selected_display_diff={}",
            resolved.character_name,
            resolved.candidate_count,
            resolved.zip_path.display(),
            resolved.psd_path_in_zip.display(),
            resolved.png_path.display(),
            optional_path_text(resolved.display_diff_path.as_deref())
        ));

        if config_matches_resolved_character(&self.config, &resolved) {
            match verify_persisted_character_change(&self.config_path, &self.config) {
                Ok(persisted) => {
                    log_server_info(format!(
                        "trigger=control_command action=change_character character変更をスキップしました: requested_character={} selected_png={} は現在の character source と同じで runtime state も一致しています runtime_state_path={} persisted_png_path={} persisted_zip={} persisted_psd={}",
                        resolved.character_name,
                        resolved.png_path.display(),
                        self.runtime_state_path.display(),
                        persisted.png_path.display(),
                        persisted.zip_path.display(),
                        persisted.psd_path_in_zip.display()
                    ));
                    return Ok(());
                }
                Err(error) => {
                    log_server_info(format!(
                        "trigger=control_command action=change_character requested_character={} selected_png={} は現在の character source と同じですが runtime state の検証に失敗したため再試行します: runtime_state_path={} error={error:#}",
                        resolved.character_name,
                        resolved.png_path.display(),
                        self.runtime_state_path.display()
                    ));
                }
            }
        }

        let previous_png_path = self.config.png_path.clone();
        log_server_info(format!(
            "trigger=control_command action=change_character character変更を開始しました: requested_character={} from={} to={} selected_zip={} selected_psd={}",
            resolved.character_name,
            previous_png_path.display(),
            resolved.png_path.display(),
            resolved.zip_path.display(),
            resolved.psd_path_in_zip.display()
        ));
        let previous_layout = self.window_layout;
        let prepared = self.prepare_character_change(ctx, &previous_png_path, &resolved)?;
        let persisted_png_path = prepared.persisted_png_path.clone();
        self.commit_character_change(ctx, previous_layout, &previous_png_path, prepared);
        log_server_info(change_character_success_message(
            &previous_png_path,
            &resolved.png_path,
            &self.runtime_state_path,
            &persisted_png_path,
        ));
        Ok(())
    }

    fn prepare_character_change(
        &mut self,
        ctx: &egui::Context,
        previous_png_path: &Path,
        resolved: &ResolvedCharacterSkin,
    ) -> Result<PreparedSkinChange> {
        let mut next_config = self.config.clone();
        apply_resolved_character(&mut next_config, resolved);

        let open_skin = run_change_character_stage(
            previous_png_path,
            &next_config.png_path,
            "load_base_skin",
            || {
                self.load_skin(ctx, &next_config.png_path).with_context(|| {
                    format!(
                        "failed to load requested mascot skin image {}",
                        next_config.png_path.display()
                    )
                })
            },
        )?;
        let closed_skin = run_change_character_stage(
            previous_png_path,
            &next_config.png_path,
            "refresh_closed_eye_skin",
            || {
                self.load_closed_eye_skin_for_config(ctx, &next_config)
                    .with_context(|| {
                        format!(
                            "failed to refresh closed-eye skin after changing to {}",
                            next_config.png_path.display()
                        )
                    })
            },
        )?;
        log_server_info(format!(
            "trigger=control_command action=change_character stage=refresh_closed_eye_skin selected_zip={} selected_psd={} derived_png_path={}",
            next_config.zip_path.display(),
            next_config.psd_path_in_zip.display(),
            optional_cached_skin_path(closed_skin.as_ref())
        ));
        let (mouth_open_skin, mouth_closed_skin) = run_change_character_stage(
            previous_png_path,
            &next_config.png_path,
            "refresh_mouth_flap_skins",
            || {
                self.load_mouth_flap_skins_for_config(ctx, &next_config)
                    .with_context(|| {
                        format!(
                            "failed to refresh mouth-flap skins after changing to {}",
                            next_config.png_path.display()
                        )
                    })
            },
        )?;
        log_server_info(format!(
            "trigger=control_command action=change_character stage=refresh_mouth_flap_skins selected_zip={} selected_psd={} derived_open_png_path={} derived_closed_png_path={}",
            next_config.zip_path.display(),
            next_config.psd_path_in_zip.display(),
            optional_cached_skin_path(mouth_open_skin.as_ref()),
            optional_cached_skin_path(mouth_closed_skin.as_ref())
        ));
        run_change_character_stage(
            previous_png_path,
            &next_config.png_path,
            "persist_runtime_state",
            || {
                persist_requested_character_change(&self.config_path, &next_config).with_context(
                    || {
                        format!(
                            "failed to persist requested mascot character to {}",
                            self.runtime_state_path.display()
                        )
                    },
                )
            },
        )?;
        let persisted = run_change_character_stage(
            previous_png_path,
            &next_config.png_path,
            "verify_runtime_state",
            || {
                verify_persisted_character_change(&self.config_path, &next_config).with_context(
                    || {
                        format!(
                            "failed to verify requested mascot character in {}",
                            self.runtime_state_path.display()
                        )
                    },
                )
            },
        )?;

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
            persisted_png_path: persisted.png_path,
        })
    }

    fn commit_character_change(
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
        log_server_info(change_character_stage_message(
            previous_png_path,
            &next_png_path,
            "refresh_window_layout",
        ));
        self.refresh_window_layout(ctx, previous_layout);
    }
}

fn apply_resolved_character(config: &mut MascotConfig, resolved: &ResolvedCharacterSkin) {
    config.png_path = resolved.png_path.clone();
    config.zip_path = resolved.zip_path.clone();
    config.psd_path_in_zip = resolved.psd_path_in_zip.clone();
    config.display_diff_path = resolved.display_diff_path.clone();
}

fn config_matches_resolved_character(
    config: &MascotConfig,
    resolved: &ResolvedCharacterSkin,
) -> bool {
    config.png_path == resolved.png_path
        && config.zip_path == resolved.zip_path
        && config.psd_path_in_zip == resolved.psd_path_in_zip
        && config.display_diff_path == resolved.display_diff_path
}

fn optional_cached_skin_path(skin: Option<&CachedSkin>) -> String {
    skin.map(|skin| skin.path.display().to_string())
        .unwrap_or_else(|| "-".to_string())
}

fn optional_path_text(path: Option<&Path>) -> String {
    path.map(|path| path.display().to_string())
        .unwrap_or_else(|| "-".to_string())
}
