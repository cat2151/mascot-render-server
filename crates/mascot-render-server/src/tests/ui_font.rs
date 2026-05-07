#[cfg(target_os = "windows")]
use std::path::{Path, PathBuf};

#[cfg(target_os = "windows")]
use crate::ui_font::windows_japanese_font_candidates_for_test;

#[cfg(target_os = "windows")]
#[test]
fn windows_font_candidates_use_windows_fonts_directory() {
    let candidates = windows_japanese_font_candidates_for_test(&[], Path::new(r"D:\Windows"));
    let expected = [
        r"D:\Windows\Fonts\YuGothR.ttc",
        r"D:\Windows\Fonts\YuGothM.ttc",
        r"D:\Windows\Fonts\meiryo.ttc",
        r"D:\Windows\Fonts\msgothic.ttc",
    ];

    assert_eq!(
        candidates,
        expected
            .iter()
            .map(|path| PathBuf::from(*path))
            .collect::<Vec<_>>()
    );
}

#[cfg(target_os = "windows")]
#[test]
fn configured_windows_font_candidates_override_default_search_order() {
    let configured = vec![
        PathBuf::from(r"custom\font-a.ttf"),
        PathBuf::from(r"C:\fonts\font-b.ttf"),
    ];

    let candidates =
        windows_japanese_font_candidates_for_test(&configured, Path::new(r"D:\Windows"));

    assert_eq!(candidates, configured);
}
