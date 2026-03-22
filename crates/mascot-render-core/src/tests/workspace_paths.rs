use crate::{
    local_data_root, mascot_config_path, workspace_cache_root, workspace_relative_display_path,
};

#[test]
fn default_config_and_cache_live_under_local_data_root() {
    assert!(mascot_config_path().starts_with(local_data_root()));
    assert!(workspace_cache_root().starts_with(local_data_root()));
}

#[test]
fn display_path_trims_local_data_root_prefix() {
    let path = workspace_cache_root().join("demo/render.png");

    assert_eq!(
        workspace_relative_display_path(&path),
        "cache/demo/render.png"
    );
}
