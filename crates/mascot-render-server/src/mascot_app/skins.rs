use std::path::Path;
use std::time::Instant;

use anyhow::{Context, Result};
use eframe::egui;
use mascot_render_control::{log_server_info, log_server_performance_info};
use mascot_render_core::{load_mascot_image_with_report, MascotConfig};

use super::{CachedSkin, FavoriteEnsembleScene, MascotApp};
use crate::app_support::cached_skin_from_image_with_report;
use crate::eye_blink::render_closed_eye_png;
use crate::favorite_ensemble::load_favorite_ensemble;
use crate::mouth_flap::render_mouth_flap_pngs;

impl MascotApp {
    pub(super) fn load_skin(&mut self, ctx: &egui::Context, png_path: &Path) -> Result<CachedSkin> {
        let total_started_at = Instant::now();
        let cache_lookup_started_at = Instant::now();
        if let Some(cached_skin) = self.skin_cache.get(png_path).cloned() {
            let cache_lookup_ms = elapsed_ms_since(cache_lookup_started_at);
            let _work =
                self.start_current_work("load_skin", "cache_hit", skin_work_summary(png_path));
            self.record_performance_stage("load_skin.memory_cache_hit", cache_lookup_ms);
            log_server_info(format!(
                "trigger=skin_cache action=load_skin stage=cache_hit png_path={}",
                png_path.display()
            ));
            log_server_performance_info(format!(
                "event=skin_load stage=memory_cache_hit elapsed_ms={} cache_lookup_ms={} png_path={}",
                elapsed_ms_since(total_started_at),
                cache_lookup_ms,
                png_path.display(),
            ));
            return Ok(cached_skin);
        }
        let cache_lookup_ms = elapsed_ms_since(cache_lookup_started_at);
        self.record_performance_stage("load_skin.memory_cache_miss_lookup", cache_lookup_ms);

        let _work = self.start_current_work(
            "load_skin",
            "cache_miss_decode_texture",
            skin_work_summary(png_path),
        );
        log_server_info(format!(
            "trigger=skin_cache action=load_skin stage=cache_miss_decode_texture png_path={}",
            png_path.display()
        ));
        let (image, image_report) = load_mascot_image_with_report(png_path)
            .with_context(|| format!("failed to load mascot skin {}", png_path.display()))?;
        self.record_performance_stage(
            "load_skin.raw_rgba_meta_read",
            image_report.raw_rgba_meta_read_ms,
        );
        self.record_performance_stage("load_skin.raw_rgba_read", image_report.raw_rgba_read_ms);
        self.record_performance_stage("load_skin.read_file", image_report.read_file_ms);
        self.record_performance_stage("load_skin.decode_png", image_report.decode_png_ms);

        let (skin, build_report) = cached_skin_from_image_with_report(ctx, &image);
        self.record_performance_stage(
            "load_skin.detail_cache_read",
            build_report.detail_cache_read_ms,
        );
        self.record_performance_stage("load_skin.alpha_mask", build_report.alpha_mask_ms);
        self.record_performance_stage("load_skin.content_bounds", build_report.content_bounds_ms);
        self.record_performance_stage("load_skin.texture_alloc", build_report.texture_alloc_ms);
        self.record_performance_stage(
            "load_skin.detail_cache_write",
            build_report.detail_cache_write_ms,
        );

        let cache_insert_started_at = Instant::now();
        let evicted_paths = self.skin_cache.insert(png_path.to_path_buf(), skin.clone());
        let cache_insert_ms = elapsed_ms_since(cache_insert_started_at);
        self.record_performance_stage("load_skin.cache_insert", cache_insert_ms);
        let evicted_count = evicted_paths.len();
        for evicted_path in evicted_paths {
            log_server_info(format!(
                "trigger=skin_cache action=evict evicted_png_path={}",
                evicted_path.display()
            ));
        }
        log_server_performance_info(format!(
            "event=skin_load stage=cache_miss_loaded elapsed_ms={} cache_lookup_ms={} raw_rgba_cache_hit={} raw_rgba_cache_status={} raw_rgba_meta_read_ms={} raw_rgba_read_ms={} read_file_ms={} decode_png_ms={} detail_cache_hit={} detail_cache_read_ms={} alpha_mask_ms={} content_bounds_ms={} detail_cache_write_ms={} texture_alloc_ms={} cache_insert_ms={} file_bytes={} rgba_bytes={} image_size={}x{} evicted_count={} png_path={}",
            elapsed_ms_since(total_started_at),
            cache_lookup_ms,
            image_report.raw_rgba_cache_hit,
            image_report.raw_rgba_cache_status,
            image_report.raw_rgba_meta_read_ms,
            image_report.raw_rgba_read_ms,
            image_report.read_file_ms,
            image_report.decode_png_ms,
            build_report.detail_cache_hit,
            build_report.detail_cache_read_ms,
            build_report.alpha_mask_ms,
            build_report.content_bounds_ms,
            build_report.detail_cache_write_ms,
            build_report.texture_alloc_ms,
            cache_insert_ms,
            image_report.file_bytes,
            image_report.rgba_bytes,
            image_report.width,
            image_report.height,
            evicted_count,
            png_path.display(),
        ));
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

    pub(super) fn queue_auxiliary_skin_refresh(&mut self) {
        self.clear_auxiliary_skins();
        self.pending_auxiliary_skin_refresh = !self.config.favorite_ensemble_enabled;
    }

    pub(super) fn has_pending_auxiliary_skin_refresh(&self) -> bool {
        self.pending_auxiliary_skin_refresh
    }

    pub(super) fn refresh_pending_auxiliary_skins(&mut self, _ctx: &egui::Context) -> Result<()> {
        if !self.pending_auxiliary_skin_refresh {
            return Ok(());
        }

        let mut work = self.start_current_work(
            "refresh_pending_auxiliary_skins",
            "refresh_closed_eye_skin",
            format!("png_path={}", self.config.png_path.display()),
        );
        self.pending_auxiliary_skin_refresh = false;
        if self.config.favorite_ensemble_enabled {
            self.clear_auxiliary_skins();
            return Ok(());
        }

        work.update_stage(
            "defer_auxiliary_skins",
            format!("png_path={}", self.config.png_path.display()),
        );
        Ok(())
    }

    fn clear_auxiliary_skins(&mut self) {
        self.eye_blink.reset(Instant::now());
        self.closed_skin = None;
        self.closed_skin_unavailable = false;
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

        let mut generation_stages = Vec::new();
        let result = render_mouth_flap_pngs(&self.core, config, |stage, elapsed_ms| {
            generation_stages.push((stage, elapsed_ms));
        });
        for (stage, elapsed_ms) in generation_stages {
            self.record_performance_stage(stage, elapsed_ms);
        }
        let Some(mouth_flap_pngs) = result? else {
            return Ok((None, None));
        };

        let open_skin_started_at = Instant::now();
        let open_skin = self.load_skin(ctx, &mouth_flap_pngs.open_png_path)?;
        self.record_performance_stage(
            "mouth_flap.load_open_skin",
            elapsed_ms_since(open_skin_started_at),
        );
        let closed_skin_started_at = Instant::now();
        let closed_skin = self.load_skin(ctx, &mouth_flap_pngs.closed_png_path)?;
        self.record_performance_stage(
            "mouth_flap.load_closed_skin",
            elapsed_ms_since(closed_skin_started_at),
        );

        Ok((Some(open_skin), Some(closed_skin)))
    }
}

fn skin_work_summary(png_path: &Path) -> String {
    format!("png_path={}", png_path.display())
}

fn elapsed_ms_since(started_at: Instant) -> u64 {
    u64::try_from(started_at.elapsed().as_millis()).unwrap_or(u64::MAX)
}
