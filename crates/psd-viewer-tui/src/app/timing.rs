use std::time::{Duration, Instant};

use mascot_render_control::log_psd_viewer_tui_info;

const SLOW_TIMING_LOG_THRESHOLD: Duration = Duration::from_millis(100);

#[derive(Debug)]
pub(super) struct TimingLog {
    action: &'static str,
    summary: String,
    started_at: Instant,
    stages: Vec<TimingStage>,
}

#[derive(Debug)]
struct TimingStage {
    name: &'static str,
    duration_ms: u128,
}

impl TimingLog {
    pub(super) fn start(action: &'static str, summary: impl Into<String>) -> Self {
        Self {
            action,
            summary: summary.into(),
            started_at: Instant::now(),
            stages: Vec::new(),
        }
    }

    pub(super) fn measure<T>(&mut self, name: &'static str, operation: impl FnOnce() -> T) -> T {
        let started_at = Instant::now();
        let output = operation();
        self.record_stage(name, started_at.elapsed());
        output
    }

    pub(super) fn measure_result<T, E>(
        &mut self,
        name: &'static str,
        operation: impl FnOnce() -> Result<T, E>,
    ) -> Result<T, E> {
        let started_at = Instant::now();
        let output = operation();
        self.record_stage(name, started_at.elapsed());
        output
    }

    fn record_stage(&mut self, name: &'static str, duration: Duration) {
        self.stages.push(TimingStage {
            name,
            duration_ms: duration.as_millis(),
        });
    }
}

impl Drop for TimingLog {
    fn drop(&mut self) {
        let elapsed = self.started_at.elapsed();
        if elapsed < SLOW_TIMING_LOG_THRESHOLD {
            return;
        }

        log_psd_viewer_tui_info(format_timing_log_message(
            self.action,
            &self.summary,
            elapsed.as_millis(),
            self.stages
                .iter()
                .map(|stage| (stage.name, stage.duration_ms)),
        ));
    }
}

fn format_timing_log_message<'a>(
    action: &str,
    summary: &str,
    total_ms: u128,
    stages: impl IntoIterator<Item = (&'a str, u128)>,
) -> String {
    let stages = stages
        .into_iter()
        .map(|(name, duration_ms)| format!("{name}:{duration_ms}ms"))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "trigger=selection_timing action={} total_ms={} summary=\"{}\" stages=[{}]",
        sanitize_log_value(action),
        total_ms,
        sanitize_log_value(summary),
        stages
    )
}

fn sanitize_log_value(value: &str) -> String {
    value.replace(['\r', '\n'], " ")
}

#[cfg(test)]
pub(crate) fn format_timing_log_message_for_test(
    action: &str,
    summary: &str,
    total_ms: u128,
    stages: &[(&str, u128)],
) -> String {
    format_timing_log_message(action, summary, total_ms, stages.iter().copied())
}
