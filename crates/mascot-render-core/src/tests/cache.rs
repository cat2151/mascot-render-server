use std::fs;
use std::path::PathBuf;

use serde_json::json;

use crate::{workspace_cache_root, Core, CoreConfig, PsdEntry};

#[test]
fn load_cached_zip_entries_snapshot_reads_cached_meta_without_hashing_zip() {
    let cache_dir = workspace_cache_root().join("test-load-cached-zip-entries-snapshot");
    let _ = fs::remove_dir_all(&cache_dir);

    let live_cache_dir = cache_dir.join("live-hash");
    let stale_cache_dir = cache_dir.join("stale-hash");
    fs::create_dir_all(&live_cache_dir).unwrap();
    fs::create_dir_all(&stale_cache_dir).unwrap();

    let live_zip_path = cache_dir.join("assets/live.zip");
    let live_psd_path = live_cache_dir.join("extracted/live/body.psd");
    let missing_render_path = live_cache_dir.join("renders/live__body.png");
    create_file(&live_zip_path);
    create_file(&live_psd_path);

    let stale_zip_path = cache_dir.join("assets/stale.zip");
    let stale_psd_path = stale_cache_dir.join("extracted/stale/body.psd");
    create_file(&stale_psd_path);

    write_snapshot_meta(
        &live_cache_dir.join("psd-meta.json"),
        &live_zip_path,
        "live-hash",
        vec![sample_psd_entry(&live_psd_path, Some(missing_render_path))],
    );
    write_snapshot_meta(
        &stale_cache_dir.join("psd-meta.json"),
        &stale_zip_path,
        "stale-hash",
        vec![sample_psd_entry(&stale_psd_path, None)],
    );

    let core = Core::new(CoreConfig {
        cache_dir: cache_dir.clone(),
    });
    let zip_entries = core.load_cached_zip_entries_snapshot().unwrap();

    assert_eq!(zip_entries.len(), 1);
    assert_eq!(zip_entries[0].zip_path, live_zip_path);
    assert_eq!(zip_entries[0].zip_hash, "live-hash");
    assert_eq!(zip_entries[0].psds.len(), 1);
    assert_eq!(zip_entries[0].psds[0].rendered_png_path, None);

    let _ = fs::remove_dir_all(&cache_dir);
}

fn sample_psd_entry(path: &std::path::Path, rendered_png_path: Option<PathBuf>) -> PsdEntry {
    PsdEntry {
        path: path.to_path_buf(),
        file_name: "body.psd".to_string(),
        metadata: "100x100 4ch depth 8".to_string(),
        rendered_png_path,
        ..PsdEntry::default()
    }
}

fn write_snapshot_meta(
    path: &std::path::Path,
    zip_path: &std::path::Path,
    zip_hash: &str,
    psds: Vec<PsdEntry>,
) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }

    let body = json!({
        "version": 2,
        "zip_path": zip_path,
        "zip_hash": zip_hash,
        "psds": psds,
        "updated_at": 1,
    });
    fs::write(path, serde_json::to_string_pretty(&body).unwrap()).unwrap();
}

fn create_file(path: &std::path::Path) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, b"test").unwrap();
}
