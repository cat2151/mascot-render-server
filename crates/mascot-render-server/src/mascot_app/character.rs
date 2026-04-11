use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context, Result};
use mascot_render_core::{Core, PsdEntry, ZipEntry};

#[derive(Debug, Clone)]
pub(super) struct ResolvedCharacterSkin {
    pub(super) character_name: String,
    pub(super) zip_path: PathBuf,
    pub(super) psd_path_in_zip: PathBuf,
    pub(super) png_path: PathBuf,
    pub(super) display_diff_path: Option<PathBuf>,
    pub(super) candidate_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CharacterSkinCandidate {
    zip_path: PathBuf,
    psd_path_in_zip: PathBuf,
    png_path: PathBuf,
}

pub(super) fn resolve_character_skin(
    core: &Core,
    character_name: &str,
) -> Result<ResolvedCharacterSkin> {
    let character_name = normalize_character_name(character_name)?;
    let zip_entries = core.load_cached_zip_entries_snapshot().with_context(|| {
        format!(
            "failed to load cached zip entries while resolving character '{}'",
            character_name
        )
    })?;
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    resolve_character_skin_from_entries(
        &zip_entries,
        character_name,
        seed,
        &core.cache_dir().display().to_string(),
    )
}

fn resolve_character_skin_from_entries(
    zip_entries: &[ZipEntry],
    character_name: &str,
    seed: u64,
    cache_context: &str,
) -> Result<ResolvedCharacterSkin> {
    let candidates = character_skin_candidates(zip_entries, character_name);
    let candidate_count = candidates.len();
    if candidate_count == 0 {
        bail!(
            "no matching character skin candidates were found: requested_character={} candidate_count=0 cache_context={} zip_entry_count={}",
            character_name,
            cache_context,
            zip_entries.len()
        );
    }
    let selected_index = candidate_index_from_seed(candidate_count, seed);
    resolve_candidate(character_name, candidate_count, &candidates[selected_index])
}

pub(crate) fn configured_character_name_for_status(
    zip_path: &Path,
    psd_path_in_zip: &Path,
) -> Option<String> {
    let zip_text = searchable_path_text(zip_path);
    path_search_tokens(psd_path_in_zip)
        .into_iter()
        .filter(|token| zip_text.contains(token))
        .max_by_key(|token| token.chars().count())
}

fn resolve_candidate(
    character_name: &str,
    candidate_count: usize,
    candidate: &CharacterSkinCandidate,
) -> Result<ResolvedCharacterSkin> {
    Ok(ResolvedCharacterSkin {
        character_name: character_name.to_string(),
        zip_path: candidate.zip_path.clone(),
        psd_path_in_zip: candidate.psd_path_in_zip.clone(),
        png_path: candidate.png_path.clone(),
        display_diff_path: None,
        candidate_count,
    })
}

fn normalize_character_name(character_name: &str) -> Result<&str> {
    let character_name = character_name.trim();
    if character_name.is_empty() {
        bail!("character_name must not be empty");
    }
    Ok(character_name)
}

fn character_skin_candidates(
    zip_entries: &[ZipEntry],
    character_name: &str,
) -> Vec<CharacterSkinCandidate> {
    zip_entries
        .iter()
        .flat_map(|zip_entry| {
            zip_entry
                .psds
                .iter()
                .filter_map(move |psd| character_skin_candidate(zip_entry, psd, character_name))
        })
        .collect()
}

fn character_skin_candidate(
    zip_entry: &ZipEntry,
    psd: &PsdEntry,
    character_name: &str,
) -> Option<CharacterSkinCandidate> {
    if !zip_matches_character(&zip_entry.zip_path, character_name) {
        return None;
    }

    let psd_path_in_zip = psd_path_in_zip(zip_entry, psd);
    if !psd_path_in_zip.to_string_lossy().contains(character_name) {
        return None;
    }

    Some(CharacterSkinCandidate {
        zip_path: zip_entry.zip_path.clone(),
        psd_path_in_zip,
        png_path: psd.rendered_png_path.clone()?,
    })
}

fn zip_matches_character(zip_path: &Path, character_name: &str) -> bool {
    zip_path.to_string_lossy().contains(character_name)
        || zip_path
            .file_name()
            .is_some_and(|file_name| file_name.to_string_lossy().contains(character_name))
}

fn psd_path_in_zip(zip_entry: &ZipEntry, psd: &PsdEntry) -> PathBuf {
    psd.path
        .strip_prefix(&zip_entry.extracted_dir)
        .map(Path::to_path_buf)
        .unwrap_or_else(|_| psd.path.clone())
}

fn candidate_index_from_seed(candidate_count: usize, seed: u64) -> usize {
    let mut value = seed.max(1);
    value ^= value >> 12;
    value ^= value << 25;
    value ^= value >> 27;
    (value.wrapping_mul(2_685_821_657_736_338_717) as usize) % candidate_count
}

fn searchable_path_text(path: &Path) -> String {
    let mut text = path.to_string_lossy().to_string();
    if let Some(file_name) = path.file_name() {
        text.push(' ');
        text.push_str(&file_name.to_string_lossy());
    }
    text
}

fn path_search_tokens(path: &Path) -> Vec<String> {
    path.components()
        .filter_map(|component| {
            let token = component.as_os_str().to_string_lossy();
            let token = token.trim();
            (!token.is_empty()).then(|| trim_extension(token).to_string())
        })
        .filter(|token| !token.is_empty())
        .collect()
}

fn trim_extension(token: &str) -> &str {
    token.rsplit_once('.').map_or(token, |(stem, _)| stem)
}

#[cfg(test)]
pub(crate) fn character_skin_candidates_for_test(
    zip_entries: &[ZipEntry],
    character_name: &str,
) -> Vec<(PathBuf, PathBuf, PathBuf)> {
    character_skin_candidates(zip_entries, character_name)
        .into_iter()
        .map(|candidate| {
            (
                candidate.zip_path,
                candidate.psd_path_in_zip,
                candidate.png_path,
            )
        })
        .collect()
}

#[cfg(test)]
pub(crate) fn candidate_index_from_seed_for_test(candidate_count: usize, seed: u64) -> usize {
    candidate_index_from_seed(candidate_count, seed)
}

#[cfg(test)]
pub(crate) fn resolve_character_skin_from_entries_for_test(
    zip_entries: &[ZipEntry],
    character_name: &str,
    seed: u64,
) -> Result<(String, PathBuf, PathBuf, PathBuf, usize)> {
    resolve_character_skin_from_entries(zip_entries, character_name, seed, "test-cache").map(
        |resolved| {
            (
                resolved.character_name,
                resolved.zip_path,
                resolved.psd_path_in_zip,
                resolved.png_path,
                resolved.candidate_count,
            )
        },
    )
}
