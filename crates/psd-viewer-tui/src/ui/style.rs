use ratatui::style::{Color, Modifier, Style};

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

pub(super) fn base_style(focused: bool) -> Style {
    let style = if focused {
        Style::default().fg(MONOKAI_FG).bg(MONOKAI_BG)
    } else {
        Style::default().fg(DIM_FG).bg(DIM_BG)
    };
    apply_dim(style, focused)
}

pub(super) fn comment_style(focused: bool) -> Style {
    let style = if focused {
        base_style(true).fg(MONOKAI_COMMENT)
    } else {
        base_style(false).fg(DIM_COMMENT)
    };
    apply_dim(style, focused)
}

pub(super) fn accent_style(focused: bool) -> Style {
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

pub(super) fn visible_style(focused: bool) -> Style {
    let style = if focused {
        base_style(true).fg(MONOKAI_GREEN)
    } else {
        base_style(false).fg(DIM_ACCENT)
    };
    apply_dim(style, focused)
}

pub(super) fn selected_style(focused: bool) -> Style {
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

pub(super) fn border_style(focused: bool, is_pane_focused: bool) -> Style {
    let style = if focused && is_pane_focused {
        Style::default().fg(MONOKAI_PINK)
    } else if focused {
        Style::default().fg(MONOKAI_COMMENT)
    } else {
        Style::default().fg(DIM_COMMENT)
    };
    apply_dim(style, focused)
}

pub(super) fn overlay_border_style(focused: bool) -> Style {
    let style = if focused {
        Style::default().fg(MONOKAI_YELLOW)
    } else {
        Style::default().fg(DIM_COMMENT)
    };
    apply_dim(style, focused)
}

pub(super) fn compact_overlay_style(focused: bool) -> Style {
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

pub(super) fn processing_overlay_style(focused: bool) -> Style {
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

pub(super) fn activity_indicator_style(focused: bool) -> Style {
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
