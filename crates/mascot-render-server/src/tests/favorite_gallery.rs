use crate::favorite_gallery::pack_positions_from_right;

#[test]
fn favorite_gallery_packs_entries_from_right_edge_without_horizontal_gaps() {
    let positions = pack_positions_from_right(&[[80.0, 120.0], [40.0, 60.0], [30.0, 90.0]]);

    assert_eq!(positions.len(), 3);
    assert_eq!(positions[0], [70.0, 0.0], "first favorite should sit at the right edge");
    assert_eq!(positions[1], [30.0, 60.0], "second favorite should continue to the left");
    assert_eq!(positions[2], [0.0, 30.0], "later favorites should keep filling leftward");
}
