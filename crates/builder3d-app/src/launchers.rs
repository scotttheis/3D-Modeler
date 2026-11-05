//! Fixed-path native launcher contracts.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use thiserror::Error;

const MAX_LAUNCHER_BYTES: usize = 16 * 1024;

/// A launcher file embedded in the application package.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LauncherSpec {
    /// Relative package path.
    pub relative_path: &'static str,
    /// Launcher source.
    pub contents: &'static str,
}

/// Errors from launcher validation or staging.
#[derive(Debug, Error)]
pub enum LauncherError {
    /// A fixed launcher violates the private native launch contract.
    #[error("launcher {name} is invalid: {reason}")]
    Invalid {
        /// Relative launcher path.
        name: &'static str,
        /// Validation reason.
        reason: &'static str,
    },
    /// The launcher output directory could not be written.
    #[error("launcher output failed: {0}")]
    Io(#[from] io::Error),
    /// The output path was outside a bounded directory contract.
    #[error("launcher output path is invalid: {0}")]
    InvalidPath(String),
}

/// Returns every supported native launcher.
#[must_use]
pub const fn launcher_specs() -> [LauncherSpec; 5] {
    [
        LauncherSpec {
            relative_path: "Builder3D.bat",
            contents: include_str!("../../../packaging/launchers/Builder3D.bat"),
        },
        LauncherSpec {
            relative_path: "Builder3D.ps1",
            contents: include_str!("../../../packaging/launchers/Builder3D.ps1"),
        },
        LauncherSpec {
            relative_path: "builder3d.sh",
            contents: include_str!("../../../packaging/launchers/builder3d.sh"),
        },
        LauncherSpec {
            relative_path: "Builder3D.command",
            contents: include_str!("../../../packaging/launchers/Builder3D.command"),
        },
        LauncherSpec {
            relative_path: "builder3d.desktop",
            contents: include_str!("../../../packaging/launchers/builder3d.desktop"),
        },
    ]
}

/// Validates the fixed launchers without executing them.
pub fn validate_launchers() -> Result<(), LauncherError> {
    for launcher in launcher_specs() {
        if launcher.contents.len() > MAX_LAUNCHER_BYTES {
            return Err(LauncherError::Invalid {
                name: launcher.relative_path,
                reason: "launcher exceeds the bounded source size",
            });
        }
        if launcher.contents.to_ascii_lowercase().contains("python") {
            return Err(LauncherError::Invalid {
                name: launcher.relative_path,
                reason: "native launcher must not invoke Python",
            });
        }
        if launcher.contents.contains("cmd /c") || launcher.contents.contains("shell=true") {
            return Err(LauncherError::Invalid {
                name: launcher.relative_path,
                reason: "launcher must not construct a shell command string",
            });
        }
        if !launcher.contents.contains("Builder3D") {
            return Err(LauncherError::Invalid {
                name: launcher.relative_path,
                reason: "launcher has no Builder3D executable contract",
            });
        }
    }
    Ok(())
}

/// Stages fixed launchers into an explicit package directory.
pub fn stage_launchers(destination: &Path) -> Result<Vec<PathBuf>, LauncherError> {
    validate_launchers()?;
    let value = destination.to_string_lossy();
    if value.trim().is_empty() || value.len() > 4096 {
        return Err(LauncherError::InvalidPath(value.into_owned()));
    }
    fs::create_dir_all(destination)?;
    let mut written = Vec::with_capacity(launcher_specs().len());
    for launcher in launcher_specs() {
        let path = destination.join(launcher.relative_path);
        fs::write(&path, launcher.contents.as_bytes())?;
        mark_executable(&path)?;
        written.push(path);
    }
    Ok(written)
}

#[cfg(unix)]
fn mark_executable(path: &Path) -> Result<(), LauncherError> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions)?;
    Ok(())
}

#[cfg(not(unix))]
fn mark_executable(_path: &Path) -> Result<(), LauncherError> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_launchers_are_native_and_bounded() {
        assert!(validate_launchers().is_ok());
        assert_eq!(launcher_specs().len(), 5);
        assert!(
            launcher_specs()
                .iter()
                .all(|spec| spec.contents.len() < MAX_LAUNCHER_BYTES)
        );
    }

    #[test]
    fn launcher_sources_keep_arguments_as_vectors() {
        let specs = launcher_specs();
        assert!(specs[0].contents.contains("%*"));
        assert!(specs[1].contents.contains("@args"));
        assert!(specs[2].contents.contains("\"$@\""));
        assert!(specs[3].contents.contains("\"$@\""));
    }
}
