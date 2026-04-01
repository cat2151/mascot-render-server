use std::path::Path;

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
        if let Some(index) = sanitized.iter().position(|saved: &FavoriteEnsembleEntry| {
            favorite_identity(saved) == favorite_identity(&favorite)
        }) {
            sanitized[index] = favorite;
        } else {
            sanitized.push(favorite);
        }
    }
    sanitized
}

fn favorite_identity(favorite: &FavoriteEnsembleEntry) -> (&Path, &Path, Vec<(usize, bool)>) {
    (
        favorite.zip_path.as_path(),
        favorite.psd_path_in_zip.as_path(),
        favorite
            .visibility_overrides
            .iter()
            .map(|layer| (layer.layer_index, layer.visible))
            .collect(),
    )
}

fn sanitize_scale(scale: Option<f32>) -> Option<f32> {
    scale.filter(|value| value.is_finite() && *value > 0.0)
}

fn sanitize_position(position: Option<[f32; 2]>) -> Option<[f32; 2]> {
    position.filter(|[x, y]| x.is_finite() && y.is_finite())
}
