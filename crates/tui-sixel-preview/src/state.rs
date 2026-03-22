use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{Context, Result};
use image::ImageReader;
use ratatui::layout::Rect;
use ratatui_image::picker::Picker;
use ratatui_image::protocol::sixel::Sixel;
use ratatui_image::protocol::{Protocol, StatefulProtocol, StatefulProtocolType};
use ratatui_image::{Resize, ResizeEncodeRender};

use crate::logging::append_sixel_preview_timing_log;

#[derive(Debug)]
struct PendingSixelTiming {
    png_path: PathBuf,
    image_pixels: (u32, u32),
    png_load_ms: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct SixelCacheKey {
    png_path: PathBuf,
    area_cells: (u16, u16),
    font_size: (u16, u16),
}

#[derive(Debug, Clone)]
struct CachedSixelRender {
    data: String,
    area: Rect,
    is_tmux: bool,
}

#[derive(Default)]
pub struct PreviewState {
    current_path: Option<PathBuf>,
    pending_path: Option<PathBuf>,
    image_state: Option<StatefulProtocol>,
    active_sixel_protocol: Option<Protocol>,
    sixel_cache: HashMap<SixelCacheKey, CachedSixelRender>,
    font_size: (u16, u16),
    pending_sixel_timing: Option<PendingSixelTiming>,
    status: String,
}

impl PreviewState {
    pub fn new() -> Self {
        Self {
            current_path: None,
            pending_path: None,
            image_state: None,
            active_sixel_protocol: None,
            sixel_cache: HashMap::new(),
            font_size: (0, 0),
            pending_sixel_timing: None,
            status: "Preview disabled".to_string(),
        }
    }

    pub fn request_sync(&mut self, png_path: Option<&Path>) {
        if same_path(self.target_path(), png_path) {
            return;
        }

        self.image_state = None;
        self.active_sixel_protocol = None;
        self.pending_sixel_timing = None;

        let Some(path) = png_path else {
            self.current_path = None;
            self.pending_path = None;
            self.status = "No cached PNG preview.".to_string();
            return;
        };

        self.pending_path = Some(path.to_path_buf());
        self.status = format!(
            "Loading preview from cache...\n{}",
            path.file_name()
                .unwrap_or(path.as_os_str())
                .to_string_lossy()
        );
    }

    pub fn sync_pending(&mut self, picker: &mut Picker) -> Result<bool> {
        let Some(path) = self.pending_path.take() else {
            return Ok(false);
        };
        self.current_path = Some(path.clone());

        let png_load_started_at = Instant::now();
        let dyn_img = ImageReader::open(&path)
            .with_context(|| format!("failed to open preview {}", path.display()))?
            .decode()
            .with_context(|| format!("failed to decode preview {}", path.display()))?;
        let png_load_ms = png_load_started_at.elapsed().as_secs_f64() * 1000.0;
        let image_pixels = (dyn_img.width(), dyn_img.height());

        self.font_size = picker.font_size();
        self.image_state = Some(picker.new_resize_protocol(dyn_img));
        self.active_sixel_protocol = None;
        self.pending_sixel_timing = Some(PendingSixelTiming {
            png_path: path.clone(),
            image_pixels,
            png_load_ms,
        });
        self.status = format!(
            "Preview: {}",
            path.file_name()
                .unwrap_or(path.as_os_str())
                .to_string_lossy()
        );
        Ok(true)
    }

    pub fn image_state_mut(&mut self) -> Option<&mut StatefulProtocol> {
        self.image_state.as_mut()
    }

    pub fn active_sixel_protocol(&self) -> Option<&Protocol> {
        self.active_sixel_protocol.as_ref()
    }

    pub fn status(&self) -> &str {
        &self.status
    }

    pub fn is_loading(&self) -> bool {
        self.pending_path.is_some()
    }

    pub fn has_sixel_cache_for_path(&self, png_path: Option<&Path>) -> bool {
        let Some(png_path) = png_path else {
            return false;
        };

        self.sixel_cache.keys().any(|key| key.png_path == *png_path)
    }

    pub fn uses_compact_loading_overlay(&self) -> bool {
        self.has_sixel_cache_for_path(self.pending_path.as_deref())
    }

    pub fn loading_overlay_message(&self) -> &str {
        if self.uses_compact_loading_overlay() {
            "Loading preview..."
        } else {
            self.status()
        }
    }

    pub fn prepare_sixel_render(&mut self, area: Rect) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        self.active_sixel_protocol = None;

        let Some(image_state) = self.image_state.as_mut() else {
            return;
        };
        if !matches!(image_state.protocol_type(), StatefulProtocolType::Sixel(_)) {
            self.active_sixel_protocol = None;
            self.pending_sixel_timing = None;
            return;
        }

        let resize = Resize::Fit(None);
        let target_area = image_state.size_for(resize.clone(), area);
        let Some(current_path) = self.current_path.clone() else {
            return;
        };
        let cache_key = SixelCacheKey {
            png_path: current_path,
            area_cells: (target_area.width, target_area.height),
            font_size: self.font_size,
        };

        if let Some(cached) = self.sixel_cache.get(&cache_key) {
            self.active_sixel_protocol = Some(Protocol::Sixel(Sixel {
                data: cached.data.clone(),
                area: cached.area,
                is_tmux: cached.is_tmux,
            }));
            self.pending_sixel_timing = None;
            return;
        }

        let Some(target_area) = image_state.needs_resize(&resize, area) else {
            return;
        };

        let encode_started_at = Instant::now();
        image_state.resize_encode(&resize, target_area);
        let sixel_encode_ms = encode_started_at.elapsed().as_secs_f64() * 1000.0;

        let Some(timing) = self.pending_sixel_timing.take() else {
            return;
        };
        let StatefulProtocolType::Sixel(sixel) = image_state.protocol_type() else {
            return;
        };
        self.sixel_cache.insert(
            cache_key,
            CachedSixelRender {
                data: sixel.data.clone(),
                area: sixel.area,
                is_tmux: sixel.is_tmux,
            },
        );

        let _ = append_sixel_preview_timing_log(
            &timing.png_path,
            timing.image_pixels,
            target_area,
            timing.png_load_ms,
            sixel_encode_ms,
            sixel.data.len(),
        );
    }

    fn target_path(&self) -> Option<&Path> {
        self.pending_path
            .as_deref()
            .or(self.current_path.as_deref())
    }
}

fn same_path(left: Option<&Path>, right: Option<&Path>) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => left == right,
        (None, None) => true,
        _ => false,
    }
}

#[cfg(test)]
impl PreviewState {
    pub(crate) fn cache_sixel_path_for_test(&mut self, png_path: &Path) {
        self.sixel_cache.insert(
            SixelCacheKey {
                png_path: png_path.to_path_buf(),
                area_cells: (27, 20),
                font_size: (10, 20),
            },
            CachedSixelRender {
                data: "cached".to_string(),
                area: Rect::new(0, 0, 27, 20),
                is_tmux: false,
            },
        );
    }
}
