//! Portable runtime paths, process checks, and desktop environment guards.
//!
//! These seams accept injected environment values so path policy and startup
//! checks stay testable without creating user-profile directories or loading a
//! desktop toolkit.

use std::{
    env,
    path::{Path, PathBuf},
};

use sysinfo::{Pid, ProcessesToUpdate, System};
use thiserror::Error;

/// Product name used in platform-owned runtime directories.
pub const APP_NAME: &str = "Builder3D";
/// Environment override for the MCP runtime descriptor path.
pub const RUNTIME_DESCRIPTOR_ENV: &str = "BUILDER3D_RUNTIME_FILE";
/// Default descriptor filename under the application configuration directory.
pub const RUNTIME_DESCRIPTOR_NAME: &str = "mcp-runtime.json";

/// Platform-owned configuration, cache, logging, descriptor, and temp paths.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimePaths {
    /// Persistent application configuration directory.
    pub config_dir: PathBuf,
    /// Disposable application cache directory.
    pub cache_dir: PathBuf,
    /// Application log directory.
    pub log_dir: PathBuf,
    /// MCP descriptor discovered by local clients.
    pub mcp_descriptor_path: PathBuf,
    /// Temporary files owned by the application.
    pub temp_dir: PathBuf,
}

/// Returns platform-appropriate paths without creating directories.
pub fn default_runtime_paths(
    environment: &[(String, String)],
    home: &Path,
    platform_name: &str,
) -> RuntimePaths {
    let platform = platform_name.trim().to_ascii_lowercase();
    let windows_target = is_windows(&platform);
    let home = home.to_path_buf();

    let (config_dir, cache_dir, log_dir, temp_base) = if windows_target {
        let base = environment_value(environment, "LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join("AppData").join("Local"));
        (
            base.join(APP_NAME),
            base.join(APP_NAME).join("cache"),
            base.join(APP_NAME).join("logs"),
            environment_path(environment, &["TEMP", "TMP"], base.join("Temp")),
        )
    } else if is_macos(&platform) {
        (
            home.join("Library")
                .join("Application Support")
                .join(APP_NAME),
            home.join("Library").join("Caches").join(APP_NAME),
            home.join("Library").join("Logs").join(APP_NAME),
            environment_path(
                environment,
                &["TMPDIR", "TEMP", "TMP"],
                PathBuf::from("/tmp"),
            ),
        )
    } else {
        let config_base = environment_path(environment, &["XDG_CONFIG_HOME"], home.join(".config"));
        let cache_base = environment_path(environment, &["XDG_CACHE_HOME"], home.join(".cache"));
        let state_base = environment_path(
            environment,
            &["XDG_STATE_HOME"],
            home.join(".local").join("state"),
        );
        (
            config_base.join(APP_NAME),
            cache_base.join(APP_NAME),
            state_base.join(APP_NAME),
            environment_path(
                environment,
                &["TMPDIR", "TEMP", "TMP"],
                PathBuf::from("/tmp"),
            ),
        )
    };

    let descriptor = environment_value(environment, RUNTIME_DESCRIPTOR_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|| config_dir.join(RUNTIME_DESCRIPTOR_NAME));
    RuntimePaths {
        config_dir,
        cache_dir: cache_dir.clone(),
        log_dir,
        mcp_descriptor_path: descriptor,
        temp_dir: temp_base.join(APP_NAME),
    }
}

/// Returns paths for the current host without creating directories.
#[must_use]
pub fn current_runtime_paths() -> RuntimePaths {
    let environment = env::vars().collect::<Vec<_>>();
    let home = environment_value(&environment, "USERPROFILE")
        .or_else(|| environment_value(&environment, "HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    default_runtime_paths(&environment, &home, env::consts::OS)
}

/// Resolves the MCP descriptor, honoring an explicit environment override.
#[must_use]
pub fn resolve_runtime_descriptor_path(
    runtime_paths: Option<&RuntimePaths>,
    environment: &[(String, String)],
    home: &Path,
    platform_name: &str,
) -> PathBuf {
    environment_value(environment, RUNTIME_DESCRIPTOR_ENV)
        .map(PathBuf::from)
        .or_else(|| runtime_paths.map(|paths| paths.mcp_descriptor_path.clone()))
        .unwrap_or_else(|| {
            default_runtime_paths(environment, home, platform_name).mcp_descriptor_path
        })
}

/// Returns whether a process ID is currently visible to the host process.
///
/// The query is read-only and uses `sysinfo` rather than sending a signal or
/// opening a platform-specific process handle.
#[must_use]
pub fn process_is_alive(process_id: u32) -> bool {
    if process_id == 0 {
        return false;
    }
    let Ok(mut system) = System::new_all() else {
        return false;
    };
    system.refresh_processes(ProcessesToUpdate::All, true);
    system.process(Pid::from(process_id as usize)).is_some()
}

/// Startup guard retained from the Python OpenGL desktop adapter.
#[derive(Debug, Error, Eq, PartialEq)]
pub enum RuntimeCheckError {
    /// Qt's offscreen platform cannot create the product's interactive OpenGL
    /// widget. The native Rust shell does not load Qt, but the guard remains a
    /// useful boundary for compatibility adapters and diagnostics.
    #[error(
        "Builder3D cannot run with QT_QPA_PLATFORM=offscreen because its desktop viewport requires an interactive desktop session"
    )]
    OffscreenGui,
}

/// Rejects the known headless Qt platform before an OpenGL desktop adapter.
pub fn ensure_opengl_gui_platform(
    environment: &[(String, String)],
) -> Result<(), RuntimeCheckError> {
    let platform = environment_value(environment, "QT_QPA_PLATFORM").unwrap_or_default();
    if platform
        .split_once(':')
        .map_or(platform, |(name, _)| name)
        .trim()
        .eq_ignore_ascii_case("offscreen")
    {
        return Err(RuntimeCheckError::OffscreenGui);
    }
    Ok(())
}

fn environment_path(
    environment: &[(String, String)],
    names: &[&str],
    fallback: PathBuf,
) -> PathBuf {
    names
        .iter()
        .find_map(|name| environment_value(environment, name).map(PathBuf::from))
        .unwrap_or(fallback)
}

fn environment_value<'a>(environment: &'a [(String, String)], wanted: &str) -> Option<&'a str> {
    environment
        .iter()
        .find(|(name, value)| name.eq_ignore_ascii_case(wanted) && !value.is_empty())
        .map(|(_, value)| value.as_str())
}

fn is_windows(platform: &str) -> bool {
    matches!(platform, "windows" | "nt" | "win32" | "cygwin") || platform.starts_with("win")
}

fn is_macos(platform: &str) -> bool {
    matches!(platform, "darwin" | "macos" | "osx")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn environment(values: &[(&str, &str)]) -> Vec<(String, String)> {
        values
            .iter()
            .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
            .collect()
    }

    #[test]
    fn windows_paths_use_local_app_data_and_temp() {
        let values = environment(&[
            ("LOCALAPPDATA", r"C:\Users\Test\AppData\Local"),
            ("TEMP", r"C:\Temp"),
        ]);
        let paths = default_runtime_paths(&values, Path::new(r"C:\Users\Test"), "windows");
        assert_eq!(
            paths.config_dir,
            PathBuf::from(r"C:\Users\Test\AppData\Local\Builder3D")
        );
        assert_eq!(
            paths.cache_dir,
            PathBuf::from(r"C:\Users\Test\AppData\Local\Builder3D\cache")
        );
        assert_eq!(
            paths.log_dir,
            PathBuf::from(r"C:\Users\Test\AppData\Local\Builder3D\logs")
        );
        assert_eq!(paths.temp_dir, PathBuf::from(r"C:\Temp\Builder3D"));
    }

    #[test]
    fn linux_paths_use_xdg_locations_and_descriptor_override() {
        let values = environment(&[
            ("XDG_CONFIG_HOME", "/config"),
            ("XDG_CACHE_HOME", "/cache"),
            ("XDG_STATE_HOME", "/state"),
            (RUNTIME_DESCRIPTOR_ENV, "/shared/mcp-runtime.json"),
        ]);
        let paths = default_runtime_paths(&values, Path::new("/home/test"), "linux");
        assert_eq!(paths.config_dir, PathBuf::from("/config/Builder3D"));
        assert_eq!(paths.cache_dir, PathBuf::from("/cache/Builder3D"));
        assert_eq!(paths.log_dir, PathBuf::from("/state/Builder3D"));
        assert_eq!(
            paths.mcp_descriptor_path,
            PathBuf::from("/shared/mcp-runtime.json")
        );
        assert_eq!(paths.temp_dir, PathBuf::from("/tmp/Builder3D"));
    }

    #[test]
    fn descriptor_resolution_prefers_override_then_injected_bundle() {
        let values = environment(&[]);
        let paths = default_runtime_paths(&values, Path::new("/home/test"), "linux");
        assert_eq!(
            resolve_runtime_descriptor_path(Some(&paths), &values, Path::new("/other"), "linux"),
            paths.mcp_descriptor_path
        );
        let override_values = environment(&[(RUNTIME_DESCRIPTOR_ENV, "/override.json")]);
        assert_eq!(
            resolve_runtime_descriptor_path(
                Some(&paths),
                &override_values,
                Path::new("/other"),
                "linux"
            ),
            PathBuf::from("/override.json")
        );
    }

    #[test]
    fn process_liveness_rejects_zero_and_finds_current_process() {
        assert!(!process_is_alive(0));
        assert!(process_is_alive(std::process::id()));
        assert!(!process_is_alive(u32::MAX));
    }

    #[test]
    fn offscreen_guard_is_case_insensitive_and_ignores_other_platforms() {
        assert!(ensure_opengl_gui_platform(&environment(&[])).is_ok());
        assert!(
            ensure_opengl_gui_platform(&environment(&[("QT_QPA_PLATFORM", "wayland")])).is_ok()
        );
        assert!(matches!(
            ensure_opengl_gui_platform(&environment(&[("QT_QPA_PLATFORM", "OFFSCREEN:verbose")])),
            Err(RuntimeCheckError::OffscreenGui)
        ));
    }
}
