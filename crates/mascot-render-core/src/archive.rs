use std::fs::{self, File};
use std::io;
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result};
use encoding_rs::SHIFT_JIS;
use zip::ZipArchive;

use crate::workspace_paths::{local_data_path, workspace_relative_display_path};

pub const ZIP_INPUT_DIRS: [&str; 2] = ["assets/inbox", "assets/zip"];

pub fn existing_zip_sources() -> Vec<PathBuf> {
    ZIP_INPUT_DIRS
        .iter()
        .map(local_data_path)
        .filter(|path| path.is_dir())
        .collect()
}

pub(crate) fn collect_zip_files(zip_sources: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut zip_files = Vec::new();

    for source in zip_sources {
        for entry in fs::read_dir(source)
            .with_context(|| format!("failed to read zip source {}", display_path(source)))?
        {
            let path = entry?.path();

            if is_zip_file(&path) {
                zip_files.push(path);
            }
        }
    }

    zip_files.sort_by_cached_key(|path| display_path(path));
    Ok(zip_files)
}

pub(crate) fn collect_psd_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut psd_files = Vec::new();
    let mut stack = vec![root.to_path_buf()];

    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir)
            .with_context(|| format!("failed to read directory {}", display_path(&dir)))?
        {
            let path = entry?.path();

            if path.is_dir() {
                stack.push(path);
                continue;
            }

            if path
                .extension()
                .is_some_and(|ext| ext.to_string_lossy().eq_ignore_ascii_case("psd"))
            {
                psd_files.push(path);
            }
        }
    }

    psd_files.sort_by_cached_key(|path| display_path(path));
    Ok(psd_files)
}

pub(crate) fn safe_entry_path(raw_name: &[u8]) -> Option<PathBuf> {
    let decoded = decode_zip_path(raw_name);
    let normalized = decoded.replace('\\', "/");
    let mut relative_path = PathBuf::new();

    for component in Path::new(&normalized).components() {
        match component {
            Component::Normal(part) => {
                let sanitized = sanitize_fs_component(part);
                if !sanitized.is_empty() {
                    relative_path.push(sanitized);
                }
            }
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return None,
        }
    }

    if relative_path.as_os_str().is_empty() {
        None
    } else {
        Some(relative_path)
    }
}

pub(crate) fn decode_zip_path(raw_name: &[u8]) -> String {
    let raw_name = raw_name.split(|byte| *byte == 0).next().unwrap_or(raw_name);

    match std::str::from_utf8(raw_name) {
        Ok(decoded) => decoded.to_string(),
        Err(_) => {
            let (decoded, _, _) = SHIFT_JIS.decode(raw_name);
            decoded.into_owned()
        }
    }
}

pub fn display_path(path: &Path) -> String {
    workspace_relative_display_path(path)
}

fn is_zip_file(path: &Path) -> bool {
    path.is_file()
        && path
            .extension()
            .is_some_and(|ext| ext.to_string_lossy().eq_ignore_ascii_case("zip"))
}

pub(crate) fn extract_zip_to_dir(zip_path: &Path, target_dir: &Path) -> Result<()> {
    fs::create_dir_all(target_dir)
        .with_context(|| format!("failed to create {}", display_path(target_dir)))?;

    let file = File::open(zip_path)
        .with_context(|| format!("failed to open {}", display_path(zip_path)))?;
    let mut archive = ZipArchive::new(file)
        .with_context(|| format!("failed to read zip {}", display_path(zip_path)))?;

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).with_context(|| {
            format!(
                "failed to read entry {index} from {}",
                display_path(zip_path)
            )
        })?;

        let Some(relative_path) = safe_entry_path(entry.name_raw()) else {
            continue;
        };

        let output_path = target_dir.join(relative_path);

        if entry.is_dir() {
            fs::create_dir_all(&output_path)
                .with_context(|| format!("failed to create {}", display_path(&output_path)))?;
            continue;
        }

        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", display_path(parent)))?;
        }

        let mut output = File::create(&output_path)
            .with_context(|| format!("failed to create {}", display_path(&output_path)))?;
        io::copy(&mut entry, &mut output)
            .with_context(|| format!("failed to write {}", display_path(&output_path)))?;
    }

    Ok(())
}

fn sanitize_fs_component(component: &std::ffi::OsStr) -> String {
    let sanitized = component
        .to_string_lossy()
        .chars()
        .map(|ch| match ch {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            _ => ch,
        })
        .collect::<String>()
        .trim_matches([' ', '.'])
        .to_string();

    if sanitized.is_empty() {
        "_".to_string()
    } else {
        sanitized
    }
}
