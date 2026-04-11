use std::path::PathBuf;

use mascot_render_core::{PsdEntry, ZipEntry};

use crate::mascot_app::{
    candidate_index_from_seed_for_test, character_skin_candidates_for_test,
    configured_character_name_for_status, resolve_character_skin_from_entries_for_test,
};

#[test]
fn character_candidate_requires_zip_and_psd_to_contain_keyword() {
    let entries = vec![zip_entry(
        "assets/zip/ずんだもん素材.zip",
        "extract/zunda",
        vec![
            psd_entry(
                "extract/zunda/ずんだもん/basic.psd",
                "cache/zunda/basic.png",
            ),
            psd_entry("extract/zunda/あんこもん/basic.psd", "cache/anko/basic.png"),
        ],
    )];

    let candidates = character_skin_candidates_for_test(&entries, "ずんだもん");

    assert_eq!(
        candidates,
        vec![(
            PathBuf::from("assets/zip/ずんだもん素材.zip"),
            PathBuf::from("ずんだもん/basic.psd"),
            PathBuf::from("cache/zunda/basic.png")
        )]
    );
}

#[test]
fn character_candidate_rejects_zip_only_match() {
    let entries = vec![zip_entry(
        "assets/zip/ずんだもん素材.zip",
        "extract/zunda",
        vec![psd_entry(
            "extract/zunda/other/basic.psd",
            "cache/zunda/basic.png",
        )],
    )];

    assert!(character_skin_candidates_for_test(&entries, "ずんだもん").is_empty());
}

#[test]
fn character_candidate_rejects_psd_only_match() {
    let entries = vec![zip_entry(
        "assets/zip/other.zip",
        "extract/other",
        vec![psd_entry(
            "extract/other/ずんだもん/basic.psd",
            "cache/zunda/basic.png",
        )],
    )];

    assert!(character_skin_candidates_for_test(&entries, "ずんだもん").is_empty());
}

#[test]
fn resolving_zero_candidates_returns_contextual_error() {
    let error = resolve_character_skin_from_entries_for_test(&[], "ずんだもん", 1)
        .expect_err("zero candidates should fail");

    let error = error.to_string();
    assert!(error.contains("requested_character=ずんだもん"));
    assert!(error.contains("candidate_count=0"));
    assert!(error.contains("zip_entry_count=0"));
}

#[test]
fn resolving_multiple_candidates_selects_deterministically_for_tests() {
    let entries = vec![
        zip_entry(
            "assets/zip/ずんだもん-a.zip",
            "extract/a",
            vec![psd_entry(
                "extract/a/ずんだもん/body.psd",
                "cache/a/body.png",
            )],
        ),
        zip_entry(
            "assets/zip/ずんだもん-b.zip",
            "extract/b",
            vec![psd_entry(
                "extract/b/ずんだもん/body.psd",
                "cache/b/body.png",
            )],
        ),
    ];
    let seed = 12_345;
    let expected_index = candidate_index_from_seed_for_test(2, seed);

    let resolved = resolve_character_skin_from_entries_for_test(&entries, "ずんだもん", seed)
        .expect("candidate should resolve");

    assert_eq!(resolved.4, 2);
    assert_eq!(
        resolved.3,
        PathBuf::from(format!("cache/{}/body.png", ["a", "b"][expected_index]))
    );
}

#[test]
fn configured_character_name_uses_common_zip_and_psd_token() {
    let character_name = configured_character_name_for_status(
        &PathBuf::from("assets/zip/ずんだもん素材.zip"),
        &PathBuf::from("ずんだもん素材/body.psd"),
    );

    assert_eq!(character_name.as_deref(), Some("ずんだもん素材"));
}

fn zip_entry(zip_path: &str, extracted_dir: &str, psds: Vec<PsdEntry>) -> ZipEntry {
    ZipEntry {
        zip_path: PathBuf::from(zip_path),
        extracted_dir: PathBuf::from(extracted_dir),
        psds,
        ..ZipEntry::default()
    }
}

fn psd_entry(path: &str, rendered_png_path: &str) -> PsdEntry {
    PsdEntry {
        path: PathBuf::from(path),
        rendered_png_path: Some(PathBuf::from(rendered_png_path)),
        ..PsdEntry::default()
    }
}
