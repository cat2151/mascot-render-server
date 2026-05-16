use super::EnsembleEntry;

pub(super) fn sanitize_ensemble_entries(entries: Vec<EnsembleEntry>) -> Vec<EnsembleEntry> {
    let mut sanitized = Vec::new();
    for mut entry in entries {
        if entry.zip_path.as_os_str().is_empty() || entry.psd_path_in_zip.as_os_str().is_empty() {
            continue;
        }
        if entry.psd_file_name.is_empty() {
            entry.psd_file_name = entry
                .psd_path_in_zip
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| entry.psd_path_in_zip.display().to_string());
        }
        entry.character_name = sanitize_character_name(entry.character_name);
        entry.mascot_scale = sanitize_scale(entry.mascot_scale);
        entry.favorite_ensemble_position = sanitize_position(entry.favorite_ensemble_position);
        if let Some(index) = sanitized
            .iter()
            .position(|saved: &EnsembleEntry| same_ensemble_entry_identity(saved, &entry))
        {
            sanitized[index] = entry;
        } else {
            sanitized.push(entry);
        }
    }
    sanitized
}

fn same_ensemble_entry_identity(left: &EnsembleEntry, right: &EnsembleEntry) -> bool {
    left.zip_path == right.zip_path
        && left.psd_path_in_zip == right.psd_path_in_zip
        && left.visibility_overrides.len() == right.visibility_overrides.len()
        && left
            .visibility_overrides
            .iter()
            .zip(&right.visibility_overrides)
            .all(|(left, right)| {
                left.layer_index == right.layer_index && left.visible == right.visible
            })
}

fn sanitize_scale(scale: Option<f32>) -> Option<f32> {
    scale.filter(|value| value.is_finite() && *value > 0.0)
}

fn sanitize_character_name(character_name: Option<String>) -> Option<String> {
    character_name.and_then(|name| {
        let trimmed = name.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    })
}

fn sanitize_position(position: Option<[f32; 2]>) -> Option<[f32; 2]> {
    position.filter(|[x, y]| x.is_finite() && y.is_finite())
}

#[cfg(test)]
pub(crate) fn sanitize_ensemble_entries_for_test(
    entries: Vec<EnsembleEntry>,
) -> Vec<EnsembleEntry> {
    sanitize_ensemble_entries(entries)
}
