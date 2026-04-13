use ratatui::layout::{Alignment, Rect};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use crate::activity_indicator::draw_activity_indicator;

use super::style::{
    activity_indicator_style, base_style, comment_style, compact_overlay_style,
    overlay_border_style, processing_overlay_style,
};

pub(super) fn draw_help_bar(frame: &mut ratatui::Frame, help_line: Line<'static>, focused: bool) {
    let help_area = Rect::new(
        frame.area().x.saturating_add(1),
        frame
            .area()
            .y
            .saturating_add(frame.area().height.saturating_sub(1)),
        frame.area().width.saturating_sub(2),
        1,
    );
    let help = Paragraph::new(help_line)
        .style(comment_style(focused))
        .alignment(Alignment::Left);
    frame.render_widget(help, help_area);
}

pub(super) fn draw_help_dialog(
    frame: &mut ratatui::Frame,
    help_lines: Vec<Line<'static>>,
    focused: bool,
) {
    draw_text_overlay(frame, "Help", help_lines, focused);
}

pub(super) fn draw_overlay_dialog(
    frame: &mut ratatui::Frame,
    title: &str,
    lines: Vec<Line<'static>>,
    focused: bool,
) {
    draw_text_overlay(frame, title, lines, focused);
}

pub(super) fn draw_unfocused_preview_overlay(
    frame: &mut ratatui::Frame,
    preview_area: Option<Rect>,
) {
    let Some(preview_area) = preview_area else {
        return;
    };

    let overlay = Paragraph::new("TUI unfocused")
        .alignment(Alignment::Center)
        .style(comment_style(false))
        .wrap(Wrap { trim: false });
    frame.render_widget(Clear, preview_area);
    frame.render_widget(overlay, preview_area);
}

pub(super) fn draw_loading_overlay(
    frame: &mut ratatui::Frame,
    message: &str,
    compact: bool,
    focused: bool,
) {
    if compact {
        draw_compact_loading_overlay(frame, message, focused);
    } else {
        draw_standard_loading_overlay(frame, message, focused);
    }
}

pub(super) fn draw_processing_overlay(
    frame: &mut ratatui::Frame,
    preview_area: Rect,
    message: &str,
    focused: bool,
) {
    let desired_width = message.chars().count().saturating_add(6);
    let overlay_width = desired_width.min(usize::from(preview_area.width.max(1))) as u16;
    let overlay_area = centered_rect(preview_area, overlay_width, 3);
    let overlay = Paragraph::new(message.to_string())
        .alignment(Alignment::Center)
        .style(processing_overlay_style(focused))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(base_style(focused))
                .border_style(overlay_border_style(focused))
                .title("Processing"),
        );

    frame.render_widget(Clear, overlay_area);
    frame.render_widget(overlay, overlay_area);
}

pub(super) fn draw_activity_overlay(
    frame: &mut ratatui::Frame,
    root_inner: Rect,
    message: &str,
    focused: bool,
) {
    let overlay_area = activity_overlay_rect(root_inner, message);
    frame.render_widget(Clear, overlay_area);
    draw_activity_indicator(
        frame,
        overlay_area,
        message,
        activity_indicator_style(focused),
    );
}

fn draw_standard_loading_overlay(frame: &mut ratatui::Frame, message: &str, focused: bool) {
    let overlay_area = centered_rect(frame.area(), 44, 5);
    let overlay = Paragraph::new(message.to_string())
        .alignment(Alignment::Center)
        .style(base_style(focused))
        .wrap(Wrap { trim: false })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(base_style(focused))
                .border_style(overlay_border_style(focused))
                .title("Loading Preview"),
        );

    frame.render_widget(Clear, overlay_area);
    frame.render_widget(overlay, overlay_area);
}

fn draw_compact_loading_overlay(frame: &mut ratatui::Frame, message: &str, focused: bool) {
    let desired_width = message.chars().count().saturating_add(4);
    let max_width = usize::from(frame.area().width.max(1));
    let overlay_width = desired_width.min(max_width) as u16;
    let overlay_area = centered_rect(frame.area(), overlay_width, 1);
    let overlay = Paragraph::new(message.to_string())
        .alignment(Alignment::Center)
        .style(compact_overlay_style(focused));

    frame.render_widget(Clear, overlay_area);
    frame.render_widget(overlay, overlay_area);
}

fn draw_text_overlay(
    frame: &mut ratatui::Frame,
    title: &str,
    lines: Vec<Line<'static>>,
    focused: bool,
) {
    let desired_width = help_overlay_width(&lines);
    let desired_height = lines.len().saturating_add(2) as u16;
    let overlay_area = centered_rect(frame.area(), desired_width, desired_height);
    let overlay = Paragraph::new(lines)
        .style(base_style(focused))
        .wrap(Wrap { trim: false })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(base_style(focused))
                .border_style(overlay_border_style(focused))
                .title(title),
        );

    frame.render_widget(Clear, overlay_area);
    frame.render_widget(overlay, overlay_area);
}

fn help_overlay_width(help_lines: &[Line<'static>]) -> u16 {
    help_lines
        .iter()
        .map(line_width)
        .max()
        .unwrap_or(24)
        .saturating_add(4)
}

fn line_width(line: &Line<'static>) -> u16 {
    line.spans
        .iter()
        .map(|span| span.content.chars().count())
        .sum::<usize>() as u16
}

fn activity_overlay_rect(area: Rect, message: &str) -> Rect {
    let overlay_width = message
        .chars()
        .count()
        .saturating_add(4)
        .min(usize::from(area.width.max(1))) as u16;
    Rect::new(area.x, area.y, overlay_width, area.height.min(1))
}

fn centered_rect(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect::new(x, y, width, height)
}
