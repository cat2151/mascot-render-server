#![cfg(target_os = "windows")]

use std::time::{Duration, Instant};

use eframe::egui::{Pos2, Rect};

use crate::window_region::WindowRegionKey;
use crate::window_region_sync::{WindowRegionSyncAction, WindowRegionSyncState};

static ALPHA_MASK: [u8; 16] = [255; 16];

fn region_key(enabled: bool, min_x: f32, width: f32) -> WindowRegionKey {
    WindowRegionKey::new(
        enabled,
        [4, 4],
        Rect::from_min_max(Pos2::new(min_x, 0.0), Pos2::new(min_x + width, 4.0)),
        1.0,
        &ALPHA_MASK,
    )
}

#[test]
fn animated_bounds_clear_region_until_new_shape_stabilizes() {
    let mut sync = WindowRegionSyncState::new();
    let start = Instant::now();
    let stable = region_key(true, 0.0, 4.0);
    let animated = region_key(true, 1.0, 4.0);

    assert_eq!(
        sync.next_action(start, stable),
        WindowRegionSyncAction::Apply
    );
    sync.mark_applied(stable, true);

    assert_eq!(
        sync.next_action(start + Duration::from_millis(16), animated),
        WindowRegionSyncAction::Clear
    );
    sync.mark_cleared();
    assert_eq!(sync.pending_key(), Some(animated));

    assert_eq!(
        sync.next_action(start + Duration::from_millis(500), animated),
        WindowRegionSyncAction::None
    );
    assert_eq!(sync.pending_key(), Some(animated));

    assert_eq!(
        sync.next_action(start + Duration::from_millis(1100), animated),
        WindowRegionSyncAction::Apply
    );
}

#[test]
fn debounce_keeps_region_disabled_even_if_shape_returns_to_previous_bounds() {
    let mut sync = WindowRegionSyncState::new();
    let start = Instant::now();
    let stable = region_key(true, 0.0, 4.0);
    let animated = region_key(true, 1.0, 4.0);

    assert_eq!(
        sync.next_action(start, stable),
        WindowRegionSyncAction::Apply
    );
    sync.mark_applied(stable, true);

    assert_eq!(
        sync.next_action(start + Duration::from_millis(16), animated),
        WindowRegionSyncAction::Clear
    );
    sync.mark_cleared();

    assert_eq!(
        sync.next_action(start + Duration::from_millis(32), stable),
        WindowRegionSyncAction::None
    );
    assert_eq!(sync.pending_key(), Some(stable));

    assert_eq!(
        sync.next_action(start + Duration::from_millis(1200), stable),
        WindowRegionSyncAction::Apply
    );
}
