use super::{pack_positions_from_right, EnsembleLayoutEntry};

const POSITION_EPSILON: f32 = 0.001;

pub(super) fn normalize_vpt_positions(layout_entries: &mut [EnsembleLayoutEntry]) -> Vec<usize> {
    let positions = pack_positions_from_right(layout_entries);
    let mut updated_indices = Vec::new();
    for (index, (entry, position)) in layout_entries.iter_mut().zip(positions).enumerate() {
        if position_needs_update(entry.position, position) {
            updated_indices.push(index);
        }
        entry.position = Some(position);
    }
    updated_indices
}

fn position_needs_update(current: Option<[f32; 2]>, expected: [f32; 2]) -> bool {
    current.is_none_or(|current| {
        (current[0] - expected[0]).abs() > POSITION_EPSILON
            || (current[1] - expected[1]).abs() > POSITION_EPSILON
    })
}

#[cfg(test)]
pub(crate) fn normalize_vpt_positions_for_test(
    layout_entries: &mut [EnsembleLayoutEntry],
) -> Vec<usize> {
    normalize_vpt_positions(layout_entries)
}
