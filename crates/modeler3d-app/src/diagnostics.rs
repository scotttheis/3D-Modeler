//! Small, bounded diagnostics for startup failures, viewport errors, and UI stalls.

use std::{
    backtrace::Backtrace,
    fmt::Display,
    fs::{self, OpenOptions},
    io::{self, Write},
    panic::{self, PanicHookInfo},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, Once, OnceLock,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use modeler3d_viewport::ViewportFrameReport;

const LOG_FILE_NAME: &str = "modeler3d.log";
const PREVIOUS_LOG_FILE_NAME: &str = "modeler3d.previous.log";
const CRASH_NOTE_FILE_NAME: &str = "last-crash.txt";
const MAX_LOG_BYTES: u64 = 2 * 1024 * 1024;
const MAX_REPORT_LOG_BYTES: usize = 256 * 1024;
const MAX_DETAIL_BYTES: usize = 12 * 1024;
const MAX_CONTEXT_BYTES: usize = 256;
const SLOW_FRAME_THRESHOLD: Duration = Duration::from_millis(500);
const SLOW_FRAME_LOG_INTERVAL: Duration = Duration::from_secs(10);
const UI_STALL_THRESHOLD: Duration = Duration::from_secs(15);
const WATCHDOG_CHECK_INTERVAL: Duration = Duration::from_secs(1);

/// Maximum time between idle UI frames used to distinguish a stall from an idle window.
pub(crate) const UI_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(10);

static LOG_PATH: OnceLock<PathBuf> = OnceLock::new();
static LOG_WRITE_LOCK: Mutex<()> = Mutex::new(());
static PANIC_HOOK_INSTALLED: Once = Once::new();
static WATCHDOG_STATE: OnceLock<Arc<UiWatchdogState>> = OnceLock::new();
static LAST_SLOW_UI_FRAME_MS: AtomicU64 = AtomicU64::new(0);
static LAST_SLOW_VIEWPORT_FRAME_MS: AtomicU64 = AtomicU64::new(0);
static ECHO_TO_CONSOLE: AtomicBool = AtomicBool::new(false);

/// Caches the platform-owned log path without creating the directory or file.
pub(crate) fn initialize() {
    let _path = LOG_PATH.get_or_init(|| {
        modeler3d_platform::current_runtime_paths()
            .log_dir
            .join(LOG_FILE_NAME)
    });
}

/// Mirrors every log line to standard error as well as the log file
/// (`--console`). The log file is always written.
pub(crate) fn echo_to_console() {
    ECHO_TO_CONSOLE.store(true, Ordering::Release);
    eprintln!(
        "3D Modeler diagnostics: also writing to {}",
        log_path().display()
    );
}

/// Installs a panic hook that records the payload and a Rust backtrace.
pub(crate) fn install_panic_hook() {
    PANIC_HOOK_INSTALLED.call_once(|| {
        let previous_hook = panic::take_hook();
        panic::set_hook(Box::new(move |panic_info| {
            let payload = panic_payload(panic_info);
            let location = panic_info
                .location()
                .map(|location| format!("{}:{}", location.file(), location.line()))
                .unwrap_or_else(|| String::from("unknown location"));
            let backtrace = Backtrace::force_capture();
            record(
                "PANIC",
                "unhandled panic",
                &format!("{payload} at {location}\n{backtrace}"),
            );
            // Kept for the next start, which offers to report it.
            let _ = write_crash_note(&format!("{payload} at {location}\n{backtrace}"));
            previous_hook(panic_info);
        }));
    });
}

/// Writes an application error to the local bounded log.
pub(crate) fn record_error(context: &str, detail: impl Display) {
    record("ERROR", context, &detail.to_string());
}

/// Writes a warning to the local bounded log.
pub(crate) fn record_warning(context: &str, detail: impl Display) {
    record("WARN", context, &detail.to_string());
}

/// Writes one informational diagnostic, used only for important state changes.
pub(crate) fn record_info(context: &str, detail: impl Display) {
    record("INFO", context, &detail.to_string());
}

/// Records a renderer frame only when it is unusually slow, with a cooldown.
pub(crate) fn record_viewport_frame(report: &ViewportFrameReport) {
    if !report.frame_time_ms.is_finite()
        || report.frame_time_ms < SLOW_FRAME_THRESHOLD.as_secs_f64() * 1_000.0
        || !should_record_slow_frame(&LAST_SLOW_VIEWPORT_FRAME_MS)
    {
        return;
    }

    record_warning(
        "slow viewport frame",
        format!(
            "{:.0} ms; fallback={}; adapter={} / {} / {}; triangles={}; CPU shaded pixels={}",
            report.frame_time_ms,
            report.cpu_fallback,
            report.adapter.name,
            report.adapter.backend,
            report.adapter.device_type,
            if report.cpu_fallback {
                report.cpu_triangles
            } else {
                report.submitted_triangles
            },
            report.cpu_shaded_pixels
        ),
    );
}

/// Records the completion time for a UI frame and samples long frame durations.
pub(crate) fn complete_ui_frame(duration: Duration) {
    if let Some(state) = WATCHDOG_STATE.get() {
        let completed_ms = state.elapsed_ms();
        state
            .last_completed_frame_ms
            .store(completed_ms, Ordering::Release);

        if state.stall_reported.swap(false, Ordering::AcqRel) {
            let stalled_since = state.stall_started_ms.load(Ordering::Acquire);
            record_info(
                "ui-watchdog",
                format!(
                    "UI frame processing resumed after approximately {} ms.",
                    completed_ms.saturating_sub(stalled_since)
                ),
            );
        }
    }

    if duration >= SLOW_FRAME_THRESHOLD && should_record_slow_frame(&LAST_SLOW_UI_FRAME_MS) {
        record_warning(
            "slow UI frame",
            format!("UI update took {} ms.", duration.as_millis()),
        );
    }
}

/// Background UI-stall detector. It writes only when a stall starts or ends.
pub(crate) struct UiWatchdog {
    state: Arc<UiWatchdogState>,
    worker: Option<JoinHandle<()>>,
}

impl UiWatchdog {
    /// Starts a low-frequency watcher for completed UI frames.
    pub(crate) fn start() -> Option<Self> {
        if WATCHDOG_STATE.get().is_some() {
            return None;
        }

        let state = Arc::new(UiWatchdogState {
            started_at: Instant::now(),
            last_completed_frame_ms: AtomicU64::new(0),
            stall_started_ms: AtomicU64::new(0),
            stall_reported: AtomicBool::new(false),
            stop: AtomicBool::new(false),
        });
        let worker_state = Arc::clone(&state);
        let worker = match thread::Builder::new()
            .name(String::from("modeler3d-ui-watchdog"))
            .spawn(move || watch_ui(worker_state))
        {
            Ok(worker) => worker,
            Err(error) => {
                record_warning(
                    "ui-watchdog",
                    format!("Could not start UI stall detection: {error}"),
                );
                return None;
            }
        };

        if WATCHDOG_STATE.set(Arc::clone(&state)).is_err() {
            state.stop.store(true, Ordering::Release);
            worker.thread().unpark();
            let _ = worker.join();
            return None;
        }

        Some(Self {
            state,
            worker: Some(worker),
        })
    }
}

impl Drop for UiWatchdog {
    fn drop(&mut self) {
        self.state.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            worker.thread().unpark();
            let _ = worker.join();
        }
    }
}

struct UiWatchdogState {
    started_at: Instant,
    last_completed_frame_ms: AtomicU64,
    stall_started_ms: AtomicU64,
    stall_reported: AtomicBool,
    stop: AtomicBool,
}

impl UiWatchdogState {
    fn elapsed_ms(&self) -> u64 {
        self.started_at
            .elapsed()
            .as_millis()
            .min(u128::from(u64::MAX)) as u64
    }
}

fn watch_ui(state: Arc<UiWatchdogState>) {
    loop {
        thread::park_timeout(WATCHDOG_CHECK_INTERVAL);
        if state.stop.load(Ordering::Acquire) {
            return;
        }

        let now_ms = state.elapsed_ms();
        let last_frame_ms = state.last_completed_frame_ms.load(Ordering::Acquire);
        let quiet_ms = now_ms.saturating_sub(last_frame_ms);
        if quiet_ms < UI_STALL_THRESHOLD.as_millis() as u64 {
            continue;
        }

        state
            .stall_started_ms
            .store(last_frame_ms, Ordering::Release);
        if state
            .stall_reported
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            continue;
        }

        // Check again after claiming the event so a frame that just completed
        // does not create a stale stall warning.
        let latest_frame_ms = state.last_completed_frame_ms.load(Ordering::Acquire);
        let quiet_ms = now_ms.saturating_sub(latest_frame_ms);
        if quiet_ms < UI_STALL_THRESHOLD.as_millis() as u64 {
            state.stall_reported.store(false, Ordering::Release);
            continue;
        }

        state
            .stall_started_ms
            .store(latest_frame_ms, Ordering::Release);
        record_warning(
            "ui-watchdog",
            format!(
                "No completed UI frame for at least {quiet_ms} ms; possible UI stall, minimized or suspended window, or OS scheduling pause."
            ),
        );
    }
}

fn panic_payload(panic_info: &PanicHookInfo<'_>) -> String {
    panic_info
        .payload()
        .downcast_ref::<&'static str>()
        .map(|message| (*message).to_owned())
        .or_else(|| panic_info.payload().downcast_ref::<String>().cloned())
        .unwrap_or_else(|| String::from("non-string panic payload"))
}

fn record(level: &str, context: &str, detail: &str) {
    let timestamp_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    let thread = thread::current();
    let thread_name = thread.name().unwrap_or("unnamed");
    let context = single_line(context, MAX_CONTEXT_BYTES);
    let detail = single_line(detail, MAX_DETAIL_BYTES);
    let line = format!(
        "{timestamp_ms}\t{level}\tpid={}\tthread={}\t{context}: {detail}\n",
        std::process::id(),
        single_line(thread_name, MAX_CONTEXT_BYTES)
    );
    if ECHO_TO_CONSOLE.load(Ordering::Acquire) {
        let _ = io::stderr().lock().write_all(line.as_bytes());
    }
    let path = log_path();
    if let Err(error) = append_line_at(&path, line.as_bytes()) {
        let _ = writeln!(
            io::stderr().lock(),
            "3D Modeler could not write its diagnostic log at {}: {error}",
            path.display()
        );
    }
}

/// The last part of the diagnostic log (the previous file, then the current
/// one), at most `MAX_REPORT_LOG_BYTES`, for a problem report. Empty when
/// nothing was ever logged.
pub(crate) fn recent_log_text() -> String {
    let path = log_path();
    let mut bytes = fs::read(path.with_file_name(PREVIOUS_LOG_FILE_NAME)).unwrap_or_default();
    bytes.extend(fs::read(&path).unwrap_or_default());
    let start = bytes.len().saturating_sub(MAX_REPORT_LOG_BYTES);
    String::from_utf8_lossy(&bytes[start..]).into_owned()
}

/// Where the diagnostic log is, for a problem report to name.
pub(crate) fn log_file_path() -> PathBuf {
    log_path()
}

/// Takes the note an internal error left in the last run, if any, so the
/// app offers to report it once.
pub(crate) fn take_crash_note() -> Option<String> {
    let path = log_path().with_file_name(CRASH_NOTE_FILE_NAME);
    let note = fs::read_to_string(&path).ok()?;
    let _ = fs::remove_file(&path);
    let note = note.trim();
    (!note.is_empty()).then(|| note.to_owned())
}

fn write_crash_note(text: &str) -> io::Result<()> {
    let path = log_path().with_file_name(CRASH_NOTE_FILE_NAME);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut text = text.to_owned();
    if text.len() > MAX_DETAIL_BYTES {
        let mut end = MAX_DETAIL_BYTES;
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        text.truncate(end);
    }
    fs::write(path, text)
}

fn log_path() -> PathBuf {
    LOG_PATH.get().cloned().unwrap_or_else(|| {
        modeler3d_platform::current_runtime_paths()
            .log_dir
            .join(LOG_FILE_NAME)
    })
}

fn single_line(value: &str, limit: usize) -> String {
    const TRUNCATION_MARKER: &str = "...[truncated]";
    let content_limit = limit.saturating_sub(TRUNCATION_MARKER.len());
    let mut result = String::with_capacity(limit.min(value.len()));
    let mut truncated = false;

    for character in value.chars() {
        let escaped = match character {
            '\n' => Some("\\n"),
            '\r' => Some("\\r"),
            '\t' => Some("\\t"),
            control if control.is_control() => Some(" "),
            _ => None,
        };
        if let Some(escaped) = escaped {
            if result.len().saturating_add(escaped.len()) > content_limit {
                truncated = true;
                break;
            }
            result.push_str(escaped);
            continue;
        }

        if result.len().saturating_add(character.len_utf8()) > content_limit {
            truncated = true;
            break;
        }
        result.push(character);
    }

    if truncated {
        result.push_str(TRUNCATION_MARKER);
    }
    result
}

fn append_line_at(path: &Path, line: &[u8]) -> io::Result<()> {
    let _guard = LOG_WRITE_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let parent = path.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "diagnostic log path has no parent directory",
        )
    })?;
    fs::create_dir_all(parent)?;

    let current_size = match fs::metadata(path) {
        Ok(metadata) => metadata.len(),
        Err(error) if error.kind() == io::ErrorKind::NotFound => 0,
        Err(error) => return Err(error),
    };
    if current_size.saturating_add(line.len() as u64) > MAX_LOG_BYTES {
        rotate_log(path)?;
    }

    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    file.write_all(line)?;
    file.flush()
}

fn rotate_log(path: &Path) -> io::Result<()> {
    let previous_path = path.with_file_name(PREVIOUS_LOG_FILE_NAME);
    match fs::remove_file(&previous_path) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    match fs::rename(path, previous_path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn should_record_slow_frame(last_recorded_ms: &AtomicU64) -> bool {
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or_default();
    let minimum_interval = SLOW_FRAME_LOG_INTERVAL.as_millis() as u64;
    loop {
        let previous_ms = last_recorded_ms.load(Ordering::Relaxed);
        if previous_ms != 0 && now_ms.saturating_sub(previous_ms) < minimum_interval {
            return false;
        }
        if last_recorded_ms
            .compare_exchange_weak(previous_ms, now_ms, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
        {
            return true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_fields_escape_newlines_and_bound_record_size() {
        assert_eq!(
            single_line("first\nsecond\titem", 64),
            "first\\nsecond\\titem"
        );

        let value = "x".repeat(MAX_DETAIL_BYTES + 100);
        let result = single_line(&value, MAX_DETAIL_BYTES);
        assert!(result.len() <= MAX_DETAIL_BYTES);
        assert!(result.ends_with("...[truncated]"));
    }

    #[test]
    fn log_rotation_keeps_one_previous_file_and_the_new_record() {
        let directory = std::env::temp_dir().join(format!(
            "modeler3d-diagnostics-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or_default()
        ));
        let path = directory.join(LOG_FILE_NAME);
        let previous_path = directory.join(PREVIOUS_LOG_FILE_NAME);
        assert!(fs::create_dir_all(&directory).is_ok());
        assert!(fs::write(&path, vec![b'x'; MAX_LOG_BYTES as usize]).is_ok());

        let append_result = append_line_at(&path, b"recent diagnostic\n");
        assert!(append_result.is_ok());
        assert_eq!(
            fs::metadata(&previous_path)
                .map(|metadata| metadata.len())
                .ok(),
            Some(MAX_LOG_BYTES)
        );
        assert_eq!(
            fs::read(&path).ok().as_deref(),
            Some(b"recent diagnostic\n".as_slice())
        );
        let _ = fs::remove_dir_all(directory);
    }
}
