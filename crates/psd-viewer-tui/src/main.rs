mod activity_heartbeat;
mod activity_indicator;
mod app;
mod cli;
mod display_diff_state;
mod favorites;
mod server_motion_sync;
mod server_preview_sync;
mod terminal;
mod tui_config;
mod tui_history;
mod ui;
mod workspace_state;

#[cfg(test)]
mascot_render_test_support::install_test_data_root!();

#[cfg(test)]
mod tests;

use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::Duration;

use anyhow::Result;
use cli::{parse_cli, CliAction};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use mascot_render_client::hide_mascot_render_server;
use mascot_render_core::{check_workspace_update, mascot_config_path, run_workspace_update};
use ratatui::Terminal;
use tui_sixel_preview::{build_picker, PreviewState};

use activity_heartbeat::ActivityHeartbeat;
use app::{spawn_startup_loader, App, StartupEvent};
use server_motion_sync::{shake_requested_status_message, ServerMotionSync};
use server_preview_sync::ServerPreviewSync;
use terminal::{Backend, TerminalGuard};

const BUILD_COMMIT_HASH: &str = env!("BUILD_COMMIT_HASH");
const STARTUP_LOADER_EVENTS_PER_FRAME: usize = 4;

fn main() -> Result<()> {
    match parse_cli(std::env::args_os())? {
        CliAction::Run => {}
        CliAction::Update => {
            run_workspace_update()?;
            return Ok(());
        }
        CliAction::Check => {
            println!("{}", check_workspace_update(BUILD_COMMIT_HASH)?);
            return Ok(());
        }
        CliAction::PrintHelp(help) => {
            println!("{help}");
            return Ok(());
        }
    }

    let screen_height_px = detect_screen_height_px();
    let mut terminal = TerminalGuard::new()?;
    let app = App::loading(screen_height_px);
    let startup_rx = Some(spawn_startup_loader(screen_height_px));
    let mut picker = None;
    let mut preview = PreviewState::new();
    let _hide_mascot_on_drop = HideMascotRenderServerOnDrop;
    run_app(
        terminal.terminal_mut(),
        &mut picker,
        app,
        startup_rx,
        &mut preview,
    )
}

fn is_layer_toggle_key(key: &crossterm::event::KeyEvent) -> bool {
    key.modifiers == KeyModifiers::NONE && matches!(key.code, KeyCode::Char(' ') | KeyCode::Enter)
}

fn is_favorites_toggle_key(key: &crossterm::event::KeyEvent, favorites_visible: bool) -> bool {
    if key.modifiers != KeyModifiers::NONE {
        return false;
    }

    matches!(key.code, KeyCode::Char('v')) || favorites_visible && matches!(key.code, KeyCode::Esc)
}

fn is_overlay_close_key(key: &crossterm::event::KeyEvent, overlay_visible: bool) -> bool {
    overlay_visible && key.modifiers == KeyModifiers::NONE && matches!(key.code, KeyCode::Esc)
}

fn is_overlay_confirm_key(key: &crossterm::event::KeyEvent, overlay_visible: bool) -> bool {
    overlay_visible && key.modifiers == KeyModifiers::NONE && matches!(key.code, KeyCode::Enter)
}

fn is_favorite_save_key(
    key: &crossterm::event::KeyEvent,
    focus: app::FocusPane,
    favorites_visible: bool,
) -> bool {
    key.modifiers == KeyModifiers::NONE
        && matches!(key.code, KeyCode::Char('f'))
        && !favorites_visible
        && matches!(focus, app::FocusPane::Library | app::FocusPane::Layer)
}

fn is_favorite_ensemble_toggle_key(key: &crossterm::event::KeyEvent) -> bool {
    key.modifiers == KeyModifiers::NONE && matches!(key.code, KeyCode::Char('e'))
}

fn run_app(
    terminal: &mut Terminal<Backend>,
    picker: &mut Option<ratatui_image::picker::Picker>,
    mut app: App,
    mut startup_rx: Option<Receiver<StartupEvent>>,
    preview: &mut PreviewState,
) -> Result<()> {
    let mut activity_heartbeat =
        ActivityHeartbeat::start(&mascot_config_path(), std::time::Instant::now())?;
    let mut server_motion_sync = ServerMotionSync::new();
    let mut server_preview_sync = ServerPreviewSync::new();
    while !app.should_quit {
        activity_heartbeat.refresh_if_due(std::time::Instant::now())?;
        process_startup_events(&mut app, &mut startup_rx, preview, &mut server_preview_sync);
        if let Some(error) = server_motion_sync.drain_completions() {
            app.set_status_message(format!("mascot-render-server motion failed: {error:#}"));
            eprintln!("{error:#}");
        }
        if let Some(error) = server_preview_sync.drain_completions() {
            app.fallback_to_sixel_preview(format!("mascot-render-server sync failed: {error:#}"));
            eprintln!("{error:#}");
        }

        let activity_message =
            current_activity_message(&app, &server_motion_sync, &server_preview_sync);
        terminal.draw(|frame| ui::draw(frame, &mut app, preview, activity_message.as_deref()))?;
        sync_runtime_targets(&mut app, preview, &mut server_preview_sync);

        if app.process_pending_actions()? {
            sync_runtime_targets(&mut app, preview, &mut server_preview_sync);
            continue;
        }

        if app.process_mouth_flap_animation() || app.process_eye_blink_animation() {
            sync_runtime_targets(&mut app, preview, &mut server_preview_sync);
            continue;
        }

        if !app.uses_server_preview() && preview.is_loading() {
            let picker = picker.get_or_insert_with(build_picker);
            preview.sync_pending(picker)?;
            continue;
        }

        if !app.uses_server_preview() {
            if let Some(image_state) = preview.image_state_mut() {
                if let Some(encoding_result) = image_state.last_encoding_result() {
                    encoding_result?;
                }
            }
        }

        if !event::poll(app.event_poll_timeout(Duration::from_millis(250)))? {
            continue;
        }

        let event = event::read()?;
        match event {
            Event::FocusGained => {
                app.set_terminal_focus(true);
                continue;
            }
            Event::FocusLost => {
                app.set_terminal_focus(false);
                continue;
            }
            Event::Key(key) => {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                let mut queued_toggle = false;
                let mut force_server_sync = false;
                if app.is_log_overlay_visible() {
                    match key.code {
                        _ if is_overlay_close_key(&key, app.is_log_overlay_visible())
                            || is_overlay_confirm_key(&key, app.is_log_overlay_visible()) =>
                        {
                            app.clear_log_overlay();
                        }
                        KeyCode::Char('q') if key.modifiers == KeyModifiers::NONE => {
                            app.should_quit = true;
                        }
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            app.should_quit = true;
                        }
                        _ => continue,
                    }
                    sync_runtime_targets(&mut app, preview, &mut server_preview_sync);
                    continue;
                }

                let help_overlay_visible = app.is_help_overlay_visible();
                if help_overlay_visible {
                    match key.code {
                        _ if is_overlay_close_key(&key, help_overlay_visible) => {
                            app.toggle_help_overlay();
                        }
                        KeyCode::Char('?') if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                            app.toggle_help_overlay();
                        }
                        KeyCode::Char('q') if key.modifiers == KeyModifiers::NONE => {
                            app.should_quit = true;
                        }
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            app.should_quit = true;
                        }
                        _ => continue,
                    }
                    sync_runtime_targets(&mut app, preview, &mut server_preview_sync);
                    continue;
                }

                match key.code {
                    KeyCode::Char('?') if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                        app.toggle_help_overlay();
                    }
                    _ if is_favorites_toggle_key(&key, app.favorites_visible()) => {
                        app.toggle_favorites_view();
                    }
                    _ if is_favorite_save_key(&key, app.focus, app.favorites_visible()) => {
                        app.add_current_favorite()?;
                    }
                    _ if is_favorite_ensemble_toggle_key(&key) => {
                        force_server_sync = app.toggle_favorite_ensemble_enabled()?;
                    }
                    KeyCode::Up | KeyCode::Char('k') if key.modifiers == KeyModifiers::NONE => {
                        app.select_previous()?;
                    }
                    KeyCode::Down | KeyCode::Char('j') if key.modifiers == KeyModifiers::NONE => {
                        app.select_next()?;
                    }
                    KeyCode::PageUp => {
                        app.page_up(page_step_for_terminal(terminal.size()?.height))?;
                    }
                    KeyCode::PageDown => {
                        app.page_down(page_step_for_terminal(terminal.size()?.height))?;
                    }
                    KeyCode::Left | KeyCode::Char('h') if key.modifiers == KeyModifiers::NONE => {
                        app.move_focus_left();
                    }
                    KeyCode::Right | KeyCode::Char('l') if key.modifiers == KeyModifiers::NONE => {
                        app.move_focus_right();
                    }
                    KeyCode::Enter
                        if key.modifiers == KeyModifiers::NONE && app.favorites_visible() =>
                    {
                        app.activate_selected_favorite()?;
                    }
                    _ if is_layer_toggle_key(&key) => {
                        let predicted_preview_path =
                            app.predicted_preview_png_path_for_selected_toggle();
                        if let Some(predicted_preview_path) = predicted_preview_path {
                            let can_show_immediately = if app.uses_server_preview() {
                                predicted_preview_path.exists()
                            } else {
                                preview.has_sixel_cache_for_path(Some(
                                    predicted_preview_path.as_path(),
                                ))
                            };
                            if can_show_immediately {
                                app.toggle_selected_layer()?;
                            } else {
                                queued_toggle = app.queue_selected_layer_toggle();
                            }
                        }
                    }
                    KeyCode::Char('=') if key.modifiers == KeyModifiers::NONE => {
                        force_server_sync = app.increase_mascot_scale()?;
                    }
                    KeyCode::Char('+') => {
                        force_server_sync = app.increase_mascot_scale()?;
                    }
                    KeyCode::Char('-') if key.modifiers == KeyModifiers::NONE => {
                        force_server_sync = app.decrease_mascot_scale()?;
                    }
                    KeyCode::Char('_') => {
                        force_server_sync = app.decrease_mascot_scale()?;
                    }
                    KeyCode::Char('t') if key.modifiers == KeyModifiers::NONE => {
                        app.start_mouth_flap_preview();
                    }
                    KeyCode::Char('m') if key.modifiers == KeyModifiers::NONE => {
                        app.start_eye_blink_preview();
                    }
                    KeyCode::Char('s') if key.modifiers == KeyModifiers::NONE => {
                        if app.uses_server_preview() {
                            server_motion_sync.request_shake();
                            app.set_status_message(shake_requested_status_message());
                        } else {
                            app.set_status_message(
                                "Mascot shake unavailable: preview backend is sixel.",
                            );
                        }
                    }
                    KeyCode::Char('q') if key.modifiers == KeyModifiers::NONE => {
                        app.should_quit = true;
                    }
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        app.should_quit = true;
                    }
                    _ => {}
                }

                if queued_toggle {
                    continue;
                }

                if force_server_sync {
                    server_preview_sync.cancel();
                }
                sync_runtime_targets(&mut app, preview, &mut server_preview_sync);
            }
            _ => continue,
        }
    }

    app.persist_workspace_state()?;
    Ok(())
}

fn sync_runtime_targets(
    app: &mut App,
    preview: &mut PreviewState,
    server_preview_sync: &mut ServerPreviewSync,
) {
    if app.uses_server_preview() {
        server_preview_sync.request(app.selected_preview_png_path());
    }

    if !app.uses_server_preview() && !app.is_preview_animation_active() {
        preview.request_sync(app.selected_preview_png_path());
    }
}

fn request_selected_preview_sync(
    app: &App,
    preview: &mut PreviewState,
    server_preview_sync: &mut ServerPreviewSync,
) {
    if app.uses_server_preview() {
        server_preview_sync.request(app.selected_preview_png_path());
    } else {
        preview.request_sync(app.selected_preview_png_path());
    }
}

fn process_startup_events(
    app: &mut App,
    startup_rx: &mut Option<Receiver<StartupEvent>>,
    preview: &mut PreviewState,
    server_preview_sync: &mut ServerPreviewSync,
) {
    let Some(rx) = startup_rx.as_ref() else {
        return;
    };

    let mut close_receiver = false;
    let mut loader_events_processed = 0usize;
    loop {
        match rx.try_recv() {
            Ok(StartupEvent::Progress(message)) => app.apply_startup_progress(message),
            Ok(StartupEvent::Loader(event)) => {
                match app.apply_startup_loader_event(event) {
                    Ok(needs_sync) if needs_sync => {
                        request_selected_preview_sync(app, preview, server_preview_sync);
                    }
                    Ok(_) => {}
                    Err(error) => app.finish_startup_error(error),
                }
                loader_events_processed += 1;
                if loader_events_processed >= STARTUP_LOADER_EVENTS_PER_FRAME {
                    break;
                }
            }
            Ok(StartupEvent::Snapshot(snapshot_app)) => {
                let terminal_focused = app.is_terminal_focused();
                let help_overlay_visible = app.is_help_overlay_visible();
                *app = snapshot_app;
                app.set_terminal_focus(terminal_focused);
                app.set_help_overlay_visible(help_overlay_visible);
                request_selected_preview_sync(app, preview, server_preview_sync);
            }
            Ok(StartupEvent::Ready(result)) => {
                close_receiver = true;
                match result {
                    Ok(mut loaded_app) => {
                        if let Err(error) = loaded_app.adopt_runtime_state_from(app) {
                            app.finish_startup_error(error);
                            continue;
                        }
                        *app = loaded_app;
                        request_selected_preview_sync(app, preview, server_preview_sync);
                    }
                    Err(error) => app.finish_startup_error(error),
                }
            }
            Err(TryRecvError::Empty) => break,
            Err(TryRecvError::Disconnected) => {
                close_receiver = true;
                break;
            }
        }
    }

    if close_receiver {
        *startup_rx = None;
    }
}

fn current_activity_message(
    app: &App,
    server_motion_sync: &ServerMotionSync,
    server_preview_sync: &ServerPreviewSync,
) -> Option<String> {
    if let Some(message) = server_motion_sync.activity_message() {
        return Some(message.to_string());
    }

    if let Some(message) = server_preview_sync.activity_message() {
        return Some(message.to_string());
    }

    if app.is_startup_loading() {
        return Some(
            app.startup_notice()
                .unwrap_or("Loading ZIP/PSD cache index in background...")
                .to_string(),
        );
    }

    None
}

fn page_step_for_terminal(height: u16) -> usize {
    usize::from(height.saturating_sub(14).max(1))
}

fn detect_screen_height_px() -> Option<u16> {
    crossterm::terminal::window_size()
        .ok()
        .map(|size| size.height)
        .filter(|height| *height > 0)
}

struct HideMascotRenderServerOnDrop;

impl Drop for HideMascotRenderServerOnDrop {
    fn drop(&mut self) {
        let _ = hide_mascot_render_server();
    }
}
