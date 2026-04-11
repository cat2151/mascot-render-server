use std::fs;
use std::fs::File;
use std::path::PathBuf;
use std::time::Duration;

use serde_json::json;
use zip::ZipWriter;

use crate::cache::{zip_cache_key_for_test, zip_source_stamp_for_test, ZipSourceStamp};
use crate::{workspace_cache_root, Core, CoreConfig, PsdEntry};

#[test]
fn load_cached_zip_entries_snapshot_trusts_matching_persisted_meta() {
    let cache_dir = workspace_cache_root().join("test-load-cached-zip-entries-snapshot");
    let _ = fs::remove_dir_all(&cache_dir);

    let live_zip_path = cache_dir.join("assets/live.zip");
    create_file(&live_zip_path);
    let live_cache_key = zip_cache_key_for_test(&live_zip_path).unwrap();
    let live_source = zip_source_stamp_for_test(&live_zip_path).unwrap();
    let live_cache_dir = cache_dir.join(&live_cache_key);
    let live_psd_path = live_cache_dir.join("extracted/live/body.psd");
    let missing_render_path = live_cache_dir.join("renders/live__body.png");

    let stale_zip_path = cache_dir.join("assets/stale.zip");
    create_file(&stale_zip_path);
    let stale_cache_key = zip_cache_key_for_test(&stale_zip_path).unwrap();
    let mut stale_source = zip_source_stamp_for_test(&stale_zip_path).unwrap();
    stale_source.modified_unix_nanos = Some(
        stale_source
            .modified_unix_nanos
            .unwrap_or_default()
            .saturating_add(1),
    );
    let stale_cache_dir = cache_dir.join(&stale_cache_key);
    let stale_psd_path = stale_cache_dir.join("extracted/stale/body.psd");

    write_snapshot_meta(
        &live_cache_dir.join("psd-meta.json"),
        &live_zip_path,
        &live_cache_key,
        &live_source,
        vec![sample_psd_entry(&live_psd_path, Some(missing_render_path))],
    );
    write_snapshot_meta(
        &stale_cache_dir.join("psd-meta.json"),
        &stale_zip_path,
        &stale_cache_key,
        &stale_source,
        vec![sample_psd_entry(&stale_psd_path, None)],
    );

    let core = Core::new(CoreConfig {
        cache_dir: cache_dir.clone(),
    });
    let zip_entries = core.load_cached_zip_entries_snapshot().unwrap();

    assert_eq!(zip_entries.len(), 1);
    assert_eq!(zip_entries[0].zip_path, live_zip_path);
    assert_eq!(zip_entries[0].zip_cache_key, live_cache_key);
    assert_eq!(zip_entries[0].psds.len(), 1);
    assert_eq!(
        zip_entries[0].psds[0].rendered_png_path,
        Some(live_cache_dir.join("renders/live__body.png"))
    );

    let _ = fs::remove_dir_all(&cache_dir);
}

#[test]
fn load_zip_entry_uses_filename_and_timestamp_cache_key() {
    let cache_dir = workspace_cache_root().join("test-load-zip-entry-source-stamp");
    let _ = fs::remove_dir_all(&cache_dir);

    let zip_path = cache_dir.join("assets/live.zip");
    create_empty_zip(&zip_path);

    let core = Core::new(CoreConfig {
        cache_dir: cache_dir.clone(),
    });

    let entry = core.load_zip_entry(&zip_path).unwrap();

    assert!(entry.zip_cache_key.starts_with("live.zip__mtime_"));
    assert_eq!(entry.cache_dir, cache_dir.join(&entry.zip_cache_key));
    assert_ne!(
        entry.zip_cache_key.len(),
        64,
        "cache key should not be a content hash"
    );

    let _ = fs::remove_dir_all(&cache_dir);
}

#[test]
fn load_zip_entry_reuses_memory_cache_until_source_metadata_changes() {
    let cache_dir = workspace_cache_root().join("test-load-zip-entry-memory-cache");
    let _ = fs::remove_dir_all(&cache_dir);

    let zip_path = cache_dir.join("assets/live.zip");
    create_empty_zip(&zip_path);

    let core = Core::new(CoreConfig {
        cache_dir: cache_dir.clone(),
    });

    let first_entry = core.load_zip_entry(&zip_path).unwrap();
    let second_entry = core.load_zip_entry(&zip_path).unwrap();

    assert_eq!(second_entry.zip_path, first_entry.zip_path);
    assert_eq!(second_entry.zip_cache_key, first_entry.zip_cache_key);
    assert_eq!(second_entry.cache_dir, first_entry.cache_dir);

    rewrite_zip_until_cache_key_changes(&zip_path, &first_entry.zip_cache_key);
    let third_entry = core.load_zip_entry(&zip_path).unwrap();

    assert_eq!(third_entry.zip_path, first_entry.zip_path);
    assert_ne!(third_entry.zip_cache_key, first_entry.zip_cache_key);
    assert_ne!(third_entry.cache_dir, first_entry.cache_dir);

    let _ = fs::remove_dir_all(&cache_dir);
}

#[test]
fn load_zip_entry_reuses_persisted_meta_without_scanning_extracted_psds() {
    let cache_dir = workspace_cache_root().join("test-load-zip-entry-no-extracted-scan");
    let _ = fs::remove_dir_all(&cache_dir);

    let zip_path = cache_dir.join("assets/live.zip");
    create_zip_with_file(&zip_path, "demo/body.psd", b"not a real psd");

    let core = Core::new(CoreConfig {
        cache_dir: cache_dir.clone(),
    });
    let first_entry = core.load_zip_entry(&zip_path).unwrap();
    assert_eq!(first_entry.psds.len(), 1);

    fs::remove_dir_all(&first_entry.extracted_dir).unwrap();
    let fresh_core = Core::new(CoreConfig {
        cache_dir: cache_dir.clone(),
    });
    let second_entry = fresh_core.load_zip_entry(&zip_path).unwrap();

    assert_eq!(second_entry.zip_cache_key, first_entry.zip_cache_key);
    assert_eq!(second_entry.psds.len(), 1);
    assert!(
        !second_entry.extracted_dir.exists(),
        "cache hit should not re-extract or scan PSD files"
    );

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
    zip_cache_key: &str,
    source: &ZipSourceStamp,
    psds: Vec<PsdEntry>,
) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }

    let body = json!({
        "version": 4,
        "zip_path": zip_path,
        "zip_cache_key": zip_cache_key,
        "source": source,
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

fn create_empty_zip(path: &std::path::Path) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }

    let file = File::create(path).unwrap();
    ZipWriter::new(file).finish().unwrap();
}

fn create_zip_with_file(path: &std::path::Path, name: &str, contents: &[u8]) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }

    let file = File::create(path).unwrap();
    let mut zip = ZipWriter::new(file);
    zip.start_file(name, zip::write::SimpleFileOptions::default())
        .unwrap();
    std::io::Write::write_all(&mut zip, contents).unwrap();
    zip.finish().unwrap();
}

fn rewrite_zip_until_cache_key_changes(path: &std::path::Path, previous_key: &str) {
    for attempt in 0..20 {
        std::thread::sleep(Duration::from_millis(25));
        let contents = format!("changed-{attempt}");
        create_zip_with_file(path, "note.txt", contents.as_bytes());
        if zip_cache_key_for_test(path).unwrap() != previous_key {
            return;
        }
    }

    panic!("zip mtime did not change after rewrites");
}
