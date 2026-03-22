#[cfg(target_os = "windows")]
use std::time::{Duration, Instant};

#[cfg(target_os = "windows")]
use crate::window_region::WindowRegionKey;

#[cfg(target_os = "windows")]
const WINDOW_REGION_UPDATE_DEBOUNCE: Duration = Duration::from_secs(1);

#[cfg(target_os = "windows")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WindowRegionSyncAction {
    None,
    Clear,
    Apply,
}

#[cfg(target_os = "windows")]
#[derive(Debug, Clone, Copy)]
pub(crate) struct WindowRegionSyncState {
    applied_key: Option<WindowRegionKey>,
    pending_key: Option<WindowRegionKey>,
    pending_since: Option<Instant>,
    active: bool,
}

#[cfg(target_os = "windows")]
impl WindowRegionSyncState {
    pub(crate) fn new() -> Self {
        Self {
            applied_key: None,
            pending_key: None,
            pending_since: None,
            active: false,
        }
    }

    pub(crate) fn next_action(
        &mut self,
        now: Instant,
        region_key: WindowRegionKey,
    ) -> WindowRegionSyncAction {
        if !region_key.is_enabled() {
            self.pending_key = None;
            self.pending_since = None;
            return if self.applied_key == Some(region_key) {
                WindowRegionSyncAction::None
            } else {
                WindowRegionSyncAction::Apply
            };
        }

        if self.pending_key.is_some() {
            if self.pending_key != Some(region_key) {
                self.pending_key = Some(region_key);
                self.pending_since = Some(now);
                return if self.active {
                    WindowRegionSyncAction::Clear
                } else {
                    WindowRegionSyncAction::None
                };
            }

            if self
                .pending_since
                .is_some_and(|since| now.duration_since(since) >= WINDOW_REGION_UPDATE_DEBOUNCE)
            {
                self.pending_key = None;
                self.pending_since = None;
                return WindowRegionSyncAction::Apply;
            }

            return if self.active {
                WindowRegionSyncAction::Clear
            } else {
                WindowRegionSyncAction::None
            };
        }

        if self.applied_key == Some(region_key) {
            return if self.active {
                WindowRegionSyncAction::None
            } else {
                WindowRegionSyncAction::Apply
            };
        }

        if self
            .applied_key
            .is_none_or(|applied| region_key.requires_immediate_apply(applied))
        {
            return WindowRegionSyncAction::Apply;
        }

        self.pending_key = Some(region_key);
        self.pending_since = Some(now);
        if self.active {
            WindowRegionSyncAction::Clear
        } else {
            WindowRegionSyncAction::None
        }
    }

    pub(crate) fn mark_applied(&mut self, region_key: WindowRegionKey, active: bool) {
        self.applied_key = Some(region_key);
        self.pending_key = None;
        self.pending_since = None;
        self.active = active;
    }

    pub(crate) fn mark_cleared(&mut self) {
        self.active = false;
    }

    pub(crate) fn is_active(self) -> bool {
        self.active
    }

    #[cfg(test)]
    pub(crate) fn pending_key(self) -> Option<WindowRegionKey> {
        self.pending_key
    }
}
