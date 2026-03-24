mod overlay;

use mascot_render_core::display_path;
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};
use ratatui_image::{Image, StatefulImage};
use tui_sixel_preview::PreviewState;

use crate::app::library::LibraryRow;
use crate::app::{App, FocusPane};

const MONOKAI_BG: Color = Color::Rgb(39, 40, 34);
const MONOKAI_FG: Color = Color::Rgb(248, 248, 242);
const MONOKAI_COMMENT: Color = Color::Rgb(117, 113, 94);
const MONOKAI_CYAN: Color = Color::Rgb(102, 217, 239);
const MONOKAI_GREEN: Color = Color::Rgb(166, 226, 46);
const MONOKAI_ORANGE: Color = Color::Rgb(253, 151, 31);
const MONOKAI_PINK: Color = Color::Rgb(249, 38, 114);
const MONOKAI_YELLOW: Color = Color::Rgb(230, 219, 116);

const DIM_BG: Color = Color::Rgb(36, 36, 36);
const DIM_FG: Color = Color::Rgb(184, 184, 184);
const DIM_COMMENT: Color = Color::Rgb(128, 128, 128);
const DIM_ACCENT: Color = Color::Rgb(210, 210, 210);
const DIM_SELECTED_BG: Color = Color::Rgb(160, 160, 160);
const DIM_SELECTED_FG: Color = Color::Rgb(30, 30, 30);

pub(crate) fn draw(
    frame: &mut ratatui::Frame,
    app: &mut App,
    preview: &mut PreviewState,
    activity_message: Option<&str>,
) {
    let terminal_focused = app.is_terminal_focused();
    let info_lines = app.info_lines();
    let log_lines = app.log_lines();
    let info_height = info_panel_height(info_lines.len());
    let log_height = log_panel_height(log_lines.len());

    let root_block = Block::default()
        .borders(Borders::ALL)
        .style(base_style(terminal_focused))
        .border_style(border_style(terminal_focused, false));
    frame.render_widget(root_block.clone(), frame.area());

    let root_inner = root_block.inner(frame.area());

    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            left_column_constraint(app.uses_server_preview()),
            right_column_constraint(app.uses_server_preview()),
        ])
        .split(root_inner);
    let left_column = layout[0];
    let layer_area = layout[1];

    let left_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(8),
            Constraint::Length(info_height),
            Constraint::Length(log_height),
        ])
        .split(left_column);

    let use_server_preview = app.uses_server_preview();
    let main_content = if use_server_preview {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(100)])
            .split(left_layout[0])
    } else {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(49), Constraint::Percentage(51)])
            .split(left_layout[0])
    };
    let library_area = main_content[0];

    let library_items = if app.favorites_visible() {
        let favorite_rows = app.favorite_rows();
        if favorite_rows.is_empty() {
            vec![
                ListItem::new("No favorites saved yet. Press f on the layer pane.")
                    .style(comment_style(terminal_focused)),
            ]
        } else {
            favorite_rows
                .iter()
                .map(|row| {
                    let style = if row.available {
                        base_style(terminal_focused)
                    } else {
                        comment_style(terminal_focused)
                    };
                    ListItem::new(row.label.clone()).style(style)
                })
                .collect()
        }
    } else {
        let library_rows = app.library_rows();
        if library_rows.is_empty() {
            let message = app.startup_notice().unwrap_or("No PSD files found.");
            vec![ListItem::new(message).style(comment_style(terminal_focused))]
        } else {
            library_rows
                .iter()
                .map(|row| match row {
                    LibraryRow::ZipHeader { zip_index } => {
                        let zip_entry = &app.zip_entries[*zip_index];
                        let label = zip_entry
                            .zip_path
                            .file_name()
                            .map(|name| name.to_string_lossy().into_owned())
                            .unwrap_or_else(|| display_path(&zip_entry.zip_path));
                        ListItem::new(format!("[ZIP] {label}"))
                            .style(accent_style(terminal_focused))
                    }
                    LibraryRow::PsdItem {
                        zip_index,
                        psd_index,
                    } => {
                        let psd_entry = &app.zip_entries[*zip_index].psds[*psd_index];
                        ListItem::new(format!("  {}", psd_entry.file_name))
                            .style(base_style(terminal_focused))
                    }
                })
                .collect()
        }
    };
    let library_list = List::new(library_items)
        .block(pane_block(
            if app.favorites_visible() {
                "Favorites"
            } else {
                "ZIP / PSD"
            },
            terminal_focused,
            app.focus == FocusPane::Library,
        ))
        .highlight_style(selected_style(terminal_focused))
        .highlight_symbol("> ");
    let mut library_state = ListState::default();
    library_state.select(if app.favorites_visible() {
        app.selected_favorite_selection()
    } else {
        app.selected_library_selection()
    });
    frame.render_stateful_widget(library_list, library_area, &mut library_state);

    let preview_area = if use_server_preview {
        None
    } else {
        let preview_block = Block::default()
            .borders(Borders::ALL)
            .title("Preview")
            .style(base_style(terminal_focused))
            .border_style(border_style(terminal_focused, false));
        let preview_inner = preview_block.inner(main_content[1]);
        frame.render_widget(preview_block, main_content[1]);

        let compact_loading_overlay =
            preview.is_loading() && preview.uses_compact_loading_overlay();
        let startup_preview_placeholder =
            app.startup_notice().is_some() && app.selected_preview_png_path().is_none();

        if preview.is_loading() {
            if !compact_loading_overlay {
                let preview_text = Paragraph::new(preview.status().to_string())
                    .alignment(Alignment::Center)
                    .style(base_style(terminal_focused))
                    .wrap(Wrap { trim: false });
                frame.render_widget(preview_text, preview_inner);
            }
        } else if startup_preview_placeholder {
            let preview_text =
                Paragraph::new("Loading preview...\nZIP/PSD cache is still loading.")
                    .alignment(Alignment::Center)
                    .style(comment_style(terminal_focused))
                    .wrap(Wrap { trim: false });
            frame.render_widget(preview_text, preview_inner);
        } else {
            preview.prepare_sixel_render(preview_inner);
            if let Some(protocol) = preview.active_sixel_protocol() {
                let image = Image::new(protocol);
                frame.render_widget(image, preview_inner);
            } else if let Some(image_state) = preview.image_state_mut() {
                let image = StatefulImage::default();
                frame.render_stateful_widget(image, preview_inner, image_state);
            } else {
                let preview_text = Paragraph::new(preview.status().to_string())
                    .style(base_style(terminal_focused))
                    .wrap(Wrap { trim: false });
                frame.render_widget(preview_text, preview_inner);
            }
        }

        Some(preview_inner)
    };

    let layer_items = if app.selected_layer_rows().is_empty() {
        let message = if app.is_startup_loading() {
            "Loading layer tree..."
        } else {
            "No layer nodes found."
        };
        vec![ListItem::new(message).style(comment_style(terminal_focused))]
    } else {
        let mut items = Vec::new();
        if let Some(psd) = app.selected_psd_entry() {
            items.push(
                ListItem::new(format!("[PSD] {}", psd.file_name))
                    .style(accent_style(terminal_focused)),
            );
            items.push(
                ListItem::new(format!("[Meta] {}", psd.metadata))
                    .style(comment_style(terminal_focused)),
            );
        }
        items.extend(app.selected_layer_rows().iter().map(|node| {
            let style = if node.visible {
                visible_style(terminal_focused)
            } else {
                base_style(terminal_focused)
            };
            ListItem::new(node.display_label()).style(style)
        }));
        items
    };
    let layer_block = pane_block(
        "Layer Tree",
        terminal_focused,
        app.focus == FocusPane::Layer,
    );
    let layer_inner = layer_block.inner(layer_area);
    let layer_list = List::new(layer_items)
        .block(layer_block)
        .highlight_style(selected_style(terminal_focused))
        .highlight_symbol("> ");
    let mut layer_state = app.layer_list_state(layer_inner.height);
    frame.render_stateful_widget(layer_list, layer_area, &mut layer_state);

    let info = Paragraph::new(info_lines)
        .style(base_style(terminal_focused))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Info")
                .style(base_style(terminal_focused))
                .border_style(border_style(terminal_focused, false)),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(info, left_layout[1]);

    let log = Paragraph::new(log_lines)
        .style(base_style(terminal_focused))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Log")
                .style(base_style(terminal_focused))
                .border_style(border_style(terminal_focused, false)),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(log, left_layout[2]);

    overlay::draw_help_bar(frame, app.help_line(), terminal_focused);

    if !use_server_preview && preview.is_loading() {
        overlay::draw_loading_overlay(
            frame,
            preview.loading_overlay_message(),
            preview.uses_compact_loading_overlay(),
            terminal_focused,
        );
    }

    if let Some(message) = app.processing_overlay_message() {
        overlay::draw_processing_overlay(
            frame,
            preview_area.unwrap_or(layer_area),
            message,
            terminal_focused,
        );
    }

    if !terminal_focused {
        overlay::draw_unfocused_preview_overlay(frame, preview_area);
    }

    if let Some(message) = activity_message {
        overlay::draw_activity_overlay(frame, root_inner, message, terminal_focused);
    }

    if app.is_help_overlay_visible() {
        overlay::draw_help_dialog(frame, app.help_overlay_lines(), terminal_focused);
    }
}

fn pane_block(title: &str, terminal_focused: bool, is_pane_focused: bool) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .style(base_style(terminal_focused))
        .border_style(border_style(terminal_focused, is_pane_focused))
        .title(title.to_string())
}

fn info_panel_height(line_count: usize) -> u16 {
    (line_count as u16).saturating_add(2).clamp(5, 10)
}

fn log_panel_height(line_count: usize) -> u16 {
    (line_count as u16).saturating_add(2).clamp(3, 5)
}

fn left_column_constraint(use_server_preview: bool) -> Constraint {
    if use_server_preview {
        Constraint::Percentage(34)
    } else {
        Constraint::Percentage(67)
    }
}

fn right_column_constraint(use_server_preview: bool) -> Constraint {
    if use_server_preview {
        Constraint::Percentage(66)
    } else {
        Constraint::Percentage(33)
    }
}

fn base_style(focused: bool) -> Style {
    let style = if focused {
        Style::default().fg(MONOKAI_FG).bg(MONOKAI_BG)
    } else {
        Style::default().fg(DIM_FG).bg(DIM_BG)
    };
    apply_dim(style, focused)
}

fn comment_style(focused: bool) -> Style {
    let style = if focused {
        base_style(true).fg(MONOKAI_COMMENT)
    } else {
        base_style(false).fg(DIM_COMMENT)
    };
    apply_dim(style, focused)
}

fn accent_style(focused: bool) -> Style {
    let style = if focused {
        base_style(true)
            .fg(MONOKAI_ORANGE)
            .add_modifier(Modifier::BOLD)
    } else {
        base_style(false)
            .fg(DIM_ACCENT)
            .add_modifier(Modifier::BOLD)
    };
    apply_dim(style, focused)
}

fn visible_style(focused: bool) -> Style {
    let style = if focused {
        base_style(true).fg(MONOKAI_GREEN)
    } else {
        base_style(false).fg(DIM_ACCENT)
    };
    apply_dim(style, focused)
}

fn selected_style(focused: bool) -> Style {
    let style = if focused {
        Style::default()
            .fg(MONOKAI_BG)
            .bg(MONOKAI_CYAN)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(DIM_SELECTED_FG)
            .bg(DIM_SELECTED_BG)
            .add_modifier(Modifier::BOLD)
    };
    apply_dim(style, focused)
}

fn border_style(focused: bool, is_pane_focused: bool) -> Style {
    let style = if focused && is_pane_focused {
        Style::default().fg(MONOKAI_PINK)
    } else if focused {
        Style::default().fg(MONOKAI_COMMENT)
    } else {
        Style::default().fg(DIM_COMMENT)
    };
    apply_dim(style, focused)
}

fn overlay_border_style(focused: bool) -> Style {
    let style = if focused {
        Style::default().fg(MONOKAI_YELLOW)
    } else {
        Style::default().fg(DIM_COMMENT)
    };
    apply_dim(style, focused)
}

fn compact_overlay_style(focused: bool) -> Style {
    let style = if focused {
        base_style(true)
            .fg(MONOKAI_BG)
            .bg(MONOKAI_YELLOW)
            .add_modifier(Modifier::BOLD)
    } else {
        base_style(false)
            .fg(DIM_SELECTED_FG)
            .bg(DIM_SELECTED_BG)
            .add_modifier(Modifier::BOLD)
    };
    apply_dim(style, focused)
}

fn processing_overlay_style(focused: bool) -> Style {
    let style = if focused {
        base_style(true)
            .fg(MONOKAI_BG)
            .bg(MONOKAI_ORANGE)
            .add_modifier(Modifier::BOLD)
    } else {
        base_style(false)
            .fg(DIM_SELECTED_FG)
            .bg(DIM_SELECTED_BG)
            .add_modifier(Modifier::BOLD)
    };
    apply_dim(style, focused)
}

fn activity_indicator_style(focused: bool) -> Style {
    let style = if focused {
        base_style(true).fg(MONOKAI_BG).bg(MONOKAI_CYAN)
    } else {
        base_style(false).fg(DIM_SELECTED_FG).bg(DIM_SELECTED_BG)
    };
    apply_dim(style, focused)
}

fn apply_dim(style: Style, focused: bool) -> Style {
    if focused {
        style
    } else {
        style.add_modifier(Modifier::DIM)
    }
}
