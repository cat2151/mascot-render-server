use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use super::sanitize_favorites;
use super::FavoriteEnsembleEntry;

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default, deny_unknown_fields)]
struct FavoritesFile {
    favorites: Vec<FavoriteEnsembleEntry>,
}

pub(crate) fn load_favorites(path: &Path) -> Result<Vec<FavoriteEnsembleEntry>> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let bytes = fs::read_to_string(path).with_context(|| {
        format!(
            "failed to read favorite ensemble entries {}",
            path.display()
        )
    })?;
    match toml::from_str::<FavoritesFile>(&bytes) {
        Ok(file) => Ok(sanitize_favorites(file.favorites)),
        Err(error) => {
            eprintln!(
                "favorite ensemble ignored invalid favorites cache {}: {error:#}",
                path.display()
            );
            Ok(Vec::new())
        }
    }
}

pub(crate) fn patch_favorite_ensemble_positions(
    path: &Path,
    updates: &[FavoriteEnsembleEntry],
) -> Result<()> {
    let raw = fs::read_to_string(path).with_context(|| {
        format!(
            "failed to read favorite ensemble entries {}",
            path.display()
        )
    })?;
    let patched = patch_favorite_ensemble_positions_toml(&raw, updates)?;
    fs::write(path, patched).with_context(|| {
        format!(
            "failed to write favorite ensemble entries {}",
            path.display()
        )
    })
}

pub(crate) fn patch_favorite_ensemble_positions_toml(
    raw: &str,
    updates: &[FavoriteEnsembleEntry],
) -> Result<String> {
    let mut value = toml::from_str::<toml::Value>(raw)
        .context("failed to parse favorites TOML while patching ensemble positions")?;
    let favorites = value
        .get_mut("favorites")
        .and_then(toml::Value::as_array_mut)
        .context("favorites should remain an array while patching ensemble positions")?;

    for update in updates {
        let Some(position) = update.favorite_ensemble_position else {
            continue;
        };
        let Some(entry) = favorites
            .iter_mut()
            .find(|entry| favorite_entry_matches_update(entry, update))
        else {
            continue;
        };
        // Only backfill entries missing favorite_ensemble_position, preserving
        // user-adjusted coordinates.
        if entry
            .get("favorite_ensemble_position")
            .and_then(toml::Value::as_array)
            .is_some()
        {
            continue;
        }

        let Some(table) = entry.as_table_mut() else {
            continue;
        };
        table.insert(
            "favorite_ensemble_position".to_string(),
            toml::Value::Array(vec![position[0].into(), position[1].into()]),
        );
    }

    toml::to_string_pretty(&value).context("failed to serialize patched favorites TOML")
}

fn favorite_entry_matches_update(value: &toml::Value, update: &FavoriteEnsembleEntry) -> bool {
    let Some(table) = value.as_table() else {
        return false;
    };
    let zip_path = table
        .get("zip_path")
        .and_then(toml::Value::as_str)
        .map(Path::new);
    let psd_path_in_zip = table
        .get("psd_path_in_zip")
        .and_then(toml::Value::as_str)
        .map(Path::new);
    zip_path == Some(update.zip_path.as_path())
        && psd_path_in_zip == Some(update.psd_path_in_zip.as_path())
        && table_visibility_overrides(table.get("visibility_overrides"))
            == update
                .visibility_overrides
                .iter()
                .map(|layer| (layer.layer_index, layer.visible))
                .collect::<Vec<_>>()
}

fn table_visibility_overrides(value: Option<&toml::Value>) -> Vec<(usize, bool)> {
    value
        .and_then(toml::Value::as_array)
        .map(|layers| {
            layers
                .iter()
                .filter_map(|layer| {
                    let table = layer.as_table()?;
                    let layer_index_value =
                        table.get("layer_index").and_then(toml::Value::as_integer)?;
                    let layer_index = match layer_index_value.try_into() {
                        Ok(layer_index) => layer_index,
                        Err(_) => {
                            eprintln!(
                                "favorite ensemble ignored invalid layer_index {} while matching visibility_overrides",
                                layer_index_value
                            );
                            return None;
                        }
                    };
                    let visible = table.get("visible").and_then(toml::Value::as_bool)?;
                    Some((layer_index, visible))
                })
                .collect()
        })
        .unwrap_or_default()
}
