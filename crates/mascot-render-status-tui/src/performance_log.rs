use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub(crate) const PERFORMANCE_LOG_TAIL_LINES: usize = 6;
const PERFORMANCE_LOG_VISIBLE_EVENTS: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PerformanceLogSnapshot {
    pub(crate) lines: Vec<String>,
}

pub(crate) struct PerformanceLogPoll {
    path: PathBuf,
    last_modified_at: Option<SystemTime>,
    last_len: Option<u64>,
    initialized: bool,
}

impl PerformanceLogPoll {
    pub(crate) fn new(path: PathBuf) -> Self {
        Self {
            path,
            last_modified_at: None,
            last_len: None,
            initialized: false,
        }
    }

    pub(crate) fn poll(&mut self) -> Option<Result<PerformanceLogSnapshot, String>> {
        let metadata = match fs::metadata(&self.path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                if self.initialized && self.last_modified_at.is_none() && self.last_len.is_none() {
                    return None;
                }
                self.initialized = true;
                self.last_modified_at = None;
                self.last_len = None;
                return Some(Ok(PerformanceLogSnapshot {
                    lines: vec![format!("not created: {}", self.path.display())],
                }));
            }
            Err(error) => {
                return Some(Err(format!(
                    "failed to stat server performance log {}: {error}",
                    self.path.display()
                )));
            }
        };

        let modified_at = metadata.modified().ok();
        let len = metadata.len();
        if self.initialized && self.last_modified_at == modified_at && self.last_len == Some(len) {
            return None;
        }

        self.initialized = true;
        self.last_modified_at = modified_at;
        self.last_len = Some(len);

        Some(
            read_tail(&self.path, PERFORMANCE_LOG_TAIL_LINES).map(|lines| PerformanceLogSnapshot {
                lines: compact_performance_log_lines(lines),
            }),
        )
    }
}

fn read_tail(path: &Path, max_lines: usize) -> Result<Vec<String>, String> {
    let contents = fs::read_to_string(path).map_err(|error| {
        format!(
            "failed to read server performance log {}: {error}",
            path.display()
        )
    })?;
    Ok(tail_lines(&contents, max_lines))
}

pub(crate) fn tail_lines(contents: &str, max_lines: usize) -> Vec<String> {
    let mut lines = contents
        .lines()
        .rev()
        .take(max_lines)
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    lines.reverse();
    if lines.is_empty() {
        vec!["empty".to_string()]
    } else {
        lines
    }
}

pub(crate) fn compact_performance_log_line(line: &str) -> String {
    if !line.contains("event=post_to_status_settled") {
        return shorten_middle(line, 120);
    }

    let timestamp = compact_timestamp(line);
    let action = key_value(line, "action").unwrap_or("-");
    let result = key_value(line, "result").unwrap_or("-");
    let elapsed_ms = key_value(line, "elapsed_ms")
        .map(|value| format!("{value}ms"))
        .unwrap_or_else(|| "-".to_string());
    let queue_ms = duration_value(line, "queue_ms");
    let apply_ms = duration_value(line, "apply_ms");
    let settle_ms = duration_value(line, "settle_ms");
    let settled = key_value(line, "status_settled").unwrap_or("-");
    let texture_changed = key_value(line, "texture_changed").unwrap_or("-");
    let top_stage = key_value(line, "stage_ms").and_then(slowest_stage);
    let target = compact_target(line);

    let mut compact =
        format!("{timestamp} {elapsed_ms} {action} {result} q={queue_ms} apply={apply_ms} settle={settle_ms}");
    if let Some(top_stage) = top_stage {
        compact.push_str(" top=");
        compact.push_str(&top_stage);
    }
    if settled != "true" {
        compact.push_str(" settled=");
        compact.push_str(settled);
    }
    if texture_changed != "true" {
        compact.push_str(" tex=");
        compact.push_str(texture_changed);
    }
    if let Some((key, value)) = target {
        compact.push(' ');
        compact.push_str(key);
        compact.push('=');
        compact.push_str(&value);
    }
    compact
}

pub(crate) fn compact_performance_log_lines(lines: Vec<String>) -> Vec<String> {
    if lines.len() == 1 && (lines[0] == "empty" || lines[0].starts_with("not created:")) {
        return lines;
    }

    lines
        .into_iter()
        .rev()
        .take(PERFORMANCE_LOG_VISIBLE_EVENTS)
        .enumerate()
        .map(|(index, line)| {
            let prefix = if index == 0 {
                "latest".to_string()
            } else {
                format!("prev{index}")
            };
            format!("{prefix} {}", compact_performance_log_line(&line))
        })
        .collect()
}

fn key_value<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    let prefix = format!("{key}=");
    text.split_whitespace()
        .find_map(|part| part.strip_prefix(&prefix))
}

fn compact_timestamp(line: &str) -> String {
    let Some(timestamp) = line.strip_prefix('[').and_then(|line| line.split_once(']')) else {
        return "--:--:--".to_string();
    };
    timestamp
        .0
        .split_whitespace()
        .nth(1)
        .unwrap_or(timestamp.0)
        .trim_end_matches('Z')
        .split_once('.')
        .map(|(time, _)| time)
        .unwrap_or(timestamp.0)
        .to_string()
}

fn duration_value(line: &str, key: &str) -> String {
    key_value(line, key)
        .map(|value| format!("{value}ms"))
        .unwrap_or_else(|| "-".to_string())
}

fn compact_target(line: &str) -> Option<(&'static str, String)> {
    if let Some(summary) = key_value(line, "command_summary") {
        if let Some(character) = summary.strip_prefix("character=") {
            return Some(("char", shorten_middle(character, 24)));
        }
    }

    key_value(line, "displayed_png_path")
        .map(compact_path_tail)
        .map(|path| ("png", path))
}

fn slowest_stage(stage_ms: &str) -> Option<String> {
    if stage_ms == "none" {
        return None;
    }

    stage_ms
        .split(',')
        .filter_map(parse_stage_duration)
        .max_by_key(|(_, elapsed_ms)| *elapsed_ms)
        .map(|(stage, elapsed_ms)| format!("{stage}:{elapsed_ms}ms"))
}

fn parse_stage_duration(stage_ms: &str) -> Option<(&str, u64)> {
    let (stage, elapsed_ms) = stage_ms.rsplit_once(':')?;
    let elapsed_ms = elapsed_ms.strip_suffix("ms").unwrap_or(elapsed_ms);
    let elapsed_ms = elapsed_ms.parse::<u64>().ok()?;
    Some((stage, elapsed_ms))
}

fn compact_path_tail(path: &str) -> String {
    let tail = path
        .rsplit(['/', '\\'])
        .next()
        .filter(|tail| !tail.is_empty())
        .unwrap_or(path);
    shorten_middle(tail, 36)
}

fn shorten_middle(text: &str, max_chars: usize) -> String {
    let char_count = text.chars().count();
    if char_count <= max_chars {
        return text.to_string();
    }
    if max_chars <= 3 {
        return ".".repeat(max_chars);
    }

    let head_len = (max_chars - 3) / 2;
    let tail_len = max_chars - 3 - head_len;
    let head = text.chars().take(head_len).collect::<String>();
    let tail = text
        .chars()
        .rev()
        .take(tail_len)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<String>();
    format!("{head}...{tail}")
}
