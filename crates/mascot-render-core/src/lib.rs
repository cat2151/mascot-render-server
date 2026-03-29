mod api;
mod archive;
mod cache;
mod core;
mod eye_blink;
mod layer_name_format;
mod logging;
mod mascot;
mod mascot_motion;
mod mascot_paths;
mod model;
mod mouth_flap;
mod psd;
mod render;
mod variation;
mod workspace_paths;
mod workspace_update;

#[cfg(test)]
mod tests;

pub use api::{
    DisplayDiff, LayerDescriptor, LayerVisibilityOverride, PsdDocument, PsdSummary, RenderRequest,
    RenderedPng, VariationSpec, DISPLAY_DIFF_VERSION, VARIATION_SPEC_VERSION,
};
pub use archive::{display_path, existing_zip_sources};
pub use core::{Core, CoreConfig};
pub use eye_blink::{
    auto_generate_eye_blink_target, build_closed_eye_display_diff, resolve_eye_blink_rows,
    EyeBlinkRows, EyeBlinkTarget, AUTO_EYE_BLINK_SECOND_LAYER_KEYWORDS, BASIC_EYE_LAYER,
    CLOSED_EYE_LAYER, CLOSED_EYE_LAYER_ALT_1, CLOSED_EYE_LAYER_ALT_2, EYE_SET_LAYER,
    NORMAL_EYE_LAYER, SMILE_LAYER,
};
pub use layer_name_format::{
    is_exclusive_kind, is_exclusive_name, is_mandatory_kind, is_mandatory_name, is_toggleable_kind,
};
pub use logging::log_file_name;
pub use mascot::{
    default_mascot_scale_for_screen_height, load_favorite_ensemble_enabled, load_mascot_config,
    load_mascot_image, mascot_config_path, mascot_runtime_state_path, mascot_window_size,
    parse_mascot_config_path, psd_viewer_tui_activity_path, set_favorite_ensemble_enabled,
    unix_timestamp, write_mascot_config, MascotConfig, MascotImageData, MascotTarget,
};
pub use mascot_motion::{
    AlwaysBendConfig, BendConfig, BounceAlgorithm, BounceAnimationConfig, IdleAlgorithm,
    IdleSinkAnimationConfig, MotionState, MotionTransform, SquashAlgorithm,
    SquashBounceAnimationConfig, IDLE_SINK_LIFT_SCALE_X_RATIO,
};
pub use model::{LayerKind, LayerNode, PsdEntry, ZipEntry};
pub use mouth_flap::{
    auto_generate_mouth_flap_target, build_mouth_flap_display_diffs,
    describe_mouth_flap_auto_generation_failure, resolve_mouth_flap_rows, MouthFlapDisplayDiffs,
    MouthFlapRows, MouthFlapTarget, MOUTH_CLOSED_LAYER, MOUTH_CLOSED_LAYER_ALT_1,
    MOUTH_CLOSED_LAYER_ALT_2, MOUTH_CLOSED_LAYER_ALT_3, MOUTH_CLOSED_LAYER_ALT_4,
    MOUTH_GROUP_LAYER, MOUTH_OPEN_LAYER, MOUTH_OPEN_LAYER_ALT_1, MOUTH_OPEN_LAYER_ALT_2,
    MOUTH_OPEN_LAYER_ALT_3,
};
pub use variation::{
    load_variation_spec, save_variation_spec, variation_hash, variation_png_path,
    variation_render_meta_path, variation_spec_path,
};
pub use workspace_paths::{
    local_data_root, workspace_cache_root, workspace_log_root, workspace_path,
    workspace_relative_display_path, workspace_root,
};
pub use workspace_update::{run_workspace_update, workspace_install_command};
