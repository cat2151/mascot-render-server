use ratatui_image::picker::{Capability, ProtocolType};

use crate::logging::{
    capability_name, protocol_type_name, sixel_preview_timing_log_path, terminal_protocol_log_path,
};
use crate::workspace_paths::workspace_log_root;

#[test]
fn terminal_protocol_log_path_is_json_file() {
    assert_eq!(
        terminal_protocol_log_path(),
        workspace_log_root().join("terminal-protocol.json")
    );
}

#[test]
fn sixel_preview_timing_log_path_is_jsonl_file() {
    assert_eq!(
        sixel_preview_timing_log_path(),
        workspace_log_root().join("sixel-preview-timings.jsonl")
    );
}

#[test]
fn terminal_protocol_labels_are_stable() {
    assert_eq!(protocol_type_name(ProtocolType::Halfblocks), "halfblocks");
    assert_eq!(protocol_type_name(ProtocolType::Iterm2), "iterm2");
    assert_eq!(capability_name(&Capability::Sixel), "sixel");
    assert_eq!(
        capability_name(&Capability::CellSize(Some((10, 20)))),
        "cell_size:10x20"
    );
}
