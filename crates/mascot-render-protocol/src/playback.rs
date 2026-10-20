use crate::now_unix_ms;
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

pub const PLAYBACK_LEASE: Duration = Duration::from_millis(750);
pub const DEFAULT_MOUTH_FLAP_FPS: u16 = 4;

/// Ephemeral localhost playback state; no configuration is persisted.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlaybackRequest {
    pub sent_unix_ms: u64,
    pub playing: bool,
    pub beat: Option<f64>,
    pub beats_per_second: f64,
    pub singing: bool,
}
impl PlaybackRequest {
    pub fn validate(&self) -> Result<()> {
        if self.beat.is_some_and(|v| !v.is_finite() || v < 0.0)
            || !self.beats_per_second.is_finite()
            || self.beats_per_second <= 0.0
        {
            bail!("invalid playback beat or rate");
        }
        Ok(())
    }
}
#[derive(Debug, Clone)]
pub struct PlaybackSnapshot {
    pub request: PlaybackRequest,
    pub received: Instant,
    pub expires: Instant,
    pub mouth_started: Instant,
}
impl PlaybackSnapshot {
    pub fn is_active(&self, now: Instant) -> bool {
        self.request.playing && now < self.expires
    }
    pub fn beat(&self, now: Instant) -> Option<f64> {
        self.is_active(now).then_some(())?;
        self.request.beat.map(|beat| {
            beat + now.saturating_duration_since(self.received).as_secs_f64()
                * self.request.beats_per_second
        })
    }
    pub fn mouth_open(&self, now: Instant) -> Option<bool> {
        (self.is_active(now) && self.request.singing).then(|| {
            let frame = (now
                .saturating_duration_since(self.mouth_started)
                .as_secs_f64()
                * f64::from(DEFAULT_MOUTH_FLAP_FPS)) as u64;
            frame.is_multiple_of(2)
        })
    }
}
#[derive(Default)]
pub(crate) struct PlaybackSlot(pub Option<PlaybackSnapshot>);
impl PlaybackSlot {
    pub fn set(&mut self, request: PlaybackRequest, now: Instant, unix_ms: u64) -> Result<()> {
        request.validate()?;
        let age = Duration::from_millis(unix_ms.saturating_sub(request.sent_unix_ms));
        // Requests waiting in an HTTP socket must not resurrect expired playback.
        if age >= PLAYBACK_LEASE {
            return Ok(());
        }
        let mouth_started = self
            .0
            .as_ref()
            .filter(|old| old.is_active(now) && old.request.singing && request.singing)
            .map_or(now, |old| old.mouth_started);
        let mut request = request;
        if let Some(beat) = request.beat.as_mut() {
            *beat += age.as_secs_f64() * request.beats_per_second;
        }
        self.0 = Some(PlaybackSnapshot {
            request,
            received: now,
            expires: now + PLAYBACK_LEASE - age,
            mouth_started,
        });
        Ok(())
    }
    pub fn update(&mut self, request: PlaybackRequest) -> Result<()> {
        self.set(request, Instant::now(), now_unix_ms())
    }
}
