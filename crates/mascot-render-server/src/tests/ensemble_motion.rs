use mascot_render_core::MascotImageData;

use super::*;

#[test]
fn ensemble_plan_targets_include_each_member_source_key() {
    let ctx = egui::Context::default();
    let scene = EnsembleScene::from_loaded(
        &ctx,
        Ensemble {
            canvas_size: [30.0, 20.0],
            members: vec![
                member("a.zip", "a.psd", [0.0, 0.0], [10.0, 20.0]),
                member("b.zip", "b.psd", [10.0, 0.0], [20.0, 20.0]),
            ],
        },
        false,
        Instant::now(),
    );

    let targets = scene.placement_plan_targets(Pos2::new(100.0, 200.0), Vec2::ZERO, 1.0);

    assert_eq!(targets.len(), 2);
    assert_eq!(targets[0].zip_path, PathBuf::from("a.zip"));
    assert_eq!(targets[0].psd_path_in_zip, PathBuf::from("a.psd"));
    assert_eq!(targets[1].zip_path, PathBuf::from("b.zip"));
    assert_eq!(targets[1].psd_path_in_zip, PathBuf::from("b.psd"));
    assert_eq!(targets[1].bottom_right_anchor_offset, [30.0, 20.0]);
}

#[test]
fn ensemble_triggers_mouth_flap_for_named_member_only() {
    let ctx = egui::Context::default();
    let mut first = member("a.zip", "a.psd", [0.0, 0.0], [10.0, 20.0]);
    first.character_name = Some("ずんだもん".to_string());
    let mut second = member("b.zip", "b.psd", [10.0, 0.0], [20.0, 20.0]);
    second.character_name = Some("四国めたん".to_string());
    let mut scene = EnsembleScene::from_loaded(
        &ctx,
        Ensemble {
            canvas_size: [30.0, 20.0],
            members: vec![first, second],
        },
        false,
        Instant::now(),
    );

    assert!(scene.trigger_mouth_flap_for_character(
        "四国めたん",
        Instant::now(),
        Duration::from_secs(1),
        4,
    ));

    assert!(!scene.members[0].motion.is_active());
    assert!(scene.members[1].motion.is_active());
}

#[test]
fn ensemble_member_active_skin_prefers_mouth_flap_over_blink() {
    let ctx = egui::Context::default();
    let mut member = member("a.zip", "a.psd", [0.0, 0.0], [10.0, 20.0]);
    member.closed_image = Some(image("blink-closed", [10.0, 20.0]));
    member.mouth_open_image = Some(image("mouth-open", [10.0, 20.0]));
    member.mouth_closed_image = Some(image("mouth-closed", [10.0, 20.0]));
    let now = Instant::now();
    let mut scene = EnsembleScene::from_loaded(
        &ctx,
        Ensemble {
            canvas_size: [10.0, 20.0],
            members: vec![member],
        },
        false,
        now,
    );

    scene.members[0]
        .motion
        .trigger_mouth_flap(now, Duration::from_secs(1), 4);

    assert_eq!(
        scene.members[0].active_skin(true, now).path.clone(),
        PathBuf::from("mouth-open.png")
    );
    assert_eq!(
        scene.members[0]
            .active_skin(true, now + Duration::from_millis(250))
            .path
            .clone(),
        PathBuf::from("mouth-closed.png")
    );
    assert_eq!(
        scene.members[0]
            .active_skin(true, now + Duration::from_secs(1))
            .path
            .clone(),
        PathBuf::from("blink-closed.png")
    );
}

fn member(
    zip_path: &str,
    psd_path_in_zip: &str,
    canvas_position: [f32; 2],
    base_size: [f32; 2],
) -> EnsembleMember {
    EnsembleMember {
        character_name: None,
        zip_path: PathBuf::from(zip_path),
        psd_path_in_zip: PathBuf::from(psd_path_in_zip),
        image: image(zip_path, base_size),
        closed_image: None,
        mouth_open_image: None,
        mouth_closed_image: None,
        base_size,
        canvas_position,
    }
}

fn image(path: &str, size: [f32; 2]) -> MascotImageData {
    let width = size[0].ceil().max(1.0) as u32;
    let height = size[1].ceil().max(1.0) as u32;
    MascotImageData {
        path: PathBuf::from(path).with_extension("png"),
        width,
        height,
        rgba: vec![255; width as usize * height as usize * 4],
    }
}
