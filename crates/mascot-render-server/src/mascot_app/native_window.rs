use eframe::CreationContext;
use mascot_render_protocol::ScreenRectPx;

#[derive(Default)]
pub(crate) struct NativeWindowHandle {
    #[cfg(target_os = "windows")]
    hwnd: Option<std::num::NonZeroIsize>,
}

impl NativeWindowHandle {
    pub(crate) fn from_creation_context(cc: &CreationContext<'_>) -> Self {
        #[cfg(target_os = "windows")]
        {
            use raw_window_handle::{HasWindowHandle as _, RawWindowHandle};

            let hwnd = cc
                .window_handle()
                .ok()
                .and_then(|handle| match handle.as_raw() {
                    RawWindowHandle::Win32(handle) => Some(handle.hwnd),
                    _ => None,
                });
            Self { hwnd }
        }

        #[cfg(not(target_os = "windows"))]
        {
            let _ = cc;
            Self::default()
        }
    }

    pub(crate) fn monitor_screen_rect(&self) -> Option<ScreenRectPx> {
        #[cfg(target_os = "windows")]
        {
            self.hwnd.and_then(monitor_screen_rect_for_hwnd)
        }

        #[cfg(not(target_os = "windows"))]
        {
            None
        }
    }
}

#[cfg(target_os = "windows")]
fn monitor_screen_rect_for_hwnd(hwnd: std::num::NonZeroIsize) -> Option<ScreenRectPx> {
    use std::mem::size_of;

    use windows_sys::Win32::Foundation::{HWND, RECT};
    use windows_sys::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };

    unsafe {
        let monitor = MonitorFromWindow(hwnd.get() as HWND, MONITOR_DEFAULTTONEAREST);
        if monitor.is_null() {
            return None;
        }
        let mut info = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            rcMonitor: RECT::default(),
            rcWork: RECT::default(),
            dwFlags: 0,
        };
        if GetMonitorInfoW(monitor, &mut info as *mut MONITORINFO) == 0 {
            return None;
        }
        Some(screen_rect_from_monitor_rect(info.rcMonitor))
    }
}

#[cfg(target_os = "windows")]
fn screen_rect_from_monitor_rect(rect: windows_sys::Win32::Foundation::RECT) -> ScreenRectPx {
    screen_rect_from_monitor_bounds([rect.left, rect.top, rect.right, rect.bottom])
}

#[cfg(any(test, target_os = "windows"))]
fn screen_rect_from_monitor_bounds([left, top, right, bottom]: [i32; 4]) -> ScreenRectPx {
    ScreenRectPx {
        min_x: left as f32,
        min_y: top as f32,
        max_x: right as f32,
        max_y: bottom as f32,
    }
}
