mod actions;
mod poller;
mod startup;
mod state;
mod terminal;
mod ui;

#[cfg(test)]
mod tests;

use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use mascot_render_core::mascot_config_path;
use ratatui::Terminal;

use actions::{TestPostAction, TestPostSync};
use poller::StatusPollSync;
use startup::ServerStartupSync;
use state::StatusTuiState;
use terminal::{Backend, TerminalGuard};

const POLL_INTERVAL: Duration = Duration::from_millis(250);

fn main() -> Result<()> {
    let mut terminal = TerminalGuard::new()?;
    let mut state = StatusTuiState::new();
    let config_path = mascot_config_path();
    let mut status_polls = StatusPollSync::new();
    let mut startup = ServerStartupSync::new(config_path.clone());
    let mut test_posts = TestPostSync::new(config_path);

    run_app(
        terminal.terminal_mut(),
        &mut state,
        &mut status_polls,
        &mut startup,
        &mut test_posts,
    )
}

fn run_app(
    terminal: &mut Terminal<Backend>,
    state: &mut StatusTuiState,
    status_polls: &mut StatusPollSync,
    startup: &mut ServerStartupSync,
    test_posts: &mut TestPostSync,
) -> Result<()> {
    let mut next_poll_at = Instant::now();

    while !state.should_quit() {
        drain_startup(state, startup);
        drain_status_poll(state, status_polls, startup);
        drain_test_posts(state, test_posts);

        let now = Instant::now();
        if now >= next_poll_at {
            if status_polls.start_if_idle() {
                state.record_poll_started();
            }
            next_poll_at = now + POLL_INTERVAL;
        }

        terminal.draw(|frame| ui::draw(frame, state))?;

        let wait_for_event = next_poll_at
            .saturating_duration_since(Instant::now())
            .min(POLL_INTERVAL);
        if !event::poll(wait_for_event)? {
            continue;
        }

        if let Event::Key(key) = event::read()? {
            handle_key(state, test_posts, key);
        }
    }

    Ok(())
}

fn drain_status_poll(
    state: &mut StatusTuiState,
    status_polls: &mut StatusPollSync,
    startup: &mut ServerStartupSync,
) {
    if let Some(result) = status_polls.drain_completion() {
        match result {
            Ok(snapshot) => state.record_poll_success(snapshot, Instant::now()),
            Err(error) => {
                state.record_poll_error(error);
                if startup.start_if_idle() {
                    state.record_startup_starting();
                }
            }
        }
    }
}

fn drain_startup(state: &mut StatusTuiState, startup: &mut ServerStartupSync) {
    if let Some(result) = startup.drain_completion() {
        match result {
            Ok(()) => state.record_startup_started(),
            Err(error) if !state.is_connected() => state.record_startup_failed(error),
            Err(_) => state.record_startup_started(),
        }
    }
}

fn drain_test_posts(state: &mut StatusTuiState, test_posts: &mut TestPostSync) {
    if let Some(result) = test_posts.drain_completion() {
        match result {
            Ok(label) => state.record_test_post_success(label),
            Err((label, error)) => state.record_test_post_failed(label, error),
        }
    }
}

fn handle_key(state: &mut StatusTuiState, test_posts: &mut TestPostSync, key: event::KeyEvent) {
    if key.kind != KeyEventKind::Press {
        return;
    }

    match key.code {
        KeyCode::Char('?') if key.modifiers == KeyModifiers::NONE => state.toggle_help(),
        KeyCode::Esc if state.is_help_visible() => state.close_help(),
        KeyCode::Char('q') if state.is_help_visible() && key.modifiers == KeyModifiers::NONE => {
            state.request_quit();
        }
        KeyCode::Char('c')
            if state.is_help_visible() && key.modifiers.contains(KeyModifiers::CONTROL) =>
        {
            state.request_quit();
        }
        _ if state.is_help_visible() => {}
        KeyCode::Char('q') if key.modifiers == KeyModifiers::NONE => state.request_quit(),
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            state.request_quit();
        }
        KeyCode::Char('s') if key.modifiers == KeyModifiers::NONE => {
            start_test_post(state, test_posts, TestPostAction::Show);
        }
        KeyCode::Char('h') if key.modifiers == KeyModifiers::NONE => {
            start_test_post(state, test_posts, TestPostAction::Hide);
        }
        KeyCode::Char('p') if key.modifiers == KeyModifiers::NONE => {
            let Some(character_name) = state.configured_character_name() else {
                state.record_test_post_failed(
                    TestPostAction::change_character_label(),
                    "configured_character_name unavailable: wait for a status snapshot with configured source paths".to_string(),
                );
                return;
            };
            start_test_post(
                state,
                test_posts,
                TestPostAction::ChangeCharacter(character_name),
            );
        }
        KeyCode::Char('t') if key.modifiers == KeyModifiers::NONE => {
            start_test_post(state, test_posts, TestPostAction::ShakeTimeline);
        }
        KeyCode::Char('m') if key.modifiers == KeyModifiers::NONE => {
            start_test_post(state, test_posts, TestPostAction::MouthFlapTimeline);
        }
        _ => {}
    }
}

fn start_test_post(
    state: &mut StatusTuiState,
    test_posts: &mut TestPostSync,
    action: TestPostAction,
) {
    let label = action.label();
    match test_posts.start_if_idle(action) {
        Ok(()) => state.record_test_post_started(label),
        Err(error) => state.record_test_post_failed(label, error),
    }
}
