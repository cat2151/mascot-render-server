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
    if line.contains("event=skin_load") {
        return compact_skin_load_log_line(line);
    }

    if line.contains("event=mouth_flap_png_generation") {
        return compact_mouth_flap_log_line(line);
    }

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
    let stage_ms = key_value(line, "stage_ms");
    let top_stage = stage_ms.and_then(slowest_stage);
    let stage_breakdown = stage_ms.and_then(|stage_ms| slowest_stages(stage_ms, 3));
    let target = compact_target(line);

    let mut compact =
        format!("{timestamp} {elapsed_ms} {action} {result} q={queue_ms} apply={apply_ms} settle={settle_ms}");
    if let Some(top_stage) = top_stage {
        compact.push_str(" top=");
        compact.push_str(&top_stage);
    }
    if let Some(stage_breakdown) = stage_breakdown {
        compact.push_str(" parts=");
        compact.push_str(&stage_breakdown);
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

fn compact_skin_load_log_line(line: &str) -> String {
    let timestamp = compact_timestamp(line);
    let stage = key_value(line, "stage").unwrap_or("-");
    let elapsed_ms = key_value(line, "elapsed_ms")
        .map(|value| format!("{value}ms"))
        .unwrap_or_else(|| "-".to_string());
    match stage {
        "memory_cache_hit" => {
            let lookup_ms = duration_value(line, "cache_lookup_ms");
            let target = key_value(line, "png_path")
                .map(compact_path_tail)
                .unwrap_or_else(|| "-".to_string());
            format!("{timestamp} {elapsed_ms} skin memory_cache lookup={lookup_ms} png={target}")
        }
        "cache_miss_loaded" => {
            let raw_cache = key_value(line, "raw_rgba_cache_status").unwrap_or("-");
            let raw_meta_ms = duration_value(line, "raw_rgba_meta_read_ms");
            let raw_read_ms = duration_value(line, "raw_rgba_read_ms");
            let read_ms = duration_value(line, "read_file_ms");
            let decode_ms = duration_value(line, "decode_png_ms");
            let detail_cache = key_value(line, "detail_cache_hit").unwrap_or("-");
            let detail_read_ms = duration_value(line, "detail_cache_read_ms");
            let alpha_ms = duration_value(line, "alpha_mask_ms");
            let bounds_ms = duration_value(line, "content_bounds_ms");
            let detail_write_ms = duration_value(line, "detail_cache_write_ms");
            let texture_ms = duration_value(line, "texture_alloc_ms");
            let cache_insert_ms = duration_value(line, "cache_insert_ms");
            let image_size = key_value(line, "image_size").unwrap_or("-");
            let file_bytes = key_value(line, "file_bytes").unwrap_or("-");
            let target = key_value(line, "png_path")
                .map(compact_path_tail)
                .unwrap_or_else(|| "-".to_string());
            format!(
                "{timestamp} {elapsed_ms} skin file_load raw={raw_cache} raw_meta={raw_meta_ms} raw_read={raw_read_ms} read={read_ms} decode={decode_ms} detail_cache={detail_cache} detail_read={detail_read_ms} alpha={alpha_ms} bounds={bounds_ms} detail_write={detail_write_ms} texture={texture_ms} cache={cache_insert_ms} size={image_size} bytes={file_bytes} png={target}"
            )
        }
        _ => format!("{timestamp} {elapsed_ms} skin {stage}"),
    }
}

fn compact_mouth_flap_log_line(line: &str) -> String {
    let timestamp = compact_timestamp(line);
    let stage = key_value(line, "stage").unwrap_or("-");
    let elapsed_ms = key_value(line, "elapsed_ms")
        .map(|value| format!("{value}ms"))
        .unwrap_or_else(|| "-".to_string());

    match stage {
        "completed" => compact_mouth_flap_completed_line(line, &timestamp, &elapsed_ms),
        "render_open_png" | "render_closed_png" => {
            compact_mouth_flap_render_line(line, &timestamp, stage, &elapsed_ms)
        }
        "inspect_psd" => compact_mouth_flap_inspect_line(line, &timestamp, &elapsed_ms),
        "find_target" => compact_mouth_flap_find_target_line(line, &timestamp, &elapsed_ms),
        _ => format!("{timestamp} {elapsed_ms} mouth_flap {stage}"),
    }
}

fn compact_mouth_flap_completed_line(line: &str, timestamp: &str, elapsed_ms: &str) -> String {
    let miss_count = key_value(line, "variation_png_cache_miss_count").unwrap_or("-");
    let zip_extracted = key_value(line, "any_zip_extracted").unwrap_or("-");
    let psd_meta_rebuilt = key_value(line, "any_psd_meta_rebuilt").unwrap_or("-");
    let open_cache = key_value(line, "open_render_cache_hit").unwrap_or("-");
    let closed_cache = key_value(line, "closed_render_cache_hit").unwrap_or("-");
    let target = key_value(line, "open_png_path")
        .map(compact_path_tail)
        .unwrap_or_else(|| "-".to_string());

    format!(
        "{timestamp} {elapsed_ms} mouth_flap completed miss={miss_count} zip={zip_extracted} psd_meta={psd_meta_rebuilt} open_cache={open_cache} closed_cache={closed_cache} png={target}"
    )
}

fn compact_mouth_flap_render_line(
    line: &str,
    timestamp: &str,
    stage: &str,
    elapsed_ms: &str,
) -> String {
    let cache_hit = key_value(line, "render_cache_hit").unwrap_or("-");
    let analyze_ms = duration_value(line, "custom_psd_analyze_ms");
    let compose_ms = duration_value(line, "compose_and_save_png_ms");
    let zip_extracted = key_value(line, "zip_extracted").unwrap_or("-");
    let psd_meta_rebuilt = key_value(line, "psd_meta_rebuilt").unwrap_or("-");
    let target = key_value(line, "output_path")
        .map(compact_path_tail)
        .unwrap_or_else(|| "-".to_string());

    format!(
        "{timestamp} {elapsed_ms} mouth_flap {stage} cache={cache_hit} analyze={analyze_ms} compose={compose_ms} zip={zip_extracted} psd_meta={psd_meta_rebuilt} png={target}"
    )
}

fn compact_mouth_flap_inspect_line(line: &str, timestamp: &str, elapsed_ms: &str) -> String {
    let layer_count = key_value(line, "layer_count").unwrap_or("-");
    let zip_extracted = key_value(line, "zip_extracted").unwrap_or("-");
    let psd_meta_rebuilt = key_value(line, "psd_meta_rebuilt").unwrap_or("-");
    let psd_entries_built = key_value(line, "psd_entries_built").unwrap_or("-");
    let zip_load_ms = duration_value(line, "zip_load_ms");

    format!(
        "{timestamp} {elapsed_ms} mouth_flap inspect layers={layer_count} zip_load={zip_load_ms} zip={zip_extracted} psd_meta={psd_meta_rebuilt} psds={psd_entries_built}"
    )
}

fn compact_mouth_flap_find_target_line(line: &str, timestamp: &str, elapsed_ms: &str) -> String {
    let open_layers = key_value(line, "open_layers").unwrap_or("-");
    let closed_layers = key_value(line, "closed_layers").unwrap_or("-");
    let psd_file = key_value(line, "psd_file")
        .map(|value| shorten_middle(value, 28))
        .unwrap_or_else(|| "-".to_string());

    format!(
        "{timestamp} {elapsed_ms} mouth_flap find_target open={open_layers} closed={closed_layers} psd={psd_file}"
    )
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
    slowest_stages(stage_ms, 1)
}

fn slowest_stages(stage_ms: &str, limit: usize) -> Option<String> {
    if stage_ms == "none" {
        return None;
    }

    let stages = stage_ms
        .split(',')
        .filter_map(parse_stage_duration)
        .collect::<Vec<_>>();
    let mut stages = visible_stage_breakdown(stages);
    stages.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(right.0)));
    let summary = stages
        .into_iter()
        .take(limit)
        .map(|(stage, elapsed_ms)| format!("{stage}:{elapsed_ms}ms"))
        .collect::<Vec<_>>()
        .join("|");
    (!summary.is_empty()).then_some(summary)
}

fn visible_stage_breakdown(stages: Vec<(&str, u64)>) -> Vec<(&str, u64)> {
    if !stages
        .iter()
        .any(|(stage, _)| stage.starts_with("mouth_flap."))
    {
        return stages;
    }

    let leaf_stages = stages
        .iter()
        .copied()
        .filter(|(stage, _)| !is_mouth_flap_aggregate_stage(stage))
        .collect::<Vec<_>>();
    if leaf_stages.is_empty() {
        stages
    } else {
        leaf_stages
    }
}

fn is_mouth_flap_aggregate_stage(stage: &str) -> bool {
    matches!(
        stage,
        "refresh_mouth_flap_skins"
            | "refresh_pending_auxiliary_skins.refresh_mouth_flap_skins"
            | "mouth_flap.inspect_psd"
            | "mouth_flap.render_open_png"
            | "mouth_flap.render_closed_png"
    )
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
