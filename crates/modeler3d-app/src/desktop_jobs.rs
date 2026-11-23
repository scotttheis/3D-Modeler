//! Isolated work used by the native shell's background operation lifecycle.
//!
//! Workers receive owned mesh or scene snapshots. They never mutate the live
//! command state; `shell` validates and commits results on the UI thread.

use std::path::PathBuf;

use modeler3d_core::{
    BooleanResult, CoreError, Mesh, MeshOperationLimits, PreparedBoolean, RepairEngine,
    RepairRouteControl, Scene, SceneSnapshot, repair_mesh_routed, simplify_mesh_by_percent,
    smooth_mesh, subdivide_mesh_even,
};
use modeler3d_platform::SlicerInfo;

use crate::{
    OperationHandle, PreparedPrint, PrintPrepReport,
    print_handoff::{discard_prepared_print, prepare_print_with_hooks},
    print_prep::{
        PrintPrepError, check_bed_fit, check_empty_hidden, check_non_manifold, check_scale,
    },
};

/// A core mesh operation accepted by the native worker.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum MeshJobOperation {
    Repair,
    /// Remove this percentage of the faces (OF-51).
    Simplify(u32),
    Subdivide,
    Smooth,
}

impl MeshJobOperation {
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Repair => "Repair",
            Self::Simplify(_) => "Simplify",
            Self::Subdivide => "Subdivide",
            Self::Smooth => "Smooth",
        }
    }
}

/// Owned work payload. Scene state is copied before the worker is started.
pub(crate) enum DesktopJobRequest {
    Mesh {
        operation: MeshJobOperation,
        mesh: Mesh,
    },
    PrintPrep {
        snapshot: SceneSnapshot,
        bed_size_mm: f32,
    },
    PreparePrint {
        snapshot: SceneSnapshot,
        slicer: SlicerInfo,
        temp_root: PathBuf,
        document_generation: u64,
    },
    /// Merge, Subtract or Intersect on owned copies of the two objects.
    Boolean {
        prepared: PreparedBoolean,
        clear_selection: bool,
    },
}

/// Result that must be applied or retained by the UI thread.
pub(crate) enum DesktopJobResult {
    Mesh {
        operation: MeshJobOperation,
        mesh: Mesh,
        message: String,
    },
    PrintPrep {
        bed_size_mm: f32,
        report: PrintPrepReport,
    },
    PreparedPrint(PreparedPrintGuard),
    /// A computed boolean, applied on the UI thread.
    Boolean {
        prepared: Box<PreparedBoolean>,
        result: Box<BooleanResult>,
        clear_selection: bool,
    },
    /// The operation found nothing to do; the scene is left untouched.
    Unchanged {
        message: String,
    },
}

/// Shown when Smooth finds only flat faces, sharp edges and corners, which it
/// keeps (H1 / D9), so the shape is left as it is.
pub(crate) const SMOOTH_NOTHING_TO_SMOOTH_MESSAGE: &str =
    "Smooth: nothing to smooth. Flat faces, sharp edges and corners are kept";

/// Owns staged output until the UI accepts it. Channel teardown, cancellation,
/// and stale results automatically remove unclaimed files.
pub(crate) struct PreparedPrintGuard {
    prepared: Option<PreparedPrint>,
}

impl PreparedPrintGuard {
    fn new(prepared: PreparedPrint) -> Self {
        Self {
            prepared: Some(prepared),
        }
    }

    pub(crate) fn into_prepared(mut self) -> Option<PreparedPrint> {
        self.prepared.take()
    }
}

impl Drop for PreparedPrintGuard {
    fn drop(&mut self) {
        if let Some(prepared) = self.prepared.take() {
            let _ = discard_prepared_print(&prepared);
        }
    }
}

/// Worker progress or its single terminal result.
pub(crate) enum DesktopJobEvent {
    Progress { percent: u8, message: String },
    Finished(Result<DesktopJobResult, String>),
}

struct PrintPrepCheck {
    progress: u8,
    message: &'static str,
    check: fn(&Scene, f32) -> Result<PrintPrepReport, PrintPrepError>,
}

/// Runs one owned payload, checking cooperative cancellation at each safe
/// boundary and reporting bounded stage-level progress.
pub(crate) fn run_desktop_job<F>(
    request: DesktopJobRequest,
    handle: &OperationHandle,
    mut report: F,
) -> Result<DesktopJobResult, String>
where
    F: FnMut(u8, &str),
{
    check_cancelled(handle)?;
    match request {
        DesktopJobRequest::Mesh { operation, mesh } => {
            report(5, "Validating isolated mesh input");
            check_cancelled(handle)?;
            let (mesh, detail) = match operation {
                MeshJobOperation::Repair => {
                    let mut progress =
                        |elapsed: std::time::Duration, estimate: std::time::Duration| {
                            let percent = (10.0
                                + 80.0 * elapsed.as_secs_f64() / estimate.as_secs_f64().max(0.001))
                            .clamp(10.0, 90.0) as u8;
                            report(
                                percent,
                                &format!(
                                    "Repairing with the Windows engine, {}s of about {:.0}s",
                                    elapsed.as_secs(),
                                    estimate.as_secs_f64().ceil()
                                ),
                            );
                        };
                    let result = repair_mesh_routed(
                        &mesh,
                        RepairRouteControl {
                            allow_windows: modeler3d_platform::windows_repair_supported(),
                            cancelled: &|| handle.is_cancelled(),
                            progress: Some(&mut progress),
                        },
                    )
                    .map_err(|error| error.to_string())?;
                    if result.report.nothing_to_fix() {
                        return Ok(DesktopJobResult::Unchanged {
                            message: modeler3d_core::REPAIR_NOTHING_TO_FIX_MESSAGE.to_owned(),
                        });
                    }
                    if result.report.refused() {
                        return Ok(DesktopJobResult::Unchanged {
                            message: modeler3d_core::REPAIR_REFUSED_MESSAGE.to_owned(),
                        });
                    }
                    let stage_count = result.report.stages.len();
                    let mut detail = if result.report.engine == RepairEngine::Windows {
                        "Windows repair engine".to_owned()
                    } else {
                        format!("{stage_count} repair stage(s)")
                    };
                    if let Some(note) = &result.report.engine_note {
                        detail.push_str(&format!(". {note}"));
                    }
                    (result.mesh, detail)
                }
                MeshJobOperation::Simplify(percent) => {
                    let result =
                        simplify_mesh_by_percent(&mesh, percent, MeshOperationLimits::default())
                            .map_err(|error| error.to_string())?;
                    let report = result.report;
                    if !report.changed {
                        return Ok(DesktopJobResult::Unchanged {
                            message: format!(
                                "Simplify: this shape can't lose any faces without changing it ({} faces)",
                                group_thousands(report.before_faces)
                            ),
                        });
                    }
                    let mut message = format!(
                        "Simplify: {} → {} faces",
                        group_thousands(report.before_faces),
                        group_thousands(report.after_faces)
                    );
                    if report.after_faces > report.target_faces {
                        message.push_str(" (stopped early to keep the shape)");
                    }
                    return Ok(DesktopJobResult::Mesh {
                        operation,
                        mesh: result.mesh,
                        message,
                    });
                }
                MeshJobOperation::Subdivide => {
                    let result = subdivide_mesh_even(&mesh, None, MeshOperationLimits::default())
                        .map_err(|error| error.to_string())?;
                    let message = format!(
                        "Subdivide: {} \u{2192} {} faces",
                        group_thousands(result.report.before_faces),
                        group_thousands(result.report.after_faces)
                    );
                    return Ok(DesktopJobResult::Mesh {
                        operation,
                        mesh: result.mesh,
                        message,
                    });
                }
                MeshJobOperation::Smooth => {
                    let result = smooth_mesh(&mesh, 1).map_err(|error| error.to_string())?;
                    if !result.report.changed {
                        return Ok(DesktopJobResult::Unchanged {
                            message: SMOOTH_NOTHING_TO_SMOOTH_MESSAGE.to_owned(),
                        });
                    }
                    (result.mesh, "one smoothing pass complete".to_owned())
                }
            };
            report(95, "Checking isolated mesh result");
            check_cancelled(handle)?;
            let message = format!(
                "{} complete: {} face(s); {detail}",
                operation.label(),
                mesh.face_count()
            );
            Ok(DesktopJobResult::Mesh {
                operation,
                mesh,
                message,
            })
        }
        DesktopJobRequest::PrintPrep {
            snapshot,
            bed_size_mm,
        } => {
            let scene = scene_from_snapshot(snapshot)?;
            let mut combined_report = PrintPrepReport::default();
            let checks = [
                PrintPrepCheck {
                    progress: 10,
                    message: "Checking bed fit, height, and below-bed geometry",
                    check: check_bed_fit,
                },
                PrintPrepCheck {
                    progress: 35,
                    message: "Checking print scale",
                    check: |scene, _bed| check_scale(scene),
                },
                PrintPrepCheck {
                    progress: 55,
                    message: "Checking mesh health and disconnected shells",
                    check: |scene, _bed| check_non_manifold(scene),
                },
                PrintPrepCheck {
                    progress: 80,
                    message: "Checking hidden objects and empty meshes",
                    check: |scene, _bed| check_empty_hidden(scene),
                },
            ];
            for check in checks {
                check_cancelled(handle)?;
                report(check.progress, check.message);
                combined_report
                    .extend((check.check)(&scene, bed_size_mm).map_err(|error| error.to_string())?);
            }
            check_cancelled(handle)?;
            report(95, "Print-readiness report complete");
            Ok(DesktopJobResult::PrintPrep {
                bed_size_mm,
                report: combined_report,
            })
        }
        DesktopJobRequest::PreparePrint {
            snapshot,
            slicer,
            temp_root,
            document_generation,
        } => {
            let scene = scene_from_snapshot(snapshot)?;
            let mut prepared = prepare_print_with_hooks(
                &scene,
                &slicer,
                &temp_root,
                || handle.is_cancelled(),
                |percent, message| report(percent, message),
            )
            .map_err(|error| error.to_string())?;
            prepared.source_document_generation = Some(document_generation);
            if handle.is_cancelled() {
                let _ = discard_prepared_print(&prepared);
                return Err(String::from("print staging was cancelled"));
            }
            Ok(DesktopJobResult::PreparedPrint(PreparedPrintGuard::new(
                prepared,
            )))
        }
        DesktopJobRequest::Boolean {
            prepared,
            clear_selection,
        } => {
            report(
                5,
                &format!("Checking {} faces", group_thousands(prepared.input_faces())),
            );
            let result = prepared
                .compute(&mut |step| {
                    if handle.is_cancelled() {
                        return Err(CoreError::OperationCancelled(step.to_owned()));
                    }
                    if let Some((percent, next)) = boolean_progress(step) {
                        report(percent, next);
                    }
                    Ok(())
                })
                .map_err(|error| match error {
                    CoreError::OperationCancelled(_) => {
                        String::from("operation was cancelled; no scene changes were applied")
                    }
                    error => error.to_string(),
                })?;
            check_cancelled(handle)?;
            report(95, "Applying the result");
            Ok(DesktopJobResult::Boolean {
                prepared: Box::new(prepared),
                result: Box::new(result),
                clear_selection,
            })
        }
    }
}

/// Progress after a finished boolean step, and what the boolean does next.
fn boolean_progress(finished_step: &str) -> Option<(u8, &'static str)> {
    Some(match finished_step {
        "check input health" => (25, "Preparing the shapes"),
        "prepare operands" | "check limits" => (30, "Preparing the shapes"),
        "build kernel input" => (45, "Cutting"),
        "kernel boolean" => (70, "Reading the result"),
        "read kernel result" => (75, "Tidying the cut faces"),
        "regularize cut caps" => (85, "Checking the result"),
        _ => return None,
    })
}

fn scene_from_snapshot(snapshot: SceneSnapshot) -> Result<Scene, String> {
    let mut scene = Scene::new();
    scene
        .restore_snapshot(snapshot)
        .map_err(|error| error.to_string())?;
    Ok(scene)
}

fn check_cancelled(handle: &OperationHandle) -> Result<(), String> {
    if handle.is_cancelled() {
        Err(String::from(
            "operation was cancelled; no scene changes were applied",
        ))
    } else {
        Ok(())
    }
}

/// Writes a count with thousands separators, as the dialogs show it.
pub(crate) fn group_thousands(value: usize) -> String {
    let digits = value.to_string();
    let mut grouped = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(digit);
    }
    grouped
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::OperationRegistry;
    use modeler3d_core::PrimitiveKind;

    #[test]
    fn worker_uses_an_owned_mesh_and_reports_staged_progress() {
        let mut scene = Scene::new();
        assert!(
            scene
                .add_primitive(PrimitiveKind::Sphere, None, None)
                .is_ok()
        );
        let faces = scene.objects()[0].mesh().face_count();
        let mesh = scene.objects()[0].mesh().clone();
        let registry = OperationRegistry::default();
        let handle = registry
            .start("Smooth", "desktop", None)
            .unwrap_or_else(|error| panic!("start failed: {error}"));
        assert!(
            registry
                .update(
                    &handle.operation_id,
                    Some(crate::OperationPhase::Running),
                    None,
                    None
                )
                .is_ok()
        );
        let mut progress = Vec::new();
        let result = run_desktop_job(
            DesktopJobRequest::Mesh {
                operation: MeshJobOperation::Smooth,
                mesh,
            },
            &handle,
            |percent, message| progress.push((percent, message.to_owned())),
        );
        assert!(matches!(
            result,
            Ok(DesktopJobResult::Mesh {
                operation: MeshJobOperation::Smooth,
                ..
            })
        ));
        assert_eq!(progress.first().map(|item| item.0), Some(5));
        assert_eq!(progress.last().map(|item| item.0), Some(95));
        assert_eq!(scene.objects()[0].mesh().face_count(), faces);
    }

    #[test]
    fn cancelled_worker_returns_before_computing_or_producing_a_result() {
        let registry = OperationRegistry::default();
        let mut scene = Scene::new();
        assert!(scene.add_primitive(PrimitiveKind::Cube, None, None).is_ok());
        let handle = registry
            .start("Smooth", "desktop", None)
            .unwrap_or_else(|error| panic!("start failed: {error}"));
        assert!(registry.cancel(&handle.operation_id).is_ok());
        let result = run_desktop_job(
            DesktopJobRequest::Mesh {
                operation: MeshJobOperation::Smooth,
                mesh: scene.objects()[0].mesh().clone(),
            },
            &handle,
            |_, _| {},
        );
        assert_eq!(
            result.err().as_deref(),
            Some("operation was cancelled; no scene changes were applied")
        );
    }
}
