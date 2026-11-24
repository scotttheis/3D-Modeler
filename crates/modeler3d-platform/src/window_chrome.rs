//! Native Windows 11 caption styling and window-size guard; other platforms
//! retain native chrome.

use raw_window_handle::{RawWindowHandle, WindowHandle};
use windows::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    Graphics::Dwm::{DWMWA_CAPTION_COLOR, DWMWA_TEXT_COLOR, DwmSetWindowAttribute},
    UI::{
        Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass},
        WindowsAndMessaging::{
            GetSystemMetrics, SM_CXMAXTRACK, SM_CYMAXTRACK, SWP_NOSIZE, WINDOWPOS, WM_NCDESTROY,
            WM_WINDOWPOSCHANGING,
        },
    },
};

/// Sets the Windows 11 native caption and text colours for a live borrowed
/// window. Older Windows versions may reject these optional attributes.
pub fn set_title_bar_colors(
    window: WindowHandle<'_>,
    background: [u8; 3],
    text: [u8; 3],
) -> windows::core::Result<()> {
    let RawWindowHandle::Win32(handle) = window.as_raw() else {
        return Ok(());
    };
    let hwnd = HWND(handle.hwnd.get() as *mut core::ffi::c_void);
    for (attribute, rgb) in [(DWMWA_CAPTION_COLOR, background), (DWMWA_TEXT_COLOR, text)] {
        let color = u32::from_le_bytes([rgb[0], rgb[1], rgb[2], 0]);
        // SAFETY: the borrowed handle keeps the window alive for this call;
        // DWM reads a correctly sized COLORREF synchronously.
        unsafe {
            DwmSetWindowAttribute(
                hwnd,
                attribute,
                (&color as *const u32).cast(),
                core::mem::size_of_val(&color) as u32,
            )?;
        }
    }
    Ok(())
}

/// Identifies this crate's size-guard subclass on a window.
const SIZE_GUARD_ID: usize = 0x4D33_4453; // "M3DS"

/// Caps the outer size of a live borrowed window, whoever asks for the size:
/// the user dragging an edge, maximising, or another program calling
/// `SetWindowPos`/`MoveWindow` (which skips the minimum and maximum track
/// sizes a toolkit sets). The cap on each side is the smaller of
/// `max_pixels` and Windows' own largest drag size (`SM_CXMAXTRACK` /
/// `SM_CYMAXTRACK`, about all monitors together), read at each resize so a
/// newly attached monitor counts. The drawing surface is the client area,
/// which is never larger than the outer window, so it stays within a GPU
/// texture limit of `max_pixels` and never needs more graphics memory than
/// a window spanning every monitor (OF-71).
///
/// Windows only; other platforms rely on the toolkit's maximum inner size.
/// Calling it again replaces the earlier cap. The guard removes itself when
/// the window is destroyed.
pub fn limit_window_size(window: WindowHandle<'_>, max_pixels: u32) -> windows::core::Result<()> {
    let RawWindowHandle::Win32(handle) = window.as_raw() else {
        return Ok(());
    };
    let hwnd = HWND(handle.hwnd.get() as *mut core::ffi::c_void);
    // SAFETY: the borrowed handle keeps the window alive for this call, the
    // window belongs to this thread (eframe creates it on the UI thread), and
    // the procedure below has the SUBCLASSPROC signature.
    unsafe {
        SetWindowSubclass(
            hwnd,
            Some(size_guard_proc),
            SIZE_GUARD_ID,
            max_pixels as usize,
        )
        .ok()
    }
}

unsafe extern "system" fn size_guard_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    id: usize,
    max_pixels: usize,
) -> LRESULT {
    if message == WM_WINDOWPOSCHANGING && lparam.0 != 0 {
        // SAFETY: for WM_WINDOWPOSCHANGING, lparam points to a WINDOWPOS the
        // caller owns for the duration of the message and lets us edit.
        let position = unsafe { &mut *(lparam.0 as *mut WINDOWPOS) };
        let texture_max = i32::try_from(max_pixels).unwrap_or(i32::MAX);
        // SAFETY: GetSystemMetrics has no preconditions; 0 means unavailable.
        let (track_width, track_height) = unsafe {
            (
                GetSystemMetrics(SM_CXMAXTRACK),
                GetSystemMetrics(SM_CYMAXTRACK),
            )
        };
        if let Some((width, height)) = capped_size(
            (position.cx, position.cy),
            position.flags.contains(SWP_NOSIZE),
            (
                side_cap(texture_max, track_width),
                side_cap(texture_max, track_height),
            ),
        ) {
            position.cx = width;
            position.cy = height;
        }
    } else if message == WM_NCDESTROY {
        // SAFETY: removes only our own subclass from the window being destroyed.
        unsafe {
            let _ = RemoveWindowSubclass(hwnd, Some(size_guard_proc), id);
        }
    }
    // SAFETY: forwards the unchanged message (or the capped WINDOWPOS) to the
    // next procedure in the subclass chain.
    unsafe { DefSubclassProc(hwnd, message, wparam, lparam) }
}

/// One side's cap: the texture limit, lowered to the system's largest drag
/// size when Windows reports one.
fn side_cap(texture_max: i32, track_max: i32) -> i32 {
    if track_max > 0 {
        texture_max.min(track_max)
    } else {
        texture_max
    }
}

/// The capped outer size for a requested move, or `None` when the request
/// keeps its size or is already within the cap.
fn capped_size(
    (width, height): (i32, i32),
    keeps_size: bool,
    (max_width, max_height): (i32, i32),
) -> Option<(i32, i32)> {
    if keeps_size || (width <= max_width && height <= max_height) {
        return None;
    }
    Some((width.min(max_width), height.min(max_height)))
}

#[cfg(test)]
mod tests {
    use super::{capped_size, side_cap};

    #[test]
    fn oversized_requests_are_capped_on_each_side() {
        let cap = (8_192, 8_192);
        assert_eq!(capped_size((700, 32_728), false, cap), Some((700, 8_192)));
        assert_eq!(
            capped_size((40_000, 30_000), false, (2_576, 1_456)),
            Some((2_576, 1_456))
        );
    }

    #[test]
    fn in_range_and_move_only_requests_are_left_alone() {
        let cap = (8_192, 8_192);
        assert_eq!(capped_size((1_280, 840), false, cap), None);
        assert_eq!(capped_size((8_192, 8_192), false, cap), None);
        assert_eq!(capped_size((0, 0), false, cap), None);
        assert_eq!(capped_size((40_000, 40_000), true, cap), None);
    }

    #[test]
    fn each_side_uses_the_smaller_of_texture_and_drag_limits() {
        assert_eq!(side_cap(32_768, 2_576), 2_576);
        assert_eq!(side_cap(4_096, 7_696), 4_096);
        assert_eq!(side_cap(8_192, 0), 8_192);
    }
}
