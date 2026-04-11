use ratatui::layout::Rect;

use crate::ui::help_overlay_area;

#[test]
fn help_overlay_area_is_centered_and_sized_to_content() {
    let area = Rect::new(0, 0, 100, 40);

    let overlay = help_overlay_area(area);

    assert!(overlay.width < area.width);
    assert!(overlay.height < area.height);
    assert_eq!(overlay.x, (area.width - overlay.width) / 2);
    assert_eq!(overlay.y, (area.height - overlay.height) / 2);
}

#[test]
fn help_overlay_area_is_clamped_for_small_terminals() {
    let area = Rect::new(5, 7, 20, 8);

    let overlay = help_overlay_area(area);

    assert!(overlay.width <= area.width);
    assert!(overlay.height <= area.height);
    assert!(overlay.x >= area.x);
    assert!(overlay.y >= area.y);
}
