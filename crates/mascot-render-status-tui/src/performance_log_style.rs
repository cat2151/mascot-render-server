use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

pub(crate) fn styled_performance_log_lines(lines: Vec<String>) -> Vec<Line<'static>> {
    lines
        .into_iter()
        .map(|line| styled_performance_log_line(&line))
        .collect()
}

pub(crate) fn styled_performance_log_line(line: &str) -> Line<'static> {
    let mut spans = Vec::new();
    for (index, token) in line.split_whitespace().enumerate() {
        if index > 0 {
            spans.push(Span::raw(" "));
        }
        push_styled_token(&mut spans, token, index);
    }
    if spans.is_empty() {
        spans.push(Span::raw(""));
    }
    Line::from(spans)
}

fn push_styled_token(spans: &mut Vec<Span<'static>>, token: &str, index: usize) {
    if token == "latest" {
        spans.push(Span::styled(
            token.to_string(),
            Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        ));
        return;
    }

    if token.starts_with("prev") {
        spans.push(Span::styled(
            token.to_string(),
            Style::default().fg(Color::Gray),
        ));
        return;
    }

    if looks_like_time(token) {
        spans.push(Span::styled(
            token.to_string(),
            Style::default().fg(Color::Gray),
        ));
        return;
    }

    if token.ends_with("ms") && (index <= 2 || duration_ms(token).is_some()) {
        spans.push(Span::styled(
            token.to_string(),
            duration_style(duration_ms(token)).add_modifier(Modifier::BOLD),
        ));
        return;
    }

    if matches!(token, "completed" | "failed") {
        spans.push(Span::styled(
            token.to_string(),
            result_style(token).add_modifier(Modifier::BOLD),
        ));
        return;
    }

    if let Some((key, value)) = token.split_once('=') {
        push_key_value_token(spans, key, value);
        return;
    }

    spans.push(Span::raw(token.to_string()));
}

fn push_key_value_token(spans: &mut Vec<Span<'static>>, key: &str, value: &str) {
    match key {
        "q" | "apply" | "settle" => {
            spans.push(Span::raw(format!("{key}=")));
            spans.push(Span::styled(
                value.to_string(),
                duration_style(duration_ms(value)).add_modifier(Modifier::BOLD),
            ));
        }
        "top" => {
            spans.push(Span::styled(
                "top=".to_string(),
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::styled(
                value.to_string(),
                duration_style(stage_duration_ms(value)).add_modifier(Modifier::BOLD),
            ));
        }
        "settled" => {
            spans.push(Span::raw("settled="));
            spans.push(Span::styled(value.to_string(), bool_style(value)));
        }
        "texture_changed" => {
            spans.push(Span::raw("texture_changed="));
            spans.push(Span::styled(
                value.to_string(),
                Style::default().fg(Color::Cyan),
            ));
        }
        "tex" => {
            spans.push(Span::raw("tex="));
            spans.push(Span::styled(value.to_string(), bool_style(value)));
        }
        "png" | "char" => {
            spans.push(Span::raw(format!("{key}=")));
            spans.push(Span::styled(
                value.to_string(),
                Style::default().fg(Color::Gray),
            ));
        }
        _ => spans.push(Span::raw(format!("{key}={value}"))),
    }
}

fn result_style(result: &str) -> Style {
    match result {
        "completed" => Style::default().fg(Color::Green),
        "failed" => Style::default().fg(Color::Red),
        _ => Style::default(),
    }
}

fn bool_style(value: &str) -> Style {
    match value {
        "true" => Style::default().fg(Color::Green),
        "false" => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        _ => Style::default(),
    }
}

fn duration_style(duration_ms: Option<u64>) -> Style {
    match duration_ms {
        Some(ms) if ms >= 5_000 => Style::default().fg(Color::Red),
        Some(ms) if ms >= 1_000 => Style::default().fg(Color::Yellow),
        Some(_) => Style::default().fg(Color::Green),
        None => Style::default().fg(Color::DarkGray),
    }
}

fn duration_ms(value: &str) -> Option<u64> {
    value.strip_suffix("ms")?.parse::<u64>().ok()
}

fn stage_duration_ms(value: &str) -> Option<u64> {
    let (_, duration) = value.rsplit_once(':')?;
    duration_ms(duration)
}

fn looks_like_time(token: &str) -> bool {
    let mut parts = token.split(':');
    let Some(hour) = parts.next() else {
        return false;
    };
    let Some(minute) = parts.next() else {
        return false;
    };
    let Some(second) = parts.next() else {
        return false;
    };
    parts.next().is_none()
        && hour.len() == 2
        && minute.len() == 2
        && second.len() == 2
        && hour.chars().all(|ch| ch.is_ascii_digit())
        && minute.chars().all(|ch| ch.is_ascii_digit())
        && second.chars().all(|ch| ch.is_ascii_digit())
}
