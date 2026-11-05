//! Application print staging and known-slicer handoff.
//!
//! The platform crate owns discovery and the constrained process launch.  This
//! module owns the scene-facing workflow: visible-object selection, temporary
//! staging, and the Bambu-compatible individual STL fallback.

use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use builder3d_core::{Scene, load_3mf, load_mesh};
use builder3d_platform::{SlicerError, SlicerInfo, SlicerLaunch, discover_slicers, launch_slicer};
use thiserror::Error;

use crate::{ExportError, export_scene_objects};

static STAGING_SEQUENCE: AtomicU64 = AtomicU64::new(0);
const MAX_STAGING_ATTEMPTS: u32 = 32;

/// The files and metadata prepared for one slicer handoff.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreparedPrint {
    /// Slicer that will receive the staged files.
    pub slicer: SlicerInfo,
    /// Files in launch order.
    pub paths: Vec<PathBuf>,
    /// Number of visible objects included in the handoff.
    pub objects_exported: usize,
    /// Export format used for the handoff (`3mf` or `stl`).
    pub format: String,
    /// Temporary directory retaining the staged files until launch completes.
    pub staging_dir: PathBuf,
    /// Source scene revision staged into these files.
    pub source_revision: u64,
    /// Visible object identities included in launch order.
    pub source_object_ids: Vec<String>,
    /// Native document generation that produced this handoff, when staged by
    /// the desktop worker. Standalone callers leave this unset.
    pub source_document_generation: Option<u64>,
}

/// Errors at the scene-to-slicer handoff boundary.
#[derive(Debug, Error)]
pub enum PrintHandoffError {
    /// The launch action was requested before staging a print.
    #[error("no print handoff has been prepared")]
    NoPreparedPrint,
    /// No visible scene object can be sent to the slicer.
    #[error("there are no visible objects to send to a slicer")]
    NoVisibleObjects,
    /// The user cancelled a staging operation before it could be handed off.
    #[error("print staging was cancelled")]
    Cancelled,
    /// The staged output could not be reopened as the requested format.
    #[error("staged print file validation failed for {path}: {reason}")]
    StagedFileInvalid { path: String, reason: String },
    /// Temporary staging could not be created or written.
    #[error("print staging I/O failed: {0}")]
    Io(#[from] std::io::Error),
    /// A routed export format rejected the scene or path.
    #[error(transparent)]
    Export(#[from] ExportError),
    /// Slicer discovery or launch rejected the request.
    #[error(transparent)]
    Slicer(#[from] SlicerError),
}

/// Returns the currently discovered known slicers.
#[must_use]
pub fn discover_print_slicers() -> Vec<SlicerInfo> {
    discover_slicers()
}

/// Stages all visible scene objects for a discovered slicer.
pub fn prepare_print(
    scene: &Scene,
    slicer: &SlicerInfo,
    temp_root: &Path,
) -> Result<PreparedPrint, PrintHandoffError> {
    prepare_print_with_hooks(scene, slicer, temp_root, || false, |_, _| {})
}

/// Stages and reopens the visible scene while reporting bounded progress and
/// checking cooperative cancellation between export and validation steps.
pub fn prepare_print_with_hooks<C, P>(
    scene: &Scene,
    slicer: &SlicerInfo,
    temp_root: &Path,
    mut is_cancelled: C,
    mut report: P,
) -> Result<PreparedPrint, PrintHandoffError>
where
    C: FnMut() -> bool,
    P: FnMut(u8, &str),
{
    let indices = visible_indices(scene)?;
    let source_object_ids = indices
        .iter()
        .map(|index| scene.objects()[*index].object_id().to_owned())
        .collect::<Vec<_>>();
    let source_revision = scene.revision();
    fs::create_dir_all(temp_root)?;
    let staging_dir = create_staging_dir(temp_root)?;
    let result = (|| {
        if is_cancelled() {
            return Err(PrintHandoffError::Cancelled);
        }
        if slicer.stl_fallback() {
            let mut paths = Vec::with_capacity(indices.len());
            for (position, index) in indices.iter().copied().enumerate() {
                if is_cancelled() {
                    return Err(PrintHandoffError::Cancelled);
                }
                let path = staging_dir.join(format!("Builder3D_Print_{:03}.stl", position + 1));
                let progress = 5.0 + 75.0 * position as f64 / indices.len() as f64;
                report(
                    progress.clamp(5.0, 80.0) as u8,
                    "Exporting Bambu-compatible STL files",
                );
                export_scene_objects(scene, &[index], &path)?;
                if is_cancelled() {
                    return Err(PrintHandoffError::Cancelled);
                }
                report(90, "Validating staged STL file");
                validate_staged_file(&path, "stl", &staging_dir)?;
                paths.push(path);
            }
            Ok(PreparedPrint {
                slicer: slicer.clone(),
                paths,
                objects_exported: indices.len(),
                format: "stl".to_owned(),
                staging_dir: staging_dir.clone(),
                source_revision,
                source_object_ids,
                source_document_generation: None,
            })
        } else {
            report(5, "Exporting visible objects to 3MF");
            let path = staging_dir.join("Builder3D_Print.3mf");
            export_scene_objects(scene, &indices, &path)?;
            if is_cancelled() {
                return Err(PrintHandoffError::Cancelled);
            }
            report(85, "Reopening and validating staged 3MF");
            validate_staged_file(&path, "3mf", &staging_dir)?;
            Ok(PreparedPrint {
                slicer: slicer.clone(),
                paths: vec![path],
                objects_exported: indices.len(),
                format: "3mf".to_owned(),
                staging_dir: staging_dir.clone(),
                source_revision,
                source_object_ids,
                source_document_generation: None,
            })
        }
    })();
    match result {
        Ok(prepared) => {
            report(100, "Staged files reopened successfully");
            Ok(prepared)
        }
        Err(error) => {
            let cleanup = fs::remove_dir_all(&staging_dir);
            match cleanup {
                Ok(()) => Err(error),
                Err(cleanup_error) if cleanup_error.kind() == std::io::ErrorKind::NotFound => {
                    Err(error)
                }
                Err(cleanup_error) => Err(PrintHandoffError::Io(cleanup_error)),
            }
        }
    }
}

/// Finds a discovered slicer by key or canonical path and stages the scene.
pub fn prepare_print_by_key(
    scene: &Scene,
    slicer_key: &str,
    temp_root: &Path,
) -> Result<PreparedPrint, PrintHandoffError> {
    prepare_print_by_key_with_hooks(scene, slicer_key, temp_root, || false, |_, _| {})
}

/// Discovers a known slicer and stages the scene with cancellation/progress
/// callbacks. Arbitrary executable paths are not accepted.
pub fn prepare_print_by_key_with_hooks<C, P>(
    scene: &Scene,
    slicer_key: &str,
    temp_root: &Path,
    is_cancelled: C,
    report: P,
) -> Result<PreparedPrint, PrintHandoffError>
where
    C: FnMut() -> bool,
    P: FnMut(u8, &str),
{
    let slicer = discover_print_slicers()
        .into_iter()
        .find(|candidate| {
            candidate.key == slicer_key || candidate.path.to_string_lossy() == slicer_key
        })
        .ok_or_else(|| SlicerError::UnknownSlicer(slicer_key.to_owned()))?;
    prepare_print_with_hooks(scene, &slicer, temp_root, is_cancelled, report)
}

/// Launches the prepared files through the constrained platform seam.
pub fn launch_prepared_print(prepared: &PreparedPrint) -> Result<SlicerLaunch, PrintHandoffError> {
    for path in &prepared.paths {
        validate_staged_file(path, &prepared.format, &prepared.staging_dir)?;
    }
    Ok(launch_slicer(&prepared.slicer, &prepared.paths)?)
}

/// Removes staged files when a prepared handoff is cancelled, stale, or
/// replaced before launch.
pub(crate) fn discard_prepared_print(prepared: &PreparedPrint) -> Result<(), std::io::Error> {
    match fs::remove_dir_all(&prepared.staging_dir) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn validate_staged_file(
    path: &Path,
    expected_extension: &str,
    staging_dir: &Path,
) -> Result<(), PrintHandoffError> {
    let invalid = |reason: String| PrintHandoffError::StagedFileInvalid {
        path: path.display().to_string(),
        reason,
    };
    if path.parent() != Some(staging_dir) {
        return Err(invalid(String::from(
            "file is outside the staging directory",
        )));
    }
    if !path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case(expected_extension))
    {
        return Err(invalid(format!("expected .{expected_extension} output")));
    }
    let metadata = fs::metadata(path)?;
    if !metadata.is_file() || metadata.len() == 0 {
        return Err(invalid(String::from("file is missing or empty")));
    }
    match expected_extension {
        "3mf" => {
            let reopened = load_3mf(path).map_err(|error| invalid(error.to_string()))?;
            if reopened.items.is_empty() {
                return Err(invalid(String::from("3MF contains no mesh objects")));
            }
        }
        "stl" => {
            let reopened = load_mesh(path).map_err(|error| invalid(error.to_string()))?;
            if reopened.is_empty() {
                return Err(invalid(String::from("STL contains no mesh geometry")));
            }
        }
        _ => return Err(invalid(String::from("unsupported staging format"))),
    }
    Ok(())
}

fn visible_indices(scene: &Scene) -> Result<Vec<usize>, PrintHandoffError> {
    let indices = (0..scene.objects().len())
        .filter(|index| scene.objects()[*index].is_visible())
        .collect::<Vec<_>>();
    if indices.is_empty() {
        return Err(PrintHandoffError::NoVisibleObjects);
    }
    Ok(indices)
}

fn create_staging_dir(temp_root: &Path) -> Result<PathBuf, std::io::Error> {
    for _ in 0..MAX_STAGING_ATTEMPTS {
        let sequence = STAGING_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = temp_root.join(format!(
            "builder3d-slicer-{}-{}",
            std::process::id(),
            sequence
        ));
        match fs::create_dir(&path) {
            Ok(()) => return Ok(path),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "could not allocate a unique print staging directory",
    ))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use builder3d_core::PrimitiveKind;

    fn scene_with_objects() -> Scene {
        let mut scene = Scene::new();
        assert!(scene.add_primitive(PrimitiveKind::Cube, None, None).is_ok());
        assert!(
            scene
                .add_primitive(PrimitiveKind::Sphere, None, None)
                .is_ok()
        );
        scene
    }

    fn test_root(label: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "builder3d-r7-print-{label}-{}",
            STAGING_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("test root");
        path
    }

    #[test]
    fn prepare_print_stages_visible_objects_as_one_3mf() {
        let scene = scene_with_objects();
        let root = test_root("3mf");
        let slicer = SlicerInfo {
            key: "prusa_slicer".to_owned(),
            label: "PrusaSlicer".to_owned(),
            path: PathBuf::from("PrusaSlicer.exe"),
        };
        let prepared = prepare_print(&scene, &slicer, &root).expect("prepare print");
        assert_eq!(prepared.objects_exported, 2);
        assert_eq!(prepared.format, "3mf");
        assert_eq!(prepared.paths.len(), 1);
        assert!(prepared.paths[0].is_file());
        assert!(fs::metadata(&prepared.paths[0]).is_ok_and(|metadata| metadata.len() > 0));
        assert!(load_3mf(&prepared.paths[0]).is_ok_and(|reopened| reopened.items.len() == 2));
        assert_eq!(
            prepared.paths[0]
                .extension()
                .and_then(|value| value.to_str()),
            Some("3mf")
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn prepare_print_uses_individual_stl_for_bambu() {
        let scene = scene_with_objects();
        let root = test_root("stl");
        let slicer = SlicerInfo {
            key: "bambu_studio".to_owned(),
            label: "Bambu Studio".to_owned(),
            path: PathBuf::from("BambuStudio.exe"),
        };
        let prepared = prepare_print(&scene, &slicer, &root).expect("prepare print");
        assert_eq!(prepared.objects_exported, 2);
        assert_eq!(prepared.format, "stl");
        assert_eq!(prepared.paths.len(), 2);
        assert!(prepared.paths.iter().all(|path| path.is_file()));
        assert!(prepared.paths.iter().all(|path| {
            fs::metadata(path).is_ok_and(|metadata| metadata.len() > 0)
                && load_mesh(path).is_ok_and(|mesh| !mesh.is_empty())
        }));
        assert_eq!(
            prepared.paths[0]
                .file_name()
                .and_then(|value| value.to_str()),
            Some("Builder3D_Print_001.stl")
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn prepare_print_rejects_empty_visible_scene() {
        let mut scene = scene_with_objects();
        assert!(scene.set_visible(0, false).is_ok());
        assert!(scene.set_visible(1, false).is_ok());
        let root = test_root("empty");
        let slicer = SlicerInfo {
            key: "prusa_slicer".to_owned(),
            label: "PrusaSlicer".to_owned(),
            path: PathBuf::from("PrusaSlicer.exe"),
        };
        assert!(matches!(
            prepare_print(&scene, &slicer, &root),
            Err(PrintHandoffError::NoVisibleObjects)
        ));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn cancelled_staging_removes_partial_output_and_can_be_retried() {
        let scene = scene_with_objects();
        let root = test_root("cancel");
        let slicer = SlicerInfo {
            key: "bambu_studio".to_owned(),
            label: "Bambu Studio".to_owned(),
            path: PathBuf::from("BambuStudio.exe"),
        };
        let cancelled = prepare_print_with_hooks(&scene, &slicer, &root, || true, |_, _| {});
        assert!(matches!(cancelled, Err(PrintHandoffError::Cancelled)));
        assert_eq!(
            fs::read_dir(&root).map(|entries| entries.count()).ok(),
            Some(0)
        );

        let prepared = prepare_print(&scene, &slicer, &root);
        assert!(prepared.is_ok());
        if let Ok(prepared) = prepared {
            assert_eq!(prepared.source_revision, scene.revision());
            assert_eq!(prepared.source_object_ids.len(), 2);
            assert!(discard_prepared_print(&prepared).is_ok());
        }
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn launch_reopens_staged_files_before_validating_the_slicer() {
        let scene = scene_with_objects();
        let root = test_root("revalidate");
        let slicer = SlicerInfo {
            key: "bambu_studio".to_owned(),
            label: "Bambu Studio".to_owned(),
            path: PathBuf::from("missing-bambu.exe"),
        };
        let prepared = prepare_print(&scene, &slicer, &root).expect("prepare print");
        assert!(
            prepared.paths.iter().all(|path| validate_staged_file(
                path,
                "stl",
                &prepared.staging_dir
            )
            .is_ok())
        );
        assert!(matches!(
            launch_prepared_print(&prepared),
            Err(PrintHandoffError::Slicer(SlicerError::MissingExecutable(_)))
        ));
        assert!(discard_prepared_print(&prepared).is_ok());
        let _ = fs::remove_dir_all(root);
    }
}
