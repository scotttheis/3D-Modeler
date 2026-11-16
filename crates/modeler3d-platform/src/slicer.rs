//! Known-slicer discovery and constrained process handoff.
//!
//! This module is deliberately independent of MCP and the desktop shell.  It
//! discovers only the product's declared slicer definitions and launches only
//! an executable returned by that discovery.  It is not a general command
//! runner.

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use serde::{Deserialize, Serialize};
use thiserror::Error;

const MAX_MODEL_PATHS: usize = 64;
const MAX_PATH_LENGTH: usize = 4096;

#[derive(Clone, Copy, Debug)]
struct SlicerDefinition {
    key: &'static str,
    label: &'static str,
    executable_names: &'static [&'static str],
    windows_directories: &'static [(&'static str, &'static str)],
    mac_directories: &'static [&'static str],
    linux_directories: &'static [&'static str],
}

const SLICER_DEFINITIONS: [SlicerDefinition; 6] = [
    SlicerDefinition {
        key: "prusa_slicer",
        label: "PrusaSlicer",
        executable_names: &["prusa-slicer.exe", "PrusaSlicer.exe", "prusa-slicer"],
        windows_directories: &[
            ("PROGRAMFILES", "Prusa3D/PrusaSlicer"),
            ("PROGRAMFILES", "PrusaSlicer*"),
            ("PROGRAMFILES(X86)", "Prusa3D/PrusaSlicer"),
            ("LOCALAPPDATA", "PrusaSlicer"),
            ("LOCALAPPDATA", "Programs/PrusaSlicer*"),
        ],
        mac_directories: &[
            "/Applications/PrusaSlicer.app/Contents/MacOS",
            "~/Applications/PrusaSlicer.app/Contents/MacOS",
        ],
        linux_directories: &["/usr/bin", "/usr/local/bin"],
    },
    SlicerDefinition {
        key: "orca_slicer",
        label: "OrcaSlicer",
        executable_names: &["orca-slicer.exe", "OrcaSlicer.exe", "orca-slicer"],
        windows_directories: &[
            ("PROGRAMFILES", "OrcaSlicer"),
            ("PROGRAMFILES(X86)", "OrcaSlicer"),
            ("LOCALAPPDATA", "OrcaSlicer"),
            ("LOCALAPPDATA", "Programs/OrcaSlicer*"),
        ],
        mac_directories: &[
            "/Applications/OrcaSlicer.app/Contents/MacOS",
            "~/Applications/OrcaSlicer.app/Contents/MacOS",
        ],
        linux_directories: &["/usr/bin", "/usr/local/bin"],
    },
    SlicerDefinition {
        key: "bambu_studio",
        label: "Bambu Studio",
        executable_names: &["bambu-studio.exe", "BambuStudio.exe", "bambu-studio"],
        windows_directories: &[
            ("PROGRAMFILES", "Bambu Studio"),
            ("PROGRAMFILES(X86)", "Bambu Studio"),
            ("LOCALAPPDATA", "BambuStudio"),
            ("LOCALAPPDATA", "Programs/BambuStudio*"),
        ],
        mac_directories: &[
            "/Applications/BambuStudio.app/Contents/MacOS",
            "~/Applications/BambuStudio.app/Contents/MacOS",
        ],
        linux_directories: &["/usr/bin", "/usr/local/bin"],
    },
    SlicerDefinition {
        key: "ultimaker_cura",
        label: "UltiMaker Cura",
        executable_names: &[
            "UltiMaker-Cura.exe",
            "Ultimaker-Cura.exe",
            "Cura.exe",
            "cura",
        ],
        windows_directories: &[
            ("PROGRAMFILES", "UltiMaker Cura*"),
            ("PROGRAMFILES(X86)", "UltiMaker Cura*"),
            ("LOCALAPPDATA", "Programs/UltiMaker Cura*"),
            ("LOCALAPPDATA", "Ultimaker Cura*"),
        ],
        mac_directories: &[
            "/Applications/UltiMaker Cura.app/Contents/MacOS",
            "~/Applications/UltiMaker Cura.app/Contents/MacOS",
        ],
        linux_directories: &["/usr/bin", "/usr/local/bin"],
    },
    SlicerDefinition {
        key: "superslicer",
        label: "SuperSlicer",
        executable_names: &["superslicer.exe", "SuperSlicer.exe", "superslicer"],
        windows_directories: &[
            ("PROGRAMFILES", "SuperSlicer"),
            ("PROGRAMFILES(X86)", "SuperSlicer"),
            ("LOCALAPPDATA", "SuperSlicer"),
        ],
        mac_directories: &[
            "/Applications/SuperSlicer.app/Contents/MacOS",
            "~/Applications/SuperSlicer.app/Contents/MacOS",
        ],
        linux_directories: &["/usr/bin", "/usr/local/bin"],
    },
    SlicerDefinition {
        key: "ideamaker",
        label: "ideaMaker",
        executable_names: &["ideaMaker.exe", "ideamaker.exe", "ideaMaker"],
        windows_directories: &[
            ("PROGRAMFILES", "ideaMaker"),
            ("PROGRAMFILES(X86)", "ideaMaker"),
            ("LOCALAPPDATA", "ideaMaker"),
        ],
        mac_directories: &[
            "/Applications/ideaMaker.app/Contents/MacOS",
            "~/Applications/ideaMaker.app/Contents/MacOS",
        ],
        linux_directories: &["/usr/bin", "/usr/local/bin"],
    },
];

/// A known or discovered slicer executable.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SlicerInfo {
    /// Stable catalog key.
    pub key: String,
    /// User-facing label.
    pub label: String,
    /// Canonical executable path.
    pub path: PathBuf,
}

impl SlicerInfo {
    /// Whether the handoff should use individual STL files for this slicer.
    #[must_use]
    pub fn stl_fallback(&self) -> bool {
        let executable = self
            .path
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase()
            .replace(['_', ' '], "-");
        self.key == "bambu_studio"
            || executable == "bambu-studio"
            || (executable.contains("bambu") && executable.contains("studio"))
    }
}

/// Validates a user-selected slicer file and returns its stable handoff record.
/// A saved path is not launched automatically; this record is only used after
/// the user starts a print handoff and the launch seam revalidates the file.
pub fn slicer_info_from_path(
    path: &Path,
    preferred_label: Option<&str>,
) -> Result<SlicerInfo, SlicerError> {
    let executable = canonical_file(path)
        .ok_or_else(|| SlicerError::MissingExecutable(path.display().to_string()))?;
    if let Some(known) = discover_slicers()
        .into_iter()
        .find(|candidate| candidate.path == executable)
    {
        return Ok(known);
    }
    let stem = executable
        .file_stem()
        .and_then(|name| name.to_str())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or("Custom Slicer");
    let label = preferred_label
        .map(str::trim)
        .filter(|label| !label.is_empty())
        .unwrap_or(stem)
        .to_owned();
    let key = executable.to_string_lossy().into_owned();
    Ok(SlicerInfo {
        key,
        label,
        path: executable,
    })
}

/// A bounded process-handoff result.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SlicerLaunch {
    /// Slicer selected from the discovered set.
    pub slicer: SlicerInfo,
    /// Operating-system process identifier, when the spawn succeeded.
    pub process_id: u32,
}

/// Safe failures from known-slicer launch.
#[derive(Debug, Error)]
pub enum SlicerError {
    /// The requested key was not present in the current discovery result.
    #[error("slicer_key must identify a discovered slicer: {0}")]
    UnknownSlicer(String),
    /// No exported model was supplied.
    #[error("at least one exported model is required")]
    NoModels,
    /// The model list exceeded the handoff bound.
    #[error("slicer handoff is limited to {0} model files")]
    TooManyModels(usize),
    /// One model path was malformed or too long.
    #[error("invalid model path: {0}")]
    InvalidModel(String),
    /// A staged model disappeared before launch.
    #[error("exported model was not found: {0}")]
    MissingModel(String),
    /// The discovered executable disappeared or was not a file.
    #[error("slicer executable was not found: {0}")]
    MissingExecutable(String),
    /// The operating system refused to start the known executable.
    #[error("slicer process could not be started: {0}")]
    Spawn(String),
}

/// Discovers the declared slicers using the current host environment.
#[must_use]
pub fn discover_slicers() -> Vec<SlicerInfo> {
    let environment = env::vars().collect::<Vec<_>>();
    discover_slicers_with(&environment, env::consts::OS, |name| {
        find_on_path(name, &environment)
    })
}

/// Launches a currently discovered slicer with already-staged model files.
/// The caller cannot provide an arbitrary executable path.
pub fn launch_discovered_slicer(
    slicer_key: &str,
    model_paths: &[PathBuf],
) -> Result<SlicerLaunch, SlicerError> {
    let slicers = discover_slicers();
    let slicer = slicers
        .iter()
        .find(|slicer| slicer.key == slicer_key || slicer.path.to_string_lossy() == slicer_key)
        .ok_or_else(|| SlicerError::UnknownSlicer(slicer_key.to_owned()))?;
    launch_slicer(slicer, model_paths)
}

/// Launches the supplied executable only after revalidating it as a file.
pub fn launch_slicer(
    slicer: &SlicerInfo,
    model_paths: &[PathBuf],
) -> Result<SlicerLaunch, SlicerError> {
    if model_paths.is_empty() {
        return Err(SlicerError::NoModels);
    }
    if model_paths.len() > MAX_MODEL_PATHS {
        return Err(SlicerError::TooManyModels(MAX_MODEL_PATHS));
    }
    let executable = canonical_file(&slicer.path)
        .ok_or_else(|| SlicerError::MissingExecutable(slicer.path.display().to_string()))?;
    let mut models = Vec::with_capacity(model_paths.len());
    for path in model_paths {
        let display = path.display().to_string();
        if display.len() > MAX_PATH_LENGTH || display.is_empty() {
            return Err(SlicerError::InvalidModel(display));
        }
        let model =
            canonical_file(path).ok_or_else(|| SlicerError::MissingModel(display.clone()))?;
        models.push(model);
    }
    let process_id = Command::new(&executable)
        .args(&models)
        .current_dir(executable.parent().unwrap_or_else(|| Path::new(".")))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| SlicerError::Spawn(error.to_string()))?
        .id();
    Ok(SlicerLaunch {
        slicer: SlicerInfo {
            key: slicer.key.clone(),
            label: slicer.label.clone(),
            path: executable,
        },
        process_id,
    })
}

fn discover_slicers_with<F>(
    environment: &[(String, String)],
    platform_name: &str,
    mut which: F,
) -> Vec<SlicerInfo>
where
    F: FnMut(&str) -> Option<PathBuf>,
{
    let mut results = Vec::new();
    let mut seen_paths = Vec::new();
    for definition in SLICER_DEFINITIONS {
        let mut candidates = Vec::new();
        for name in definition.executable_names {
            if let Some(path) = which(name) {
                candidates.push(path);
            }
        }
        for directory in candidate_directories(definition, environment, platform_name) {
            candidates.extend(
                definition
                    .executable_names
                    .iter()
                    .map(|name| directory.join(name)),
            );
        }
        for candidate in candidates {
            let Some(path) = canonical_file(&candidate) else {
                continue;
            };
            if seen_paths.iter().any(|seen| seen == &path) {
                continue;
            }
            seen_paths.push(path.clone());
            results.push(SlicerInfo {
                key: definition.key.to_owned(),
                label: definition.label.to_owned(),
                path,
            });
            break;
        }
    }
    results
}

fn find_on_path(name: &str, environment: &[(String, String)]) -> Option<PathBuf> {
    let raw_path = environment_value(environment, "PATH")?;
    env::split_paths(Path::new(&raw_path).as_os_str())
        .map(|directory| directory.join(name))
        .find(|candidate| canonical_file(candidate).is_some())
}

fn candidate_directories(
    definition: SlicerDefinition,
    environment: &[(String, String)],
    platform_name: &str,
) -> Vec<PathBuf> {
    if is_windows(platform_name) {
        definition
            .windows_directories
            .iter()
            .filter_map(|(root_name, relative)| {
                environment_value(environment, root_name)
                    .map(|root| PathBuf::from(root).join(relative))
            })
            .flat_map(|path| expand_pattern(&path))
            .collect()
    } else if matches!(platform_name, "darwin" | "macos" | "osx") {
        definition
            .mac_directories
            .iter()
            .flat_map(|path| expand_pattern(&expand_user(path)))
            .collect()
    } else {
        definition
            .linux_directories
            .iter()
            .flat_map(|path| expand_pattern(Path::new(path)))
            .collect()
    }
}

fn environment_value(environment: &[(String, String)], wanted: &str) -> Option<String> {
    environment
        .iter()
        .find(|(name, value)| name.eq_ignore_ascii_case(wanted) && !value.is_empty())
        .map(|(_, value)| value.clone())
}

fn expand_user(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/")
        && let Some(home) = env::var_os("HOME").or_else(|| env::var_os("USERPROFILE"))
    {
        return PathBuf::from(home).join(rest);
    }
    PathBuf::from(path)
}

fn expand_pattern(path: &Path) -> Vec<PathBuf> {
    let text = path.to_string_lossy();
    if !text.contains('*') && !text.contains('?') {
        return vec![path.to_path_buf()];
    }
    let Some(parent) = path.parent() else {
        return Vec::new();
    };
    let Some(pattern) = path.file_name().and_then(|name| name.to_str()) else {
        return Vec::new();
    };
    let Ok(entries) = fs::read_dir(parent) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name();
            let name = name.to_str()?;
            wildcard_match(name, pattern).then_some(entry.path())
        })
        .collect()
}

fn wildcard_match(value: &str, pattern: &str) -> bool {
    let value = value.as_bytes();
    let pattern = pattern.as_bytes();
    let mut table = vec![vec![false; pattern.len() + 1]; value.len() + 1];
    table[0][0] = true;
    for pattern_index in 0..pattern.len() {
        if pattern[pattern_index] == b'*' && table[0][pattern_index] {
            table[0][pattern_index + 1] = true;
        }
    }
    for value_index in 0..value.len() {
        for pattern_index in 0..pattern.len() {
            if pattern[pattern_index] == b'*' {
                table[value_index + 1][pattern_index + 1] =
                    table[value_index + 1][pattern_index] || table[value_index][pattern_index + 1];
            } else if pattern[pattern_index] == b'?'
                || equal_path_byte(value[value_index], pattern[pattern_index])
            {
                table[value_index + 1][pattern_index + 1] = table[value_index][pattern_index];
            }
        }
    }
    table[value.len()][pattern.len()]
}

fn equal_path_byte(left: u8, right: u8) -> bool {
    if cfg!(windows) {
        left.eq_ignore_ascii_case(&right)
    } else {
        left == right
    }
}

fn is_windows(platform_name: &str) -> bool {
    platform_name == "windows"
        || platform_name == "win32"
        || platform_name == "nt"
        || platform_name.starts_with("win")
}

fn canonical_file(path: &Path) -> Option<PathBuf> {
    if path.as_os_str().is_empty() || path.to_string_lossy().len() > MAX_PATH_LENGTH {
        return None;
    }
    let metadata = fs::metadata(path).ok()?;
    if !metadata.is_file() {
        return None;
    }
    fs::canonicalize(path).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovery_uses_path_and_marks_bambu_stl_fallback() {
        let root = env::temp_dir().join(format!("modeler3d-slicer-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("test root");
        let executable = root.join("BambuStudio.exe");
        fs::write(&executable, b"test executable").expect("test executable");
        let environment = vec![("PATH".to_owned(), root.display().to_string())];
        let found = discover_slicers_with(&environment, "windows", |name| {
            let candidate = root.join(name);
            candidate.is_file().then_some(candidate)
        });
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].key, "bambu_studio");
        assert!(found[0].stl_fallback());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn unknown_slicer_and_missing_models_are_typed_errors() {
        let error = launch_discovered_slicer("missing", &[]).expect_err("unknown slicer");
        assert!(matches!(error, SlicerError::UnknownSlicer(_)));
        let info = SlicerInfo {
            key: "test".to_owned(),
            label: "Test".to_owned(),
            path: PathBuf::from("missing.exe"),
        };
        let error = launch_slicer(&info, &[]).expect_err("no models");
        assert!(matches!(error, SlicerError::NoModels));
    }

    #[test]
    fn manually_selected_slicer_paths_are_canonical_and_labeled_without_launching() {
        let root = env::temp_dir().join(format!("modeler3d-custom-slicer-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("slicer fixture directory");
        let executable = root.join("ExampleSlicer.exe");
        fs::write(&executable, b"not executed by this test").expect("slicer fixture");

        let slicer = slicer_info_from_path(&executable, Some("Example Slicer"))
            .expect("validated custom slicer record");

        assert_eq!(slicer.key, slicer.path.to_string_lossy().into_owned());
        assert_eq!(slicer.label, "Example Slicer");
        assert!(slicer.path.is_absolute());
        assert!(!slicer.stl_fallback());
        assert!(matches!(
            slicer_info_from_path(&root.join("missing.exe"), None),
            Err(SlicerError::MissingExecutable(_))
        ));
        assert!(fs::remove_dir_all(root).is_ok());
    }
}
