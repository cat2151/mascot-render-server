use std::time::{Duration, SystemTime, UNIX_EPOCH};

use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::Paragraph;

const SPINNER_FRAMES: [&str; 4] = ["|", "/", "-", "\\"];
const SPINNER_INTERVAL: Duration = Duration::from_millis(250);

pub(crate) fn draw_activity_indicator(
    frame: &mut ratatui::Frame,
    area: Rect,
    message: &str,
    style: Style,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let indicator = Paragraph::new(format!("[{}] {}", spinner_frame(), message))
        .alignment(Alignment::Left)
        .style(style.add_modifier(Modifier::BOLD));
    frame.render_widget(indicator, area);
}

fn spinner_frame() -> &'static str {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let frame_index = (elapsed.as_millis() / SPINNER_INTERVAL.as_millis()) as usize;
    SPINNER_FRAMES[frame_index % SPINNER_FRAMES.len()]
}
