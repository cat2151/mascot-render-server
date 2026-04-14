use std::path::PathBuf;

use mascot_render_core::{local_data_root, PsdEntry, ZipEntry};
use mascot_render_protocol::MotionTimelineKind;

use crate::actions::{
    cached_psd_candidates, select_random_character_candidate, shake_timeline_request,
    CachedPsdSource, TestPostAction,
};

#[test]
fn local_data_root_is_redirected_to_temp_directory_for_tests() {
    assert!(
        local_data_root().starts_with(std::env::temp_dir()),
        "test local data root should live under temp dir: {}",
        local_data_root().display()
    );
}

#[test]
fn test_post_action_labels_match_key_descriptions() {
    assert_eq!(TestPostAction::Show.label(), "show");
    assert_eq!(TestPostAction::Hide.label(), "hide");
    assert_eq!(
        TestPostAction::change_character_label(),
        "change-character configured_character_name"
    );
    assert_eq!(
        TestPostAction::random_character_label(),
        "change-character random cached PSD"
    );
    assert_eq!(TestPostAction::ShakeTimeline.label(), "timeline shake");
    assert_eq!(
        TestPostAction::MouthFlapTimeline.label(),
        "timeline mouth-flap"
    );
}

#[test]
fn cached_psd_candidates_generate_character_name_matching_zip_and_psd_path() {
    let entries = vec![zip_entry(
        "assets/zip/ずんだもん立ち絵素材V3.2.zip",
        "extract/zunda",
        vec![psd_entry(
            "extract/zunda/ずんだもん立ち絵素材V3.2/ずんだもん立ち絵素材V3.2_基本版.psd",
            "cache/zunda/basic.png",
        )],
    )];

    let candidates = cached_psd_candidates(&entries);

    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].character_name, "ずんだもん立ち絵素材V3.2");
    assert_eq!(
        candidates[0].source.psd_path_in_zip,
        PathBuf::from("ずんだもん立ち絵素材V3.2/ずんだもん立ち絵素材V3.2_基本版.psd")
    );
}

#[test]
fn random_character_selection_excludes_current_source_when_possible() {
    let current = CachedPsdSource {
        zip_path: PathBuf::from("assets/zip/a.zip"),
        psd_path_in_zip: PathBuf::from("a/body.psd"),
        png_path: PathBuf::from("cache/a/body.png"),
    };
    let candidates = vec![
        selection("a", current.clone()),
        selection(
            "b",
            CachedPsdSource {
                zip_path: PathBuf::from("assets/zip/b.zip"),
                psd_path_in_zip: PathBuf::from("b/body.psd"),
                png_path: PathBuf::from("cache/b/body.png"),
            },
        ),
    ];

    let selected = select_random_character_candidate(candidates, Some(&current), 1)
        .expect("random character should select a non-current candidate");

    assert_eq!(selected.character_name, "b");
    assert_eq!(selected.candidate_count, 2);
    assert_eq!(selected.selectable_count, 1);
}

fn selection(
    character_name: &str,
    source: CachedPsdSource,
) -> crate::actions::RandomCharacterSelection {
    crate::actions::RandomCharacterSelection {
        character_name: character_name.to_string(),
        source,
        candidate_count: 0,
        selectable_count: 0,
    }
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

#[test]
fn shake_timeline_request_uses_single_test_step() {
    let request = shake_timeline_request();

    assert_eq!(request.steps.len(), 1);
    assert_eq!(request.steps[0].kind, MotionTimelineKind::Shake);
    assert_eq!(request.steps[0].duration_ms, 900);
    assert_eq!(request.steps[0].fps, 20);
}
