use std::path::PathBuf;

use mascot_render_core::{
    LayerDescriptor, LayerKind, PsdEntry, PsdLoadProgress, ZipEntry, ZipLoadEvent, ZipLoadProgress,
};

use crate::app::App;

#[test]
fn pending_psds_are_selectable_during_startup() {
    let mut app = App::loading(None);
    let zip = sample_zip_progress();
    app.apply_startup_loader_event(ZipLoadEvent::ZipStarted(zip.clone()))
        .unwrap();
    app.apply_startup_loader_event(ZipLoadEvent::PsdDiscovered(sample_psd_progress(
        zip.clone(),
        "body.psd",
    )))
    .unwrap();
    app.apply_startup_loader_event(ZipLoadEvent::PsdDiscovered(sample_psd_progress(
        zip, "face.psd",
    )))
    .unwrap();

    assert_eq!(app.selected_psd_entry().unwrap().file_name, "body.psd");
    assert!(app.selected_psd_is_pending());

    app.select_next().unwrap();

    assert_eq!(app.selected_psd_entry().unwrap().file_name, "face.psd");
    assert!(app.selected_psd_is_pending());
}

#[test]
fn selected_pending_psd_refreshes_when_ready() {
    let mut app = App::loading(None);
    let zip = sample_zip_progress();
    let psd_progress = sample_psd_progress(zip.clone(), "body.psd");
    app.apply_startup_loader_event(ZipLoadEvent::ZipStarted(zip.clone()))
        .unwrap();
    app.apply_startup_loader_event(ZipLoadEvent::PsdDiscovered(psd_progress.clone()))
        .unwrap();

    let psd = ready_psd(&psd_progress);
    let needs_sync = app
        .apply_startup_loader_event(ZipLoadEvent::PsdReady(psd_progress, Box::new(psd.clone())))
        .unwrap();

    assert!(needs_sync);
    assert!(!app.selected_psd_is_pending());
    assert_eq!(app.selected_layer_rows().len(), 1);
    assert_eq!(app.selected_layer_rows()[0].name, "Face");

    app.apply_startup_loader_event(ZipLoadEvent::ZipReady(ready_zip(&zip, vec![psd])))
        .unwrap();
    assert_eq!(app.selected_layer_rows().len(), 1);
}

fn sample_zip_progress() -> ZipLoadProgress {
    ZipLoadProgress {
        zip_path: PathBuf::from("assets/zip/live.zip"),
        zip_cache_key: "live.zip__mtime_1".to_string(),
        cache_dir: PathBuf::from("cache/live.zip__mtime_1"),
        extracted_dir: PathBuf::from("cache/live.zip__mtime_1/extracted"),
        psd_meta_path: PathBuf::from("cache/live.zip__mtime_1/psd-meta.json"),
    }
}

fn sample_psd_progress(zip: ZipLoadProgress, file_name: &str) -> PsdLoadProgress {
    PsdLoadProgress {
        psd_path: zip.extracted_dir.join(file_name),
        file_name: file_name.to_string(),
        zip,
    }
}

fn ready_psd(progress: &PsdLoadProgress) -> PsdEntry {
    PsdEntry {
        path: progress.psd_path.clone(),
        file_name: progress.file_name.clone(),
        metadata: "100x100 4ch depth 8".to_string(),
        layer_descriptors: vec![LayerDescriptor {
            layer_index: 0,
            name: "Face".to_string(),
            kind: LayerKind::Layer,
            default_visible: true,
            effective_visible: true,
            depth: 0,
        }],
        ..PsdEntry::default()
    }
}

fn ready_zip(zip: &ZipLoadProgress, psds: Vec<PsdEntry>) -> ZipEntry {
    ZipEntry {
        zip_path: zip.zip_path.clone(),
        zip_cache_key: zip.zip_cache_key.clone(),
        cache_dir: zip.cache_dir.clone(),
        extracted_dir: zip.extracted_dir.clone(),
        psd_meta_path: zip.psd_meta_path.clone(),
        psds,
        updated_at: 1,
    }
}
