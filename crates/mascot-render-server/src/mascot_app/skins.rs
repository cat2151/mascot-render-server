use std::path::Path;
use std::time::Instant;

use anyhow::{Context, Result};
use eframe::egui;
use mascot_render_core::{load_mascot_image, MascotConfig};

use super::{CachedSkin, FavoriteEnsembleScene, MascotApp};
use crate::app_support::cached_skin_from_image;
use crate::eye_blink::render_closed_eye_png;
use crate::favorite_ensemble::load_favorite_ensemble;
use crate::mouth_flap::render_mouth_flap_pngs;

impl MascotApp {
    pub(super) fn load_skin(&mut self, ctx: &egui::Context, png_path: &Path) -> Result<CachedSkin> {
        if let Some(cached_skin) = self.skin_cache.get(png_path) {
            return Ok(cached_skin.clone());
        }

        let image = load_mascot_image(png_path)
            .with_context(|| format!("failed to load mascot skin {}", png_path.display()))?;
        let skin = cached_skin_from_image(ctx, &image);
        self.skin_cache.insert(png_path.to_path_buf(), skin.clone());
        Ok(skin)
    }

    pub(super) fn load_active_skin(&mut self, ctx: &egui::Context) -> Result<CachedSkin> {
        let png_path = self.config.png_path.clone();
        self.load_skin(ctx, &png_path)
    }

    pub(super) fn load_active_ensemble_scene(
        &mut self,
        ctx: &egui::Context,
    ) -> Result<Option<FavoriteEnsembleScene>> {
        Ok(load_favorite_ensemble(&self.core)?.map(|ensemble| {
            FavoriteEnsembleScene::from_loaded(
                ctx,
                ensemble,
                self.config.always_idle_sink_enabled,
                Instant::now(),
            )
        }))
    }

    pub(super) fn refresh_closed_eye_skin(&mut self, ctx: &egui::Context) -> Result<()> {
        self.eye_blink.reset(Instant::now());
        let config = self.config.clone();
        self.closed_skin = self.load_closed_eye_skin_for_config(ctx, &config)?;
        Ok(())
    }

    pub(super) fn refresh_mouth_flap_skins(&mut self, ctx: &egui::Context) -> Result<()> {
        let config = self.config.clone();
        let (mouth_open_skin, mouth_closed_skin) =
            self.load_mouth_flap_skins_for_config(ctx, &config)?;
        self.mouth_open_skin = mouth_open_skin;
        self.mouth_closed_skin = mouth_closed_skin;
        Ok(())
    }

    pub(super) fn queue_auxiliary_skin_refresh(&mut self) {
        self.clear_auxiliary_skins();
        self.pending_auxiliary_skin_refresh = !self.config.favorite_ensemble_enabled;
    }

    pub(super) fn has_pending_auxiliary_skin_refresh(&self) -> bool {
        self.pending_auxiliary_skin_refresh
    }

    pub(super) fn refresh_pending_auxiliary_skins(&mut self, ctx: &egui::Context) -> Result<()> {
        if !self.pending_auxiliary_skin_refresh {
            return Ok(());
        }

        self.pending_auxiliary_skin_refresh = false;
        if self.config.favorite_ensemble_enabled {
            self.clear_auxiliary_skins();
            return Ok(());
        }

        self.refresh_closed_eye_skin(ctx)?;
        self.refresh_mouth_flap_skins(ctx)
    }

    fn clear_auxiliary_skins(&mut self) {
        self.eye_blink.reset(Instant::now());
        self.closed_skin = None;
        self.mouth_open_skin = None;
        self.mouth_closed_skin = None;
    }

    pub(super) fn load_closed_eye_skin_for_config(
        &mut self,
        ctx: &egui::Context,
        config: &MascotConfig,
    ) -> Result<Option<CachedSkin>> {
        if config.favorite_ensemble_enabled {
            return Ok(None);
        }

        let Some(closed_png_path) = render_closed_eye_png(&self.core, config)? else {
            return Ok(None);
        };
        if closed_png_path == config.png_path {
            return Ok(None);
        }

        Ok(Some(self.load_skin(ctx, &closed_png_path)?))
    }

    pub(super) fn load_mouth_flap_skins_for_config(
        &mut self,
        ctx: &egui::Context,
        config: &MascotConfig,
    ) -> Result<(Option<CachedSkin>, Option<CachedSkin>)> {
        if config.favorite_ensemble_enabled {
            return Ok((None, None));
        }

        let Some(mouth_flap_pngs) = render_mouth_flap_pngs(&self.core, config)? else {
            return Ok((None, None));
        };

        Ok((
            Some(self.load_skin(ctx, &mouth_flap_pngs.open_png_path)?),
            Some(self.load_skin(ctx, &mouth_flap_pngs.closed_png_path)?),
        ))
    }
}
