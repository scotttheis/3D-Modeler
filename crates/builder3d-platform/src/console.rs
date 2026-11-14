//! Text console attachment for a windowed executable.
//!
//! Contract: on Windows the release application is built as a windowed
//! (GUI-subsystem) program so no console window opens behind it. Command-line
//! modes (`--smoke`, `--package`, review captures, `--console`) call
//! [`attach_console`] so their output still reaches the terminal that started
//! them. Output already redirected to a file or pipe is unaffected. On other
//! platforms executables always keep the terminal they were started from, so
//! the function does nothing and reports `true`.

/// Connects standard output and error to a console.
///
/// Attaches to the parent process's console when there is one. When there is
/// none and `allocate` is `true`, opens a new console window (used by the
/// explicit `--console` diagnostics flag). Returns whether a console is
/// available afterwards. Never panics.
#[must_use]
pub fn attach_console(allocate: bool) -> bool {
    imp::attach_console(allocate)
}

#[cfg(windows)]
#[allow(unsafe_code)]
mod imp {
    use windows::Win32::System::Console::{ATTACH_PARENT_PROCESS, AllocConsole, AttachConsole};

    pub(super) fn attach_console(allocate: bool) -> bool {
        // SAFETY: both calls take no pointers and only change which console
        // this process is connected to; failure is reported, not undefined.
        if unsafe { AttachConsole(ATTACH_PARENT_PROCESS) }.is_ok() {
            return true;
        }
        allocate && unsafe { AllocConsole() }.is_ok()
    }
}

#[cfg(not(windows))]
mod imp {
    pub(super) fn attach_console(_allocate: bool) -> bool {
        true
    }
}
