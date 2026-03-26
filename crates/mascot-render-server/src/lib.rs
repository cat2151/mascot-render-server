mod favorite_shuffle;
mod mascot_control;
mod mascot_skin_cache;
mod motion_timeline;
mod transparent_hit_test;
mod window_history;
mod window_layout;
mod window_region;
mod window_region_sync;

#[cfg(test)]
mod eye_blink;
#[cfg(test)]
mod eye_blink_timing;
#[cfg(test)]
mod tests;

pub use favorite_shuffle::{FavoriteShufflePlaylist, FAVORITE_SHUFFLE_INTERVAL};
pub use mascot_control::{
    ensure_mascot_render_server_visible, play_mascot_render_server_timeline,
    start_mascot_control_server, start_mascot_control_server_with_notify,
    sync_mascot_render_server_preview, MascotControlCommand,
};
pub use mascot_skin_cache::MascotSkinCache;
pub use motion_timeline::{apply_motion_timeline_request, validate_motion_timeline_request};
pub use transparent_hit_test::captures_logical_point;
pub use transparent_hit_test::TransparentHitTestUpdate;
pub use transparent_hit_test::TransparentHitTestWindow;
pub use window_history::{
    load_saved_window_position_for_paths, save_window_position_for_paths,
    window_history_path_for_paths, SavedWindowPosition,
};
pub use window_layout::{
    alpha_bounds_from_mask, anchored_inner_origin, squash_bounce_bounds_config,
    transformed_image_rect, AlphaBounds, MascotWindowLayout,
};
