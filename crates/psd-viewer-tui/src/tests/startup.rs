use crate::app::App;

#[test]
fn loading_placeholder_does_not_overwrite_restored_layer_cursor() {
    let previous = App::loading(None);
    let mut loaded = App::loading(None);
    loaded.selected_layer_index = 7;

    loaded
        .adopt_runtime_state_from(&previous)
        .expect("loading placeholder should not clear restored cursor");

    assert_eq!(loaded.selected_layer_index, 7);
}
