use super::FavoriteEnsembleEntry;

pub(super) fn sanitize_favorites(
    favorites: Vec<FavoriteEnsembleEntry>,
) -> Vec<FavoriteEnsembleEntry> {
    let mut sanitized = Vec::new();
    for mut favorite in favorites {
        if favorite.zip_path.as_os_str().is_empty()
            || favorite.psd_path_in_zip.as_os_str().is_empty()
        {
            continue;
        }
        if favorite.psd_file_name.is_empty() {
            favorite.psd_file_name = favorite
                .psd_path_in_zip
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| favorite.psd_path_in_zip.display().to_string());
        }
        favorite.mascot_scale = sanitize_scale(favorite.mascot_scale);
        favorite.favorite_ensemble_position =
            sanitize_position(favorite.favorite_ensemble_position);
        if let Some(index) = sanitized
            .iter()
            .position(|saved: &FavoriteEnsembleEntry| same_favorite_identity(saved, &favorite))
        {
            sanitized[index] = favorite;
        } else {
            sanitized.push(favorite);
        }
    }
    sanitized
}

fn same_favorite_identity(left: &FavoriteEnsembleEntry, right: &FavoriteEnsembleEntry) -> bool {
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

fn sanitize_position(position: Option<[f32; 2]>) -> Option<[f32; 2]> {
    position.filter(|[x, y]| x.is_finite() && y.is_finite())
}

#[cfg(test)]
pub(crate) fn sanitize_favorites_for_test(
    favorites: Vec<FavoriteEnsembleEntry>,
) -> Vec<FavoriteEnsembleEntry> {
    sanitize_favorites(favorites)
}
