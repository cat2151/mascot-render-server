use std::fs;
use std::fs::File;
use std::io::Write;

use zip::ZipWriter;

use crate::{workspace_cache_root, Core, CoreConfig, ZipLoadEvent};

#[test]
fn incremental_loader_emits_psd_ready_for_each_psd() {
    let cache_dir = workspace_cache_root().join("test-incremental-loader-events");
    let _ = fs::remove_dir_all(&cache_dir);
    let source_dir = cache_dir.join("assets");
    let zip_path = source_dir.join("live.zip");
    create_zip_with_files(
        &zip_path,
        &[
            ("demo/body.psd", b"not a real psd"),
            ("demo/face.psd", b"also not a psd"),
        ],
    );

    let core = Core::new(CoreConfig {
        cache_dir: cache_dir.join("cache"),
    });
    let mut events = Vec::new();
    let entries = core
        .load_zip_entries_incremental(&[source_dir], |event| events.push(event))
        .unwrap();

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].psds.len(), 2);
    assert_eq!(
        event_count(&events, |event| matches!(
            event,
            ZipLoadEvent::PsdReady(_, _)
        )),
        2
    );
    assert_eq!(
        event_count(&events, |event| matches!(
            event,
            ZipLoadEvent::PsdDiscovered(_)
        )),
        2
    );
    assert!(matches!(events.last(), Some(ZipLoadEvent::Finished(_))));

    let _ = fs::remove_dir_all(&cache_dir);
}

#[test]
fn incremental_loader_writes_meta_only_when_zip_is_ready() {
    let cache_dir = workspace_cache_root().join("test-incremental-loader-meta-timing");
    let _ = fs::remove_dir_all(&cache_dir);
    let source_dir = cache_dir.join("assets");
    let zip_path = source_dir.join("live.zip");
    create_zip_with_files(&zip_path, &[("demo/body.psd", b"not a real psd")]);

    let core = Core::new(CoreConfig {
        cache_dir: cache_dir.join("cache"),
    });
    let mut saw_psd_ready_before_meta = false;
    let mut saw_zip_ready_with_meta = false;
    core.load_zip_entries_incremental(&[source_dir], |event| match event {
        ZipLoadEvent::PsdReady(progress, _) => {
            saw_psd_ready_before_meta = !progress.zip.psd_meta_path.exists();
        }
        ZipLoadEvent::ZipReady(entry) => {
            saw_zip_ready_with_meta = entry.psd_meta_path.exists();
        }
        _ => {}
    })
    .unwrap();

    assert!(saw_psd_ready_before_meta);
    assert!(saw_zip_ready_with_meta);

    let _ = fs::remove_dir_all(&cache_dir);
}

fn event_count(events: &[ZipLoadEvent], predicate: impl Fn(&ZipLoadEvent) -> bool) -> usize {
    events.iter().filter(|event| predicate(event)).count()
}

fn create_zip_with_files(path: &std::path::Path, files: &[(&str, &[u8])]) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }

    let file = File::create(path).unwrap();
    let mut zip = ZipWriter::new(file);
    for (name, contents) in files {
        zip.start_file(*name, zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(contents).unwrap();
    }
    zip.finish().unwrap();
}
