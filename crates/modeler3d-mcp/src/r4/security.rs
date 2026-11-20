//! Approved-root filesystem policy for R4 file and project tools.

use std::{
    env, fs,
    path::{Path, PathBuf},
};

#[cfg(windows)]
use std::os::windows::fs::MetadataExt;

use thiserror::Error;

/// Filesystem policy failures returned to MCP callers as stable error codes.
#[derive(Debug, Error)]
pub enum SecurityError {
    /// The approved-root environment variable contained an invalid root.
    #[error("approved root is invalid: {0}")]
    InvalidRoot(String),
    /// No configured root contains the requested path.
    #[error("path is outside the configured approved roots: {0}")]
    OutsideRoots(String),
    /// The path traverses a symbolic link/reparse point or an unsafe parent.
    #[error("path contains a symbolic-link or reparse-point component: {0}")]
    ReparsePoint(String),
    /// A write would overwrite an existing file without explicit permission.
    #[error("refusing to overwrite an existing file without overwrite=true: {0}")]
    ExistingFile(String),
    /// A path could not be resolved under the policy.
    #[error("path resolution failed for {path}: {reason}")]
    Resolution { path: String, reason: String },
}

/// Canonical approved-root policy shared by all R4 file operations.
#[derive(Clone, Debug)]
pub struct ApprovedRoots {
    roots: Vec<PathBuf>,
}

impl ApprovedRoots {
    /// Loads semicolon/path-list separated roots from the product environment.
    /// An unset or empty variable deliberately means deny-all, matching the
    /// Python gateway's safe default.
    pub fn from_env() -> Result<Self, SecurityError> {
        let Some(raw) = env::var_os("MODELER3D_AI_APPROVED_ROOTS") else {
            return Ok(Self { roots: Vec::new() });
        };
        let roots = env::split_paths(&raw)
            .map(|path| path.to_path_buf())
            .collect::<Vec<_>>();
        Self::from_paths(roots)
    }

    /// Builds a policy from explicit roots for tests and embedding adapters.
    pub fn from_paths(paths: impl IntoIterator<Item = PathBuf>) -> Result<Self, SecurityError> {
        let mut roots = Vec::new();
        for path in paths {
            let canonical = canonical_directory(&path)?;
            reject_reparse_components(&path, &canonical)?;
            if roots.iter().any(|existing| existing == &canonical) {
                continue;
            }
            roots.push(canonical);
        }
        Ok(Self { roots })
    }

    /// Returns the canonical roots without exposing mutable policy state.
    #[must_use]
    pub fn roots(&self) -> &[PathBuf] {
        &self.roots
    }

    /// Returns whether a canonical candidate is inside an approved root.
    #[must_use]
    pub fn contains(&self, candidate: &Path) -> bool {
        self.roots
            .iter()
            .any(|root| candidate == root || candidate.starts_with(root))
    }

    /// Resolves an existing read path and rejects reparse-point components.
    pub fn read_path(&self, raw: impl AsRef<Path>) -> Result<PathBuf, SecurityError> {
        let raw = raw.as_ref();
        let canonical = canonical_existing(raw)?;
        self.ensure_allowed(&canonical)?;
        reject_reparse_components(raw, &canonical)?;
        Ok(canonical)
    }

    /// Resolves a write path.  The parent must already exist under an approved
    /// root; the leaf may be new. Existing files require explicit overwrite.
    pub fn write_path(
        &self,
        raw: impl AsRef<Path>,
        overwrite: bool,
    ) -> Result<PathBuf, SecurityError> {
        let raw = raw.as_ref();
        let parent = raw.parent().unwrap_or_else(|| Path::new("."));
        let canonical_parent = canonical_directory(parent)?;
        self.ensure_allowed(&canonical_parent)?;
        reject_reparse_components(parent, &canonical_parent)?;
        let file_name = raw.file_name().ok_or_else(|| SecurityError::Resolution {
            path: raw.display().to_string(),
            reason: "a file name is required".to_owned(),
        })?;
        let candidate = canonical_parent.join(file_name);
        if candidate.exists() {
            let canonical = canonical_existing(&candidate)?;
            self.ensure_allowed(&canonical)?;
            reject_reparse_components(raw, &canonical)?;
            if !overwrite {
                return Err(SecurityError::ExistingFile(canonical.display().to_string()));
            }
            return Ok(canonical);
        }
        Ok(candidate)
    }

    fn ensure_allowed(&self, candidate: &Path) -> Result<(), SecurityError> {
        if self.contains(candidate) {
            Ok(())
        } else {
            Err(SecurityError::OutsideRoots(candidate.display().to_string()))
        }
    }
}

fn canonical_directory(path: &Path) -> Result<PathBuf, SecurityError> {
    let canonical = fs::canonicalize(path).map_err(|error| SecurityError::Resolution {
        path: path.display().to_string(),
        reason: error.to_string(),
    })?;
    let metadata = fs::metadata(&canonical).map_err(|error| SecurityError::Resolution {
        path: canonical.display().to_string(),
        reason: error.to_string(),
    })?;
    if !metadata.is_dir() {
        return Err(SecurityError::InvalidRoot(canonical.display().to_string()));
    }
    Ok(canonical)
}

fn canonical_existing(path: &Path) -> Result<PathBuf, SecurityError> {
    fs::canonicalize(path).map_err(|error| SecurityError::Resolution {
        path: path.display().to_string(),
        reason: error.to_string(),
    })
}

fn reject_reparse_components(raw: &Path, canonical: &Path) -> Result<(), SecurityError> {
    let absolute = if raw.is_absolute() {
        raw.to_path_buf()
    } else {
        env::current_dir()
            .map_err(|error| SecurityError::Resolution {
                path: raw.display().to_string(),
                reason: error.to_string(),
            })?
            .join(raw)
    };
    let mut prefix = PathBuf::new();
    for component in absolute.components() {
        prefix.push(component.as_os_str());
        if let Ok(metadata) = fs::symlink_metadata(&prefix)
            && (metadata.file_type().is_symlink() || is_reparse_point(&metadata))
        {
            return Err(SecurityError::ReparsePoint(prefix.display().to_string()));
        }
    }
    if !canonical.exists() {
        return Err(SecurityError::Resolution {
            path: raw.display().to_string(),
            reason: "canonical target does not exist".to_owned(),
        });
    }
    Ok(())
}

fn is_reparse_point(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        metadata.file_attributes() & 0x0000_0400 != 0
    }
    #[cfg(not(windows))]
    {
        let _ = metadata;
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_allows_a_child_and_denies_a_sibling() {
        let root = tempfile_dir("modeler3d-r4-policy");
        let child = root.join("child");
        fs::create_dir_all(&child).expect("test directory");
        let policy = ApprovedRoots::from_paths([root.clone()]).expect("policy");
        assert!(policy.contains(&fs::canonicalize(&child).expect("child")));
        assert!(
            !policy.contains(&fs::canonicalize(crate::r4::test_temp_dir()).expect("temp"))
                || root == crate::r4::test_temp_dir()
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn path_policy_resolves_reads_and_requires_explicit_overwrite() {
        let root = tempfile_dir("modeler3d-r4-paths");
        let child = root.join("child");
        fs::create_dir_all(&child).expect("test directory");
        let existing = child.join("existing.stl");
        fs::write(&existing, b"mesh").expect("test file");
        let policy = ApprovedRoots::from_paths([root.clone()]).expect("policy");
        assert_eq!(
            policy.read_path(&existing).expect("read path"),
            fs::canonicalize(&existing).expect("canonical file")
        );
        assert!(matches!(
            policy.write_path(&existing, false),
            Err(SecurityError::ExistingFile(_))
        ));
        assert!(policy.write_path(child.join("new.stl"), false).is_ok());
        assert!(matches!(
            policy.write_path(
                crate::r4::test_temp_dir().join("outside-modeler3d-r4.stl"),
                false
            ),
            Err(SecurityError::OutsideRoots(_))
        ));
        let _ = fs::remove_dir_all(root);
    }

    fn tempfile_dir(prefix: &str) -> PathBuf {
        let path = crate::r4::test_temp_dir().join(format!("{prefix}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("temporary directory");
        path
    }
}
