#[cfg(target_os = "windows")]
use crate::window_region::{
    apply_click_through_window_region, clear_window_region, WindowRegionCache, WindowRegionKey,
};
#[cfg(target_os = "windows")]
use crate::window_region_sync::{WindowRegionSyncAction, WindowRegionSyncState};
use anyhow::{anyhow, bail, Context, Result};
use eframe::egui::{Pos2, Rect};
use eframe::CreationContext;
use std::sync::Arc;
use std::time::{Duration, Instant};
const HIT_TEST_ALPHA_THRESHOLD: u8 = 8;
#[cfg(target_os = "windows")]
const TRANSPARENT_INPUT_DEBUG_FLASH_DURATION: Duration = Duration::from_secs(1);
pub struct TransparentHitTestWindow {
    #[cfg(target_os = "windows")]
    inner: Option<WindowsTransparentHitTestWindow>,
}
impl TransparentHitTestWindow {
    pub fn disabled() -> Self {
        Self {
            #[cfg(target_os = "windows")]
            inner: None,
        }
    }
    pub fn try_install(cc: &CreationContext<'_>) -> Result<Self> {
        #[cfg(target_os = "windows")]
        {
            let hwnd = hwnd_from_creation_context(cc)?;
            let inner = WindowsTransparentHitTestWindow::install(hwnd, cc.egui_ctx.clone())?;
            return Ok(Self { inner: Some(inner) });
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = cc;
            Ok(Self::disabled())
        }
    }
    pub fn update(
        &mut self,
        now: Instant,
        enabled: bool,
        debug_flash_enabled: bool,
        alpha_mask: Arc<[u8]>,
        image_size: [u32; 2],
        image_rect: Rect,
        pixels_per_point: f32,
    ) {
        #[cfg(target_os = "windows")]
        if let Some(inner) = &mut self.inner {
            inner.update(
                now,
                enabled,
                debug_flash_enabled,
                alpha_mask,
                image_size,
                image_rect,
                pixels_per_point,
            );
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = (
                now,
                enabled,
                debug_flash_enabled,
                alpha_mask,
                image_size,
                image_rect,
                pixels_per_point,
            );
        }
    }
    pub fn transparent_input_visual_remaining(&self, now: Instant) -> Option<Duration> {
        #[cfg(target_os = "windows")]
        {
            return self
                .inner
                .as_ref()
                .and_then(|inner| inner.transparent_input_visual_remaining(now));
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = now;
            None
        }
    }
    pub fn flash_transparent_input_visual(&mut self) {
        #[cfg(target_os = "windows")]
        if let Some(inner) = &mut self.inner {
            inner.flash_transparent_input_visual();
        }
    }
}
#[cfg(target_os = "windows")]
use eframe::egui::Context as EguiContext;
#[cfg(target_os = "windows")]
use raw_window_handle::{HasWindowHandle as _, RawWindowHandle};
#[cfg(target_os = "windows")]
use std::ptr::NonNull;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
#[cfg(target_os = "windows")]
use windows_sys::Win32::Graphics::Gdi::ScreenToClient;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, VK_LBUTTON, VK_MBUTTON, VK_RBUTTON, VK_XBUTTON1, VK_XBUTTON2,
};
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass};
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::{HTTRANSPARENT, WM_NCDESTROY, WM_NCHITTEST};

#[cfg(target_os = "windows")]
const SUBCLASS_ID: usize = 1;
#[cfg(target_os = "windows")]
struct WindowsTransparentHitTestWindow {
    hwnd: HWND,
    state: NonNull<TransparentHitTestState>,
}
#[cfg(target_os = "windows")]
impl WindowsTransparentHitTestWindow {
    fn install(hwnd: HWND, repaint_ctx: EguiContext) -> Result<Self> {
        let state = Box::new(TransparentHitTestState::new(repaint_ctx));
        let state = NonNull::from(Box::leak(state));
        let installed = unsafe {
            SetWindowSubclass(
                hwnd,
                Some(transparent_hit_test_subclass_proc),
                SUBCLASS_ID,
                state.as_ptr() as usize,
            )
        };
        if installed == 0 {
            unsafe {
                drop(Box::from_raw(state.as_ptr()));
            }
            bail!("SetWindowSubclass failed for mascot window {hwnd:p}");
        }
        Ok(Self { hwnd, state })
    }
    fn update(
        &mut self,
        now: Instant,
        enabled: bool,
        debug_flash_enabled: bool,
        alpha_mask: Arc<[u8]>,
        image_size: [u32; 2],
        image_rect: Rect,
        pixels_per_point: f32,
    ) {
        unsafe {
            let state = self.state.as_mut();
            state.enabled = enabled;
            state.debug_flash_enabled = debug_flash_enabled;
            state.alpha_mask = alpha_mask;
            state.image_size = image_size;
            state.image_rect = image_rect;
            state.pixels_per_point = pixels_per_point;

            let region_key = WindowRegionKey::new(
                enabled,
                image_size,
                image_rect,
                pixels_per_point,
                state.alpha_mask.as_ref(),
            );
            sync_window_region(self.hwnd, state, now, region_key);
        }
    }
    fn transparent_input_visual_remaining(&self, now: Instant) -> Option<Duration> {
        unsafe {
            self.state
                .as_ref()
                .transparent_input_visual_until
                .and_then(|until| until.checked_duration_since(now))
        }
    }
    fn flash_transparent_input_visual(&mut self) {
        unsafe {
            self.state.as_mut().activate_transparent_input_visual();
        }
    }
}
#[cfg(target_os = "windows")]
impl Drop for WindowsTransparentHitTestWindow {
    fn drop(&mut self) {
        unsafe {
            let state = self.state.as_ref();
            clear_window_region(self.hwnd);
            if state.window_alive {
                let _ = RemoveWindowSubclass(
                    self.hwnd,
                    Some(transparent_hit_test_subclass_proc),
                    SUBCLASS_ID,
                );
            }
            drop(Box::from_raw(self.state.as_ptr()));
        }
    }
}
#[cfg(target_os = "windows")]
struct TransparentHitTestState {
    enabled: bool,
    debug_flash_enabled: bool,
    alpha_mask: Arc<[u8]>,
    image_size: [u32; 2],
    image_rect: Rect,
    pixels_per_point: f32,
    repaint_ctx: EguiContext,
    transparent_input_visual_until: Option<Instant>,
    window_region_applied_signature: Option<u64>,
    window_region_sync: WindowRegionSyncState,
    window_region_cache: WindowRegionCache,
    window_region_error_logged: bool,
    window_alive: bool,
}
#[cfg(target_os = "windows")]
impl TransparentHitTestState {
    fn new(repaint_ctx: EguiContext) -> Self {
        Self {
            enabled: false,
            debug_flash_enabled: false,
            alpha_mask: Arc::from([]),
            image_size: [0, 0],
            image_rect: Rect::from_min_max(Pos2::ZERO, Pos2::ZERO),
            pixels_per_point: 1.0,
            repaint_ctx,
            transparent_input_visual_until: None,
            window_region_applied_signature: None,
            window_region_sync: WindowRegionSyncState::new(),
            window_region_cache: WindowRegionCache::new(),
            window_region_error_logged: false,
            window_alive: true,
        }
    }
    fn activate_transparent_input_visual(&mut self) {
        self.transparent_input_visual_until =
            Some(Instant::now() + TRANSPARENT_INPUT_DEBUG_FLASH_DURATION);
        self.repaint_ctx.request_repaint();
    }
}
#[cfg(target_os = "windows")]
fn sync_window_region(
    hwnd: HWND,
    state: &mut TransparentHitTestState,
    now: Instant,
    region_key: WindowRegionKey,
) {
    match state.window_region_sync.next_action(now, region_key) {
        WindowRegionSyncAction::None => {}
        WindowRegionSyncAction::Clear => {
            clear_window_region(hwnd);
            state.window_region_sync.mark_cleared();
        }
        WindowRegionSyncAction::Apply => {
            apply_window_region(hwnd, state, region_key);
        }
    }
}
#[cfg(target_os = "windows")]
fn apply_window_region(
    hwnd: HWND,
    state: &mut TransparentHitTestState,
    region_key: WindowRegionKey,
) {
    let apply_result = if state.enabled {
        state
            .window_region_cache
            .data_for(
                region_key,
                state.image_size,
                state.image_rect,
                state.pixels_per_point,
                state.alpha_mask.as_ref(),
                HIT_TEST_ALPHA_THRESHOLD,
            )
            .and_then(|data| {
                if state.window_region_sync.is_active()
                    && state.window_region_applied_signature == Some(data.signature)
                {
                    return Ok((false, data.signature));
                }
                apply_click_through_window_region(hwnd, &data.rects)?;
                Ok((true, data.signature))
            })
    } else {
        clear_window_region(hwnd);
        Ok((true, 0))
    };
    match apply_result {
        Ok((applied, signature)) => {
            state
                .window_region_sync
                .mark_applied(region_key, state.enabled);
            if state.enabled {
                state.window_region_applied_signature = Some(signature);
            } else {
                state.window_region_applied_signature = None;
            }
            if !applied {
                state.window_region_error_logged = false;
                return;
            }
            state.window_region_error_logged = false;
        }
        Err(error) => {
            clear_window_region(hwnd);
            state.window_region_sync.mark_cleared();
            if !state.window_region_error_logged {
                eprintln!(
                    "transparent background click-through window region update failed: {error:#}"
                );
            }
            state.window_region_error_logged = true;
        }
    }
}
#[cfg(target_os = "windows")]
unsafe extern "system" fn transparent_hit_test_subclass_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _subclass_id: usize,
    ref_data: usize,
) -> LRESULT {
    let state = &mut *(ref_data as *mut TransparentHitTestState);
    match msg {
        WM_NCHITTEST => {
            if !state.enabled {
                return DefSubclassProc(hwnd, msg, wparam, lparam);
            }
            if let Some(client_point) = screen_to_client_point(hwnd, lparam) {
                if !captures_client_point(
                    state.image_size,
                    state.image_rect,
                    state.pixels_per_point,
                    state.alpha_mask.as_ref(),
                    client_point,
                    HIT_TEST_ALPHA_THRESHOLD,
                ) {
                    if state.debug_flash_enabled && any_mouse_button_down() {
                        state.activate_transparent_input_visual();
                    }
                    return HTTRANSPARENT as LRESULT;
                }
            }
        }
        WM_NCDESTROY => {
            state.window_alive = false;
            let _ =
                RemoveWindowSubclass(hwnd, Some(transparent_hit_test_subclass_proc), SUBCLASS_ID);
        }
        _ => {}
    }
    DefSubclassProc(hwnd, msg, wparam, lparam)
}
#[cfg(target_os = "windows")]
fn hwnd_from_creation_context(cc: &CreationContext<'_>) -> Result<HWND> {
    let raw_window_handle = cc
        .window_handle()
        .context("mascot window handle is unavailable during transparent hit-test setup")?
        .as_raw();
    match raw_window_handle {
        RawWindowHandle::Win32(handle) => Ok(handle.hwnd.get() as HWND),
        other => Err(anyhow!(
            "transparent hit testing requires a Win32 window handle, got {other:?}"
        )),
    }
}
#[cfg(target_os = "windows")]
fn any_mouse_button_down() -> bool {
    [VK_LBUTTON, VK_RBUTTON, VK_MBUTTON, VK_XBUTTON1, VK_XBUTTON2]
        .into_iter()
        .any(|button| unsafe { GetAsyncKeyState(button.into()) } < 0)
}
#[cfg(target_os = "windows")]
fn screen_to_client_point(hwnd: HWND, lparam: LPARAM) -> Option<[i32; 2]> {
    let mut point = POINT {
        x: signed_word(lparam as u32) as i32,
        y: signed_word((lparam as u32) >> 16) as i32,
    };
    let converted = unsafe { ScreenToClient(hwnd, &mut point) };
    (converted != 0).then_some([point.x, point.y])
}
#[cfg(target_os = "windows")]
fn signed_word(value: u32) -> i16 {
    (value as u16) as i16
}
pub(crate) fn captures_client_point(
    image_size: [u32; 2],
    image_rect: Rect,
    pixels_per_point: f32,
    alpha_mask: &[u8],
    client_point: [i32; 2],
    alpha_threshold: u8,
) -> bool {
    let [width, height] = image_size;
    if width == 0 || height == 0 {
        return false;
    }
    if alpha_mask.len() != (width as usize * height as usize) {
        return false;
    }
    if image_rect.width() <= 0.0 || image_rect.height() <= 0.0 || pixels_per_point <= 0.0 {
        return false;
    }
    let logical_point = Pos2::new(
        client_point[0] as f32 / pixels_per_point,
        client_point[1] as f32 / pixels_per_point,
    );
    captures_logical_point(
        image_size,
        image_rect,
        alpha_mask,
        logical_point,
        alpha_threshold,
    )
}
pub fn captures_logical_point(
    image_size: [u32; 2],
    image_rect: Rect,
    alpha_mask: &[u8],
    logical_point: Pos2,
    alpha_threshold: u8,
) -> bool {
    let [width, height] = image_size;
    if width == 0 || height == 0 {
        return false;
    }
    if alpha_mask.len() != (width as usize * height as usize) {
        return false;
    }
    if image_rect.width() <= 0.0 || image_rect.height() <= 0.0 {
        return false;
    }
    if !image_rect.contains(logical_point) {
        return false;
    }
    let image_x =
        image_index_for_axis(logical_point.x, image_rect.min.x, image_rect.width(), width);
    let image_y = image_index_for_axis(
        logical_point.y,
        image_rect.min.y,
        image_rect.height(),
        height,
    );
    let alpha_index = image_y as usize * width as usize + image_x as usize;
    alpha_mask[alpha_index] > alpha_threshold
}
fn image_index_for_axis(position: f32, min: f32, span: f32, dimension: u32) -> u32 {
    let max_index = dimension.saturating_sub(1);
    let normalized = ((position - min) / span).clamp(0.0, 0.999_999_94);
    ((normalized * dimension as f32).floor() as u32).min(max_index)
}
