use crate::workspace_update::{update_bat_content, workspace_install_command};

#[test]
fn workspace_install_command_matches_readme_install_command() {
    assert_eq!(
        workspace_install_command(),
        "cargo install --force --git https://github.com/cat2151/mascot-render-server mascot-render-server psd-viewer-tui"
    );
}

#[test]
fn windows_update_bat_stops_both_binaries_and_reinstalls_workspace() {
    let bat = update_bat_content();

    assert!(bat.contains("taskkill /F /IM mascot-render-server.exe >nul 2>nul"));
    assert!(bat.contains("taskkill /F /IM psd-viewer-tui.exe >nul 2>nul"));
    assert!(bat.contains(&workspace_install_command()));
    assert!(bat.contains("if %ERRORLEVEL% neq 0 ("));
    assert!(bat.contains("The update script will not be deleted so you can inspect or rerun it."));
    assert!(bat.contains("pause"));
    assert!(bat.contains("del \"%~f0\""));
}
