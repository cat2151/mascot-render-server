use core::ffi::c_void;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use anyhow::{bail, Result};
use eframe::egui::Rect;

#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::HWND;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Graphics::Gdi::{
    CombineRgn, CreateRectRgn, DeleteObject, SetWindowRgn, RGN_OR,
};

#[cfg(target_os = "windows")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WindowRegionKey {
    enabled: bool,
    image_size: [u32; 2],
    physical_bounds: PhysicalRect,
    alpha_mask_ptr: usize,
    alpha_mask_len: usize,
}

#[cfg(target_os = "windows")]
impl WindowRegionKey {
    pub(crate) fn new(
        enabled: bool,
        image_size: [u32; 2],
        image_rect: Rect,
        pixels_per_point: f32,
        alpha_mask: &[u8],
    ) -> Self {
        Self {
            enabled,
            image_size,
            physical_bounds: PhysicalRect {
                left: (image_rect.min.x * pixels_per_point).ceil() as i32,
                top: (image_rect.min.y * pixels_per_point).ceil() as i32,
                right: (image_rect.max.x * pixels_per_point).ceil() as i32,
                bottom: (image_rect.max.y * pixels_per_point).ceil() as i32,
            },
            alpha_mask_ptr: alpha_mask.as_ptr() as usize,
            alpha_mask_len: alpha_mask.len(),
        }
    }

    pub(crate) fn requires_immediate_apply(self, applied: Self) -> bool {
        self.enabled != applied.enabled
            || self.image_size != applied.image_size
            || self.alpha_mask_ptr != applied.alpha_mask_ptr
            || self.alpha_mask_len != applied.alpha_mask_len
    }

    pub(crate) fn is_enabled(self) -> bool {
        self.enabled
    }
}

#[cfg(target_os = "windows")]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub(crate) struct PhysicalRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[cfg(target_os = "windows")]
#[derive(Clone)]
pub(crate) struct WindowRegionData {
    pub signature: u64,
    pub rects: Vec<PhysicalRect>,
}

#[cfg(target_os = "windows")]
pub(crate) struct WindowRegionCache {
    entries: Vec<(WindowRegionKey, Arc<WindowRegionData>)>,
}

#[cfg(target_os = "windows")]
impl WindowRegionCache {
    const CAPACITY: usize = 4;

    pub(crate) fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub(crate) fn data_for(
        &mut self,
        key: WindowRegionKey,
        image_size: [u32; 2],
        image_rect: Rect,
        pixels_per_point: f32,
        alpha_mask: &[u8],
        alpha_threshold: u8,
    ) -> Result<Arc<WindowRegionData>> {
        if let Some(index) = self
            .entries
            .iter()
            .position(|(entry_key, _)| *entry_key == key)
        {
            let (_, data) = self.entries.remove(index);
            self.entries.push((key, Arc::clone(&data)));
            return Ok(data);
        }

        let data = Arc::new(build_window_region_data(
            image_size,
            image_rect,
            pixels_per_point,
            alpha_mask,
            alpha_threshold,
        )?);
        if self.entries.len() >= Self::CAPACITY {
            self.entries.remove(0);
        }
        self.entries.push((key, Arc::clone(&data)));
        Ok(data)
    }
}

#[cfg(target_os = "windows")]
pub(crate) fn apply_click_through_window_region(hwnd: HWND, rects: &[PhysicalRect]) -> Result<()> {
    let region = create_region_from_rects(rects)?;
    let applied = unsafe { SetWindowRgn(hwnd, region, 1) };
    if applied == 0 {
        unsafe {
            let _ = DeleteObject(region as _);
        }
        bail!(
            "SetWindowRgn failed for mascot window {hwnd:p} with {} opaque rectangles",
            rects.len()
        );
    }
    Ok(())
}

#[cfg(target_os = "windows")]
pub(crate) fn clear_window_region(hwnd: HWND) {
    unsafe {
        let _ = SetWindowRgn(hwnd, std::ptr::null_mut(), 1);
    }
}

#[cfg(target_os = "windows")]
pub(crate) fn build_opaque_region_rects(
    image_size: [u32; 2],
    image_rect: Rect,
    pixels_per_point: f32,
    alpha_mask: &[u8],
    alpha_threshold: u8,
) -> Result<Vec<PhysicalRect>> {
    let [width, height] = image_size;
    if width == 0 || height == 0 {
        return Ok(Vec::new());
    }
    if alpha_mask.len() != width as usize * height as usize {
        bail!(
            "alpha mask length {} does not match image size {:?}",
            alpha_mask.len(),
            image_size
        );
    }
    if image_rect.width() <= 0.0 || image_rect.height() <= 0.0 || pixels_per_point <= 0.0 {
        bail!(
            "invalid image rect {:?} or pixels_per_point {} for click-through region",
            image_rect,
            pixels_per_point
        );
    }

    let left = image_rect.min.x * pixels_per_point;
    let top = image_rect.min.y * pixels_per_point;
    let width_phys = image_rect.width() * pixels_per_point;
    let height_phys = image_rect.height() * pixels_per_point;
    let row_width = width as usize;
    let mut rects = Vec::new();
    let mut active_rects = HashMap::<(i32, i32), PhysicalRect>::new();

    for y in 0..height {
        let top_px = physical_axis_boundary(top, height_phys, y, height);
        let bottom_px = physical_axis_boundary(top, height_phys, y + 1, height);
        if top_px >= bottom_px {
            continue;
        }

        let row_start = y as usize * row_width;
        let row = &alpha_mask[row_start..row_start + row_width];
        let mut run_start = None;
        let mut next_active_rects = HashMap::<(i32, i32), PhysicalRect>::new();
        for (x, &alpha) in row.iter().enumerate() {
            if alpha > alpha_threshold {
                run_start.get_or_insert(x as u32);
                continue;
            }

            if let Some(start_x) = run_start.take() {
                if let Some(rect) = opaque_run_rect(
                    start_x, x as u32, width, left, width_phys, top_px, bottom_px,
                ) {
                    merge_or_insert_opaque_rect(
                        &mut rects,
                        &mut active_rects,
                        &mut next_active_rects,
                        rect,
                    );
                }
            }
        }

        if let Some(start_x) = run_start {
            if let Some(rect) =
                opaque_run_rect(start_x, width, width, left, width_phys, top_px, bottom_px)
            {
                merge_or_insert_opaque_rect(
                    &mut rects,
                    &mut active_rects,
                    &mut next_active_rects,
                    rect,
                );
            }
        }

        rects.extend(active_rects.into_values());
        active_rects = next_active_rects;
    }

    rects.extend(active_rects.into_values());
    rects.sort_by_key(|rect| (rect.top, rect.left, rect.bottom, rect.right));
    Ok(rects)
}

#[cfg(target_os = "windows")]
pub(crate) fn build_window_region_data(
    image_size: [u32; 2],
    image_rect: Rect,
    pixels_per_point: f32,
    alpha_mask: &[u8],
    alpha_threshold: u8,
) -> Result<WindowRegionData> {
    let rects = build_opaque_region_rects(
        image_size,
        image_rect,
        pixels_per_point,
        alpha_mask,
        alpha_threshold,
    )?;
    Ok(WindowRegionData {
        signature: window_region_signature(&rects),
        rects,
    })
}

#[cfg(target_os = "windows")]
fn window_region_signature(rects: &[PhysicalRect]) -> u64 {
    let mut hasher = DefaultHasher::new();
    rects.hash(&mut hasher);
    hasher.finish()
}

#[cfg(target_os = "windows")]
fn merge_or_insert_opaque_rect(
    rects: &mut Vec<PhysicalRect>,
    active_rects: &mut HashMap<(i32, i32), PhysicalRect>,
    next_active_rects: &mut HashMap<(i32, i32), PhysicalRect>,
    rect: PhysicalRect,
) {
    let key = (rect.left, rect.right);
    if let Some(mut active_rect) = active_rects.remove(&key) {
        if active_rect.bottom == rect.top {
            active_rect.bottom = rect.bottom;
            next_active_rects.insert(key, active_rect);
            return;
        }
        rects.push(active_rect);
        next_active_rects.insert(key, rect);
        return;
    }
    next_active_rects.insert(key, rect);
}

#[cfg(target_os = "windows")]
fn opaque_run_rect(
    start_x: u32,
    end_x: u32,
    width: u32,
    left: f32,
    width_phys: f32,
    top_px: i32,
    bottom_px: i32,
) -> Option<PhysicalRect> {
    let left_px = physical_axis_boundary(left, width_phys, start_x, width);
    let right_px = physical_axis_boundary(left, width_phys, end_x, width);
    (left_px < right_px).then_some(PhysicalRect {
        left: left_px,
        top: top_px,
        right: right_px,
        bottom: bottom_px,
    })
}

#[cfg(target_os = "windows")]
fn create_region_from_rects(rects: &[PhysicalRect]) -> Result<*mut c_void> {
    let combined = unsafe { CreateRectRgn(0, 0, 0, 0) };
    if combined.is_null() {
        bail!("CreateRectRgn failed for click-through region accumulator");
    }

    for rect in rects {
        let row_region = unsafe { CreateRectRgn(rect.left, rect.top, rect.right, rect.bottom) };
        if row_region.is_null() {
            unsafe {
                let _ = DeleteObject(combined as _);
            }
            bail!("CreateRectRgn failed for click-through rectangle {rect:?}");
        }

        let combine_result = unsafe { CombineRgn(combined, combined, row_region, RGN_OR) };
        unsafe {
            let _ = DeleteObject(row_region as _);
        }
        if combine_result == 0 {
            unsafe {
                let _ = DeleteObject(combined as _);
            }
            bail!("CombineRgn failed while building the click-through window region");
        }
    }

    Ok(combined)
}

#[cfg(target_os = "windows")]
fn physical_axis_boundary(origin: f32, span: f32, index: u32, dimension: u32) -> i32 {
    (origin + span * (index as f32 / dimension as f32)).ceil() as i32
}
