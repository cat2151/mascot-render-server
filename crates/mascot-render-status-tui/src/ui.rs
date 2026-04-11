use std::path::Path;
use std::time::Instant;

use mascot_render_protocol::{
    now_unix_ms, ServerCommandKind, ServerCommandStage, ServerCommandStatus, ServerLifecyclePhase,
    ServerStatusSnapshot, ServerWorkStatus,
};
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use crate::state::{format_duration_ms, heartbeat_age_ms_at, StatusTuiState};

const HELP_LINES: &[&str] = &[
    "Keys",
    "? / Esc close help",
    "q / Ctrl-C quit",
    "s POST /show",
    "h POST /hide",
    "p POST /change-character configured_character_name",
    "r pick random cached PSD name and POST /change-character",
    "t POST /timeline shake",
    "m POST /timeline mouth-flap",
    "",
    "Status",
    "Poll interval: 250ms",
    "Disconnected starts mascot-render-server once.",
];
const KEY_HINT_TEXT: &str = "? help | q quit | s show | h hide | p change-character configured name | r random cached PSD | t shake | m mouth-flap | Ctrl-C quit";
const MIN_POST_RESULT_PANEL_HEIGHT: u16 = 6;

pub(crate) fn draw(frame: &mut ratatui::Frame, state: &StatusTuiState) {
    let root_block = Block::default().borders(Borders::ALL).title("Status");
    frame.render_widget(root_block.clone(), frame.area());
    let root_area = root_block.inner(frame.area());
    let post_result_height = post_result_panel_height(root_area.height);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(9),
            Constraint::Length(post_result_height),
            Constraint::Length(3),
        ])
        .split(root_area);

    render_header(frame, layout[0], state);

    let main_columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(layout[1]);
    render_panel(frame, main_columns[0], "Mascot", mascot_lines(state));
    render_panel(frame, main_columns[1], "Runtime", runtime_lines(state));

    let command_columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(layout[2]);
    render_panel(
        frame,
        command_columns[0],
        "Current Work",
        current_work_lines(state),
    );
    render_panel(
        frame,
        command_columns[1],
        "Current Command",
        current_command_lines(state),
    );
    render_panel(
        frame,
        command_columns[2],
        "Last Completed",
        completed_command_lines(state),
    );
    render_panel(
        frame,
        command_columns[3],
        "Last Failed",
        failed_command_lines(state),
    );

    render_panel(
        frame,
        layout[3],
        "Test POST Result",
        vec![state.test_post_status_label()],
    );
    let keys = Paragraph::new(KEY_HINT_TEXT)
        .block(Block::default().borders(Borders::ALL).title("Keys"))
        .wrap(Wrap { trim: false });
    frame.render_widget(keys, layout[4]);

    if state.is_help_visible() {
        render_help_overlay(frame, frame.area());
    }
}

pub(crate) fn post_result_panel_height(root_height: u16) -> u16 {
    let target_height = root_height / 4;
    target_height
        .max(MIN_POST_RESULT_PANEL_HEIGHT)
        .min(root_height)
}

fn render_help_overlay(frame: &mut ratatui::Frame, area: Rect) {
    let overlay_area = help_overlay_area(area);
    let help = Paragraph::new(help_text())
        .alignment(Alignment::Left)
        .block(Block::default().borders(Borders::ALL).title("Help"))
        .wrap(Wrap { trim: false });
    frame.render_widget(Clear, overlay_area);
    frame.render_widget(help, overlay_area);
}

fn help_text() -> String {
    HELP_LINES.join("\n")
}

pub(crate) fn help_overlay_area(area: Rect) -> Rect {
    let desired_width = HELP_LINES
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or_default()
        .saturating_add(4);
    let desired_height = HELP_LINES.len().saturating_add(2);
    let width = bounded_overlay_size(area.width, desired_width);
    let height = bounded_overlay_size(area.height, desired_height);

    Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    }
}

fn bounded_overlay_size(parent_size: u16, desired_size: usize) -> u16 {
    let desired_size = u16::try_from(desired_size).unwrap_or(u16::MAX);
    let max_size = parent_size.saturating_sub(2).max(1);
    desired_size.min(max_size)
}

fn render_header(frame: &mut ratatui::Frame, area: Rect, state: &StatusTuiState) {
    let now = Instant::now();
    let now_unix_ms = now_unix_ms();
    let lifecycle = state
        .last_snapshot
        .as_ref()
        .map(|snapshot| lifecycle_text(snapshot.lifecycle))
        .unwrap_or("-");
    let heartbeat_age = state
        .last_snapshot
        .as_ref()
        .map(|snapshot| format_duration_ms(heartbeat_age_ms_at(snapshot, now_unix_ms)))
        .unwrap_or_else(|| "-".to_string());
    let last_success = state
        .last_success_age_ms(now)
        .map(|ms| format!("{} ago", format_duration_ms(ms)))
        .unwrap_or_else(|| "-".to_string());

    let text = format!(
        "connection: {} | poll: {} | startup: {} | lifecycle: {lifecycle} | heartbeat age: {heartbeat_age} | last success: {last_success}",
        state.connection_label(),
        state.poll_status_label(),
        state.startup_status_summary()
    );
    let style = if state.is_connected() {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::Red)
    };
    let header = Paragraph::new(text)
        .style(style)
        .block(Block::default().borders(Borders::ALL).title("Connection"))
        .wrap(Wrap { trim: false });
    frame.render_widget(header, area);
}

fn render_panel(frame: &mut ratatui::Frame, area: Rect, title: &'static str, lines: Vec<String>) {
    let panel = Paragraph::new(lines.join("\n"))
        .block(Block::default().borders(Borders::ALL).title(title))
        .wrap(Wrap { trim: false });
    frame.render_widget(panel, area);
}

fn mascot_lines(state: &StatusTuiState) -> Vec<String> {
    let Some(snapshot) = state.last_snapshot.as_ref() else {
        return vec!["snapshot: -".to_string(), poll_error_line(state)];
    };

    vec![
        format!(
            "configured_character_name: {}",
            snapshot.configured_character_name.as_deref().unwrap_or("-")
        ),
        format!(
            "configured_png_path: {}",
            path_text(&snapshot.configured_png_path)
        ),
        format!(
            "displayed_png_path: {}",
            path_text(&snapshot.displayed_png_path)
        ),
        format!(
            "configured_zip_path: {}",
            path_text(&snapshot.configured_zip_path)
        ),
        format!(
            "configured_psd_path_in_zip: {}",
            path_text(&snapshot.configured_psd_path_in_zip)
        ),
        format!(
            "favorite_ensemble_enabled: {}",
            snapshot.favorite_ensemble_enabled
        ),
        format!(
            "favorite_ensemble_loaded: {}",
            snapshot.favorite_ensemble_loaded
        ),
        format!("scale: {:.2}", snapshot.scale),
        format!("motion.active: {}", snapshot.motion.active),
        format!("motion.blink_closed: {}", snapshot.motion.blink_closed),
        format!(
            "motion.mouth_flap_open: {}",
            option_bool_text(snapshot.motion.mouth_flap_open)
        ),
        poll_error_line(state),
    ]
}

fn runtime_lines(state: &StatusTuiState) -> Vec<String> {
    let mut lines = vec![
        format!("startup_status: {}", state.startup_status_summary()),
        format!("startup_error: {}", state.startup_error().unwrap_or("-")),
    ];

    let Some(snapshot) = state.last_snapshot.as_ref() else {
        lines.push("snapshot: -".to_string());
        lines.push(poll_error_line(state));
        return lines;
    };

    lines.extend([
        format!(
            "window.anchor_position: {}",
            point_text(snapshot.window.anchor_position)
        ),
        format!(
            "window.window_size: {}",
            size_text(snapshot.window.window_size)
        ),
        format!("config_path: {}", path_text(&snapshot.config_path)),
        format!(
            "runtime_state_path: {}",
            path_text(&snapshot.runtime_state_path)
        ),
        format!(
            "pending_persisted_scale: {}",
            snapshot.pending_persisted_scale
        ),
        format!(
            "server_last_error: {}",
            snapshot.last_error.as_deref().unwrap_or("-")
        ),
        poll_error_line(state),
    ]);
    lines
}

fn current_command_lines(state: &StatusTuiState) -> Vec<String> {
    command_lines(snapshot_ref(state), |snapshot| {
        snapshot.current_command.as_ref()
    })
}

fn current_work_lines(state: &StatusTuiState) -> Vec<String> {
    let Some(snapshot) = snapshot_ref(state) else {
        return vec!["-".to_string()];
    };

    work_status_text(snapshot.current_work.as_ref(), now_unix_ms())
        .lines()
        .map(ToString::to_string)
        .collect()
}

fn completed_command_lines(state: &StatusTuiState) -> Vec<String> {
    command_lines(snapshot_ref(state), |snapshot| {
        snapshot.last_completed_command.as_ref()
    })
}

fn failed_command_lines(state: &StatusTuiState) -> Vec<String> {
    command_lines(snapshot_ref(state), |snapshot| {
        snapshot.last_failed_command.as_ref()
    })
}

fn command_lines(
    snapshot: Option<&ServerStatusSnapshot>,
    command: impl FnOnce(&ServerStatusSnapshot) -> Option<&ServerCommandStatus>,
) -> Vec<String> {
    let Some(snapshot) = snapshot else {
        return vec!["-".to_string()];
    };

    command_status_text(command(snapshot))
        .lines()
        .map(ToString::to_string)
        .collect()
}

fn snapshot_ref(state: &StatusTuiState) -> Option<&ServerStatusSnapshot> {
    state.last_snapshot.as_ref()
}

fn poll_error_line(state: &StatusTuiState) -> String {
    format!(
        "last_poll_error: {}",
        state.last_error.as_deref().unwrap_or("-")
    )
}

pub(crate) fn command_status_text(command: Option<&ServerCommandStatus>) -> String {
    let Some(command) = command else {
        return "-".to_string();
    };

    let mut text = format!(
        "kind: {}\nstage: {}\nsummary: {}\nrequested_at_unix_ms: {}\nupdated_at_unix_ms: {}",
        command_kind_text(command.kind),
        command_stage_text(command.stage),
        command.summary,
        command.requested_at_unix_ms,
        command.updated_at_unix_ms
    );
    if let Some(error) = command.error.as_ref() {
        text.push_str("\nerror: ");
        text.push_str(error);
    }
    text
}

pub(crate) fn work_status_text(work: Option<&ServerWorkStatus>, now_unix_ms: u64) -> String {
    let Some(work) = work else {
        return "-".to_string();
    };

    format!(
        "kind: {}\nstage: {}\nsummary: {}\nstarted_at_unix_ms: {}\nupdated_at_unix_ms: {}\nelapsed: {}",
        work.kind,
        work.stage,
        work.summary,
        work.started_at_unix_ms,
        work.updated_at_unix_ms,
        format_duration_ms(now_unix_ms.saturating_sub(work.started_at_unix_ms))
    )
}

pub(crate) fn lifecycle_text(lifecycle: ServerLifecyclePhase) -> &'static str {
    match lifecycle {
        ServerLifecyclePhase::Starting => "starting",
        ServerLifecyclePhase::Running => "running",
        ServerLifecyclePhase::Stopping => "stopping",
    }
}

fn command_kind_text(kind: ServerCommandKind) -> &'static str {
    match kind {
        ServerCommandKind::Show => "show",
        ServerCommandKind::Hide => "hide",
        ServerCommandKind::ChangeCharacter => "change_character",
        ServerCommandKind::Timeline => "timeline",
    }
}

fn command_stage_text(stage: ServerCommandStage) -> &'static str {
    match stage {
        ServerCommandStage::Queued => "queued",
        ServerCommandStage::Applying => "applying",
        ServerCommandStage::Applied => "applied",
        ServerCommandStage::Failed => "failed",
    }
}

pub(crate) fn option_bool_text(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "true",
        Some(false) => "false",
        None => "-",
    }
}

pub(crate) fn point_text(value: Option<[f32; 2]>) -> String {
    value
        .map(|[x, y]| format!("[{x:.1}, {y:.1}]"))
        .unwrap_or_else(|| "-".to_string())
}

fn size_text(value: [f32; 2]) -> String {
    let [width, height] = value;
    format!("[{width:.1}, {height:.1}]")
}

fn path_text(path: &Path) -> String {
    path.display().to_string()
}
