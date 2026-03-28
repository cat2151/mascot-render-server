use std::time::{Duration, Instant};

use anyhow::Result;
use eframe::egui;

use super::MascotApp;
use crate::app_support::{path_modified_at, size_vec};
use crate::mascot_scale::{
    adjust_scale, persist_favorite_ensemble_scale, persist_scale, SCALE_PERSIST_DEBOUNCE,
};

impl MascotApp {
    pub(super) fn apply_scale_steps(
        &mut self,
        ctx: &egui::Context,
        now: Instant,
        steps: i32,
    ) -> Result<()> {
        let Some(next_scale) = adjust_scale(self.scale, steps) else {
            return Ok(());
        };

        let previous_layout = self.window_layout;
        if self.config.favorite_ensemble_enabled {
            self.config.favorite_ensemble_scale = Some(next_scale);
        } else {
            self.config.scale = Some(next_scale);
        }
        self.scale = next_scale;
        self.pending_persisted_scale = Some(next_scale);
        self.last_scale_change_at = Some(now);
        self.base_size = size_vec(
            self.open_skin.image_size[0],
            self.open_skin.image_size[1],
            Some(self.scale),
        );
        self.refresh_window_layout(ctx, previous_layout);
        ctx.request_repaint();
        Ok(())
    }

    pub(super) fn pending_scale_persist_remaining(&self, now: Instant) -> Option<Duration> {
        match (self.pending_persisted_scale, self.last_scale_change_at) {
            (Some(_), Some(changed_at)) => {
                let elapsed = now.saturating_duration_since(changed_at);
                Some(SCALE_PERSIST_DEBOUNCE.saturating_sub(elapsed))
            }
            (None, None) => None,
            _ => {
                debug_assert!(
                    matches!(
                        (self.pending_persisted_scale, self.last_scale_change_at),
                        (Some(_), Some(_)) | (None, None)
                    ),
                    "pending scale debounce state should be set and cleared together"
                );
                None
            }
        }
    }

    pub(super) fn persist_pending_scale_if_due(&mut self, now: Instant) -> Result<()> {
        let Some(pending_scale) = self.pending_persisted_scale else {
            return Ok(());
        };
        let pending_remaining = self.pending_scale_persist_remaining(now);
        if let Some(remaining) = pending_remaining {
            if !remaining.is_zero() {
                return Ok(());
            }
        }
        self.persist_pending_scale(pending_scale)
    }

    pub(super) fn persist_pending_scale(&mut self, scale: f32) -> Result<()> {
        if self.config.favorite_ensemble_enabled {
            persist_favorite_ensemble_scale(&self.config_path, &self.config, scale)?;
        } else {
            persist_scale(&self.config_path, &self.config, scale)?;
            if let Err(error) = self
                .favorite_shuffle
                .persist_scale_for_current_config(&self.config, scale)
            {
                eprintln!("{error:#}");
            }
        }
        self.pending_persisted_scale = None;
        self.last_scale_change_at = None;
        self.runtime_state_modified_at = path_modified_at(&self.runtime_state_path);
        Ok(())
    }
}
