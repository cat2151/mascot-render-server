mod always_bend;
mod app_support;
mod cli;
mod eye_blink;
mod eye_blink_timing;
mod favorite_ensemble;
mod mascot_app;
mod mascot_scale;
mod mouth_flap;

#[cfg(test)]
#[path = "tests/always_bend.rs"]
mod always_bend_tests;
#[cfg(test)]
#[path = "tests/cli.rs"]
mod cli_tests;
#[cfg(test)]
#[path = "tests/favorite_ensemble.rs"]
mod favorite_ensemble_tests;
#[cfg(test)]
#[path = "tests/mascot_app_reload.rs"]
mod mascot_app_reload_tests;
#[cfg(test)]
#[path = "tests/mascot_scale.rs"]
mod mascot_scale_tests;
#[cfg(test)]
#[path = "tests/runtime.rs"]
mod runtime_tests;
#[cfg(test)]
#[path = "tests/window_history.rs"]
mod window_history_tests;

use std::sync::mpsc;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use cli::{parse_cli, CliAction};
use eframe::egui;
use eframe::NativeOptions;
use favorite_ensemble::load_favorite_ensemble;
use mascot_app::MascotApp;
use mascot_render_core::{
    load_mascot_config, load_mascot_image, run_workspace_update, Core, CoreConfig,
};
use mascot_render_server::window_history::{
    load_window_position, outer_position_for_anchor, window_history_path,
};
use mascot_render_server::{
    init_server_log, log_server_error, log_server_info, start_mascot_control_server_with_notify,
    AlphaBounds, MascotWindowLayout,
};

use app_support::{alpha_mask, content_bounds, size_vec, window_title};

const SKIN_CACHE_CAPACITY: usize = 16;

fn main() -> Result<()> {
    let config_path = match parse_cli(std::env::args_os())? {
        CliAction::Run(config_path) => config_path,
        CliAction::Update => {
            run_workspace_update()?;
            return Ok(());
        }
        CliAction::PrintHelp(help) => {
            println!("{help}");
            return Ok(());
        }
    };
    let log_path = init_server_log()?;
    log_server_info(format!(
        "trigger=startup mascot-render-server を起動します: config_path={} log_path={}",
        config_path.display(),
        log_path.display()
    ));
    let config = load_mascot_config(&config_path)?;
    let core = Core::new(CoreConfig::default());
    let favorite_ensemble = if config.favorite_ensemble_enabled {
        load_favorite_ensemble(&core)?
    } else {
        None
    };
    let image = load_mascot_image(&config.png_path)?;
    let initial_window_layout = if let Some(favorite_ensemble) = &favorite_ensemble {
        let scale = config.favorite_ensemble_scale.unwrap_or(1.0).max(0.01);
        let image_size = [
            favorite_ensemble.canvas_size[0].ceil().max(1.0) as u32,
            favorite_ensemble.canvas_size[1].ceil().max(1.0) as u32,
        ];
        let base_size = egui::Vec2::new(
            (favorite_ensemble.canvas_size[0] * scale).max(1.0),
            (favorite_ensemble.canvas_size[1] * scale).max(1.0),
        );
        MascotWindowLayout::new(
            base_size,
            image_size,
            AlphaBounds::full(image_size),
            config.bounce,
            config.squash_bounce,
            config.always_idle_sink,
        )
    } else {
        let base_size = size_vec(image.width, image.height, config.scale);
        let initial_alpha_mask = alpha_mask(&image.rgba);
        let initial_content_bounds =
            content_bounds([image.width, image.height], initial_alpha_mask.as_ref());
        MascotWindowLayout::new(
            base_size,
            [image.width, image.height],
            initial_content_bounds,
            config.bounce,
            config.squash_bounce,
            config.always_idle_sink,
        )
    };
    let window_size = initial_window_layout.window_size();
    let history_path = window_history_path(&config);
    let saved_window_position = match load_window_position(&history_path) {
        Ok(position) => position,
        Err(error) => {
            log_server_error(format!(
                "failed to load mascot window history {}: {error:#}",
                history_path.display()
            ));
            None
        }
    };
    let mut viewport = egui::ViewportBuilder::default()
        .with_title(window_title(&config, &config_path))
        .with_inner_size(window_size)
        .with_active(false)
        .with_resizable(false)
        .with_decorations(false)
        .with_transparent(true)
        .with_always_on_top()
        .with_title_shown(false);
    if let Some(position) = saved_window_position {
        viewport = viewport.with_position(outer_position_for_anchor(
            position,
            initial_window_layout.anchor_offset(),
            egui::Vec2::ZERO,
        ));
    }
    let native_options = NativeOptions {
        viewport,
        centered: saved_window_position.is_none(),
        persist_window: false,
        ..Default::default()
    };

    let app_name = "mascot render server";
    eframe::run_native(
        app_name,
        native_options,
        Box::new(move |cc| {
            let (control_tx, control_rx) = mpsc::channel();
            let repaint_ctx = cc.egui_ctx.clone();
            let notify = Arc::new(move || repaint_ctx.request_repaint());
            let _control_server =
                start_mascot_control_server_with_notify(control_tx, Some(notify))?;
            Ok(Box::new(MascotApp::new(
                cc,
                config_path.clone(),
                config,
                image,
                favorite_ensemble,
                control_rx,
                saved_window_position,
            )))
        }),
    )
    .map_err(|error| anyhow!(error.to_string()))?;
    Ok(())
}
