use super::MascotApp;
use eframe::egui;
use mascot_render_core::{MotionTransform, SquashBounceAnimationConfig};
use mascot_render_protocol::PlaybackSnapshot;
use std::time::{Duration, Instant};

pub(crate) fn transform(
    state: Option<&PlaybackSnapshot>,
    now: Instant,
    config: SquashBounceAnimationConfig,
) -> Option<MotionTransform> {
    let state = state?;
    let beat = state.beat(now)?;
    // Each quarter note restarts the original click reaction. Keep its duration,
    // easing; deliberately cut it off when the next beat arrives.
    let elapsed =
        Duration::try_from_secs_f64(beat.rem_euclid(1.0) / state.request.beats_per_second).ok()?;
    Some(config.sample(elapsed))
}
impl MascotApp {
    pub(super) fn playback_snapshot(&mut self, ctx: &egui::Context) -> Option<PlaybackSnapshot> {
        let state = self.status_store.playback().ok().flatten()?;
        if !state.is_active(Instant::now()) {
            return None;
        }
        let name = super::character::configured_character_name_for_status(
            &self.config.zip_path,
            &self.config.psd_path_in_zip,
        );
        if name.as_deref().is_some_and(|n| n.contains("ずんだもん")) {
            if state.request.singing
                && self.mouth_open_skin.is_none()
                && self.mouth_closed_skin.is_none()
                && self.playback_skin_attempt.as_ref() != Some(&self.config.png_path)
            {
                self.playback_skin_attempt = Some(self.config.png_path.clone());
                let config = self.config.clone();
                match self.load_mouth_flap_skins_for_config(ctx, &config) {
                    Ok((open, closed)) => {
                        self.mouth_open_skin = open;
                        self.mouth_closed_skin = closed;
                    }
                    Err(e) => {
                        self.record_and_log_status_error(format!("playback mouth skins: {e:#}"))
                    }
                }
            }
        }
        // Loading can take seconds; use the newest lease and position afterward.
        let state = self.status_store.playback().ok().flatten()?;
        if state.is_active(Instant::now()) {
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
            Some(state)
        } else {
            None
        }
    }
    pub(super) fn playback_is_zundamon(&self) -> bool {
        super::character::configured_character_name_for_status(
            &self.config.zip_path,
            &self.config.psd_path_in_zip,
        )
        .is_some_and(|n| n.contains("ずんだもん"))
    }
}

impl super::ensemble::EnsembleMemberScene {
    pub(super) fn playback_skin(
        &mut self,
        state: Option<&PlaybackSnapshot>,
        blink: bool,
        now: Instant,
    ) -> &super::CachedSkin {
        if self
            .character_name
            .as_deref()
            .is_some_and(|n| n.contains("ずんだもん"))
        {
            if let Some(state) = state.filter(|p| p.is_active(now)) {
                return match state.mouth_open(now) {
                    Some(true) => self.mouth_open_skin.as_ref().unwrap_or(&self.open_skin),
                    Some(false) => self.mouth_closed_skin.as_ref().unwrap_or(&self.open_skin),
                    None if blink => self.closed_skin.as_ref().unwrap_or(&self.open_skin),
                    None => &self.open_skin,
                };
            }
        }
        self.active_skin(blink, now)
    }
}
