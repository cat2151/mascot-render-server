use std::path::PathBuf;

use mascot_render_core::{PsdEntry, ZipEntry};

use crate::psd_file_name_catalog::PsdFileNameCatalog;

#[test]
fn startup_fixed_catalog_returns_sorted_unique_usable_psd_file_names() {
    let catalog = PsdFileNameCatalog::from_entries_for_test(vec![
        zip_entry(vec![
            psd_entry("face.psd", Some("cache/face.png")),
            psd_entry("body.psd", Some("cache/body.png")),
            psd_entry("broken.psd", None),
        ]),
        zip_entry(vec![
            psd_entry("body.psd", Some("cache/body-alt.png")),
            psd_entry("", Some("cache/unnamed.png")),
            psd_entry("arm.psd", Some("cache/arm.png")),
        ]),
    ]);

    assert_eq!(
        catalog.snapshot(),
        vec![
            "arm.psd".to_string(),
            "body.psd".to_string(),
            "face.psd".to_string(),
        ]
    );
}

#[test]
fn startup_fixed_catalog_requires_rebuild_to_observe_new_entries() {
    let startup_catalog =
        PsdFileNameCatalog::from_entries_for_test(vec![zip_entry(vec![psd_entry(
            "body.psd",
            Some("cache/body.png"),
        )])]);
    let rebuilt_catalog = PsdFileNameCatalog::from_entries_for_test(vec![zip_entry(vec![
        psd_entry("body.psd", Some("cache/body.png")),
        psd_entry("face.psd", Some("cache/face.png")),
    ])]);

    assert_eq!(startup_catalog.snapshot(), vec!["body.psd".to_string()]);
    assert_eq!(
        rebuilt_catalog.snapshot(),
        vec!["body.psd".to_string(), "face.psd".to_string()]
    );
}

fn zip_entry(psds: Vec<PsdEntry>) -> ZipEntry {
    ZipEntry {
        psds,
        ..ZipEntry::default()
    }
}

fn psd_entry(file_name: &str, rendered_png_path: Option<&str>) -> PsdEntry {
    PsdEntry {
        file_name: file_name.to_string(),
        rendered_png_path: rendered_png_path.map(PathBuf::from),
        ..PsdEntry::default()
    }
}
