//! Native Windows 11 caption styling; other platforms retain native chrome.

use raw_window_handle::{RawWindowHandle, WindowHandle};
use windows::Win32::{
    Foundation::HWND,
    Graphics::Dwm::{DWMWA_CAPTION_COLOR, DWMWA_TEXT_COLOR, DwmSetWindowAttribute},
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
