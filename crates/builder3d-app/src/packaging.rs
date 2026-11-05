//! Native package staging, installer templates, and size-gate measurement.

use std::{
    fs::{self, File},
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

use serde::Serialize;
use sha2::{Digest, Sha256};
use thiserror::Error;
use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

use crate::launchers::{LauncherError, stage_launchers};

/// Maximum installed package bytes allowed by the rewrite plan.
pub const MAX_INSTALLED_BYTES: u64 = 40 * 1024 * 1024;
/// Maximum compressed private installer/package bytes.
pub const MAX_COMPRESSED_BYTES: u64 = 15 * 1024 * 1024;
const MAX_PACKAGE_FILES: usize = 512;
const MAX_PACKAGE_DEPTH: usize = 16;
const MAX_FILE_BYTES: u64 = MAX_INSTALLED_BYTES;
const MAX_PATH_LENGTH: usize = 4096;
const MANIFEST_NAME: &str = "PACKAGE-MANIFEST.json";

/// A fixed installer template embedded in the native package.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InstallerSpec {
    /// Relative installer file name.
    pub relative_path: &'static str,
    /// Installer source.
    pub contents: &'static str,
}

/// One measured package member.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PackageEntry {
    /// Slash-separated package-relative path.
    pub path: String,
    /// File length in bytes.
    pub bytes: u64,
    /// SHA-256 digest in lowercase hexadecimal.
    pub sha256: String,
}

/// Installed package size measurement.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct InstalledMeasurement {
    /// Number of regular files measured.
    pub file_count: usize,
    /// Aggregate installed bytes.
    pub installed_bytes: u64,
    /// Whether the 40 MiB gate passed.
    pub within_40_mib_gate: bool,
}

/// Result returned after staging a package and compressed archive.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PackageMeasurement {
    /// Package root used for measurement.
    pub package_root: PathBuf,
    /// Compressed archive path.
    pub archive_path: PathBuf,
    /// Installed package measurement.
    pub installed: InstalledMeasurement,
    /// Compressed archive size.
    pub compressed_bytes: u64,
    /// Whether the compressed size policy passed.
    pub within_15_mib_archive_gate: bool,
    /// Entries listed in the manifest, excluding the manifest itself.
    pub entries: Vec<PackageEntry>,
}

/// Typed failures from package staging and policy checks.
#[derive(Debug, Error)]
pub enum PackagingError {
    /// Filesystem access failed.
    #[error("package I/O failed: {0}")]
    Io(#[from] io::Error),
    /// Launcher validation failed.
    #[error(transparent)]
    Launcher(#[from] LauncherError),
    /// Original visual resources could not be staged.
    #[error(transparent)]
    Artwork(#[from] crate::artwork::ArtworkError),
    /// JSON manifest serialization failed.
    #[error("package manifest failed: {0}")]
    Json(#[from] serde_json::Error),
    /// ZIP archive writing failed.
    #[error("package archive failed: {0}")]
    Archive(#[from] zip::result::ZipError),
    /// A package path or layout violated the bounded contract.
    #[error("package path is invalid: {0}")]
    InvalidPath(String),
    /// A destination already contains files that staging could overwrite.
    #[error("package destination must be empty: {0}")]
    DestinationNotEmpty(String),
    /// The installed-size gate failed.
    #[error("installed package is {actual} bytes, above the {limit}-byte gate")]
    InstalledSize { actual: u64, limit: u64 },
    /// The compressed-size gate failed.
    #[error("compressed package is {actual} bytes, above the {limit}-byte gate")]
    CompressedSize { actual: u64, limit: u64 },
    /// A package member was not a regular file.
    #[error("package member is not a regular file: {0}")]
    NonRegularFile(String),
    /// A package contained too many files or nested directories.
    #[error("package layout exceeds bounded resource limits")]
    LayoutLimit,
}

/// Returns all platform installer templates.
#[must_use]
pub const fn installer_specs() -> [InstallerSpec; 6] {
    [
        InstallerSpec {
            relative_path: "installers/install_windows.ps1",
            contents: include_str!("../../../packaging/installers/install_windows.ps1"),
        },
        InstallerSpec {
            relative_path: "installers/uninstall_windows.ps1",
            contents: include_str!("../../../packaging/installers/uninstall_windows.ps1"),
        },
        InstallerSpec {
            relative_path: "installers/install_linux.sh",
            contents: include_str!("../../../packaging/installers/install_linux.sh"),
        },
        InstallerSpec {
            relative_path: "installers/uninstall_linux.sh",
            contents: include_str!("../../../packaging/installers/uninstall_linux.sh"),
        },
        InstallerSpec {
            relative_path: "installers/install_macos.command",
            contents: include_str!("../../../packaging/installers/install_macos.command"),
        },
        InstallerSpec {
            relative_path: "installers/uninstall_macos.command",
            contents: include_str!("../../../packaging/installers/uninstall_macos.command"),
        },
    ]
}

/// Validates installer templates without executing destructive operations.
pub fn validate_installers() -> Result<(), PackagingError> {
    for installer in installer_specs() {
        if installer.contents.len() > 16 * 1024 {
            return Err(PackagingError::LayoutLimit);
        }
        if installer.contents.to_ascii_lowercase().contains("python") {
            return Err(PackagingError::InvalidPath(format!(
                "{} invokes Python",
                installer.relative_path
            )));
        }
        if installer.contents.contains("rm -rf") && !installer.contents.contains("expected_root") {
            return Err(PackagingError::InvalidPath(format!(
                "{} has no private removal-root guard",
                installer.relative_path
            )));
        }
    }
    Ok(())
}

/// Stages all fixed installers into a package root.
pub fn stage_installers(package_root: &Path) -> Result<Vec<PathBuf>, PackagingError> {
    validate_installers()?;
    let mut paths = Vec::with_capacity(installer_specs().len());
    for installer in installer_specs() {
        let path = package_root.join(installer.relative_path);
        let parent = path
            .parent()
            .ok_or_else(|| PackagingError::InvalidPath(path.display().to_string()))?;
        fs::create_dir_all(parent)?;
        fs::write(&path, installer.contents.as_bytes())?;
        mark_executable(&path)?;
        paths.push(path);
    }
    Ok(paths)
}

#[cfg(unix)]
fn mark_executable(path: &Path) -> Result<(), PackagingError> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions)?;
    Ok(())
}

#[cfg(not(unix))]
fn mark_executable(_path: &Path) -> Result<(), PackagingError> {
    Ok(())
}

/// Measures a package directory and enforces the installed-size gate.
pub fn measure_installed(root: &Path) -> Result<InstalledMeasurement, PackagingError> {
    validate_root(root)?;
    let entries = collect_entries(root)?;
    let installed_bytes = entries.iter().try_fold(0_u64, |total, entry| {
        total
            .checked_add(entry.bytes)
            .ok_or(PackagingError::LayoutLimit)
    })?;
    if installed_bytes > MAX_INSTALLED_BYTES {
        return Err(PackagingError::InstalledSize {
            actual: installed_bytes,
            limit: MAX_INSTALLED_BYTES,
        });
    }
    Ok(InstalledMeasurement {
        file_count: entries.len(),
        installed_bytes,
        within_40_mib_gate: true,
    })
}

/// Stages a native executable, launchers, installers, manifest, and ZIP.
pub fn stage_package(
    executable: &Path,
    destination: &Path,
) -> Result<PackageMeasurement, PackagingError> {
    validate_root(destination)?;
    if !executable.is_file() {
        return Err(PackagingError::NonRegularFile(
            executable.display().to_string(),
        ));
    }
    if destination.exists() && fs::read_dir(destination)?.next().transpose()?.is_some() {
        return Err(PackagingError::DestinationNotEmpty(
            destination.display().to_string(),
        ));
    }
    fs::create_dir_all(destination)?;
    let native_name = if cfg!(windows) {
        "Builder3D.exe"
    } else {
        "Builder3D"
    };
    fs::copy(executable, destination.join(native_name))?;
    crate::artwork::stage_original_artwork(destination)?;
    stage_launchers(destination)?;
    stage_installers(destination)?;
    let licenses = destination.join("licenses");
    fs::create_dir_all(&licenses)?;
    for (name, text) in [
        (
            "BOOST-LICENSE-1.0.txt",
            include_str!("../../../packaging/licenses/BOOST-LICENSE-1.0.txt"),
        ),
        (
            "CLIPBOARD-NOTICE.txt",
            include_str!("../../../packaging/licenses/CLIPBOARD-NOTICE.txt"),
        ),
        (
            "QHULL-COPYING.txt",
            include_str!("../../../packaging/licenses/QHULL-COPYING.txt"),
        ),
        (
            "QHULL-RS-LICENSE.txt",
            include_str!("../../../packaging/licenses/QHULL-RS-LICENSE.txt"),
        ),
        (
            "QHULL-NOTICE.txt",
            include_str!("../../../packaging/licenses/QHULL-NOTICE.txt"),
        ),
    ] {
        fs::write(licenses.join(name), text)?;
    }

    let entries = collect_entries(destination)?;
    let manifest = serde_json::json!({
        "format": "Builder3D native package",
        "manifest_excluded_from_entries": true,
        "installed_gate_bytes": MAX_INSTALLED_BYTES,
        "compressed_gate_bytes": MAX_COMPRESSED_BYTES,
        "entries": entries,
    });
    fs::write(
        destination.join(MANIFEST_NAME),
        serde_json::to_vec_pretty(&manifest)?,
    )?;
    let installed = measure_installed(destination)?;
    let archive_path = destination.with_extension("zip");
    let compressed_bytes = write_archive(destination, &archive_path)?;
    if compressed_bytes > MAX_COMPRESSED_BYTES {
        return Err(PackagingError::CompressedSize {
            actual: compressed_bytes,
            limit: MAX_COMPRESSED_BYTES,
        });
    }
    Ok(PackageMeasurement {
        package_root: destination.to_path_buf(),
        archive_path,
        installed,
        compressed_bytes,
        within_15_mib_archive_gate: true,
        entries,
    })
}

fn validate_root(root: &Path) -> Result<(), PackagingError> {
    let value = root.to_string_lossy();
    if value.trim().is_empty() || value.len() > MAX_PATH_LENGTH {
        return Err(PackagingError::InvalidPath(value.into_owned()));
    }
    Ok(())
}

fn collect_entries(root: &Path) -> Result<Vec<PackageEntry>, PackagingError> {
    let mut entries = Vec::new();
    collect_entries_recursive(root, root, 0, &mut entries)?;
    entries.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(entries)
}

fn collect_entries_recursive(
    root: &Path,
    current: &Path,
    depth: usize,
    entries: &mut Vec<PackageEntry>,
) -> Result<(), PackagingError> {
    if depth > MAX_PACKAGE_DEPTH || entries.len() > MAX_PACKAGE_FILES {
        return Err(PackagingError::LayoutLimit);
    }
    for item in fs::read_dir(current)? {
        let item = item?;
        let file_type = item.file_type()?;
        let path = item.path();
        if file_type.is_symlink() {
            return Err(PackagingError::NonRegularFile(path.display().to_string()));
        }
        if file_type.is_dir() {
            collect_entries_recursive(root, &path, depth.saturating_add(1), entries)?;
            continue;
        }
        if !file_type.is_file() {
            return Err(PackagingError::NonRegularFile(path.display().to_string()));
        }
        if entries.len() >= MAX_PACKAGE_FILES {
            return Err(PackagingError::LayoutLimit);
        }
        let metadata = fs::metadata(&path)?;
        if metadata.len() > MAX_FILE_BYTES {
            return Err(PackagingError::LayoutLimit);
        }
        let relative = path
            .strip_prefix(root)
            .map_err(|_| PackagingError::InvalidPath(path.display().to_string()))?
            .to_string_lossy()
            .replace('\\', "/");
        entries.push(PackageEntry {
            path: relative,
            bytes: metadata.len(),
            sha256: hash_file(&path)?,
        });
    }
    Ok(())
}

fn hash_file(path: &Path) -> Result<String, PackagingError> {
    let mut file = File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn write_archive(root: &Path, archive_path: &Path) -> Result<u64, PackagingError> {
    let file = File::create(archive_path)?;
    let mut writer = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let entries = collect_entries(root)?;
    for entry in entries {
        let mut member_path = root.to_path_buf();
        for component in entry.path.split('/') {
            member_path.push(component);
        }
        let mut data = File::open(member_path)?;
        writer.start_file(entry.path, options)?;
        io::copy(&mut data, &mut writer)?;
    }
    let mut file = writer.finish()?;
    file.flush()?;
    Ok(file.metadata()?.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installer_templates_are_bounded_and_native() {
        assert!(validate_installers().is_ok());
        assert_eq!(installer_specs().len(), 6);
        assert!(
            installer_specs()
                .iter()
                .all(|spec| !spec.contents.contains("python"))
        );
    }

    #[test]
    fn archive_and_installed_limits_are_explicit() {
        assert_eq!(MAX_INSTALLED_BYTES, 40 * 1024 * 1024);
        assert_eq!(MAX_COMPRESSED_BYTES, 15 * 1024 * 1024);
    }
}
