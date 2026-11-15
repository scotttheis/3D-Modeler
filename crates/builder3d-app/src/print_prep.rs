//! Print-preparation checks shared by the desktop command and future adapters.
//!
//! The checks intentionally inspect authoritative scene objects and transformed
//! world bounds.  They do not mutate the scene or depend on a UI toolkit.

use builder3d_core::{CoreError, Scene, inspect_mesh};
use thiserror::Error;

type Bounds = ([f32; 3], [f32; 3]);

/// The default square build-bed size used by the reference application.
pub const DEFAULT_BED_SIZE_MM: f32 = 200.0;
/// The scale below which a model may be difficult to print.
pub const MIN_PRINTABLE_DIMENSION_MM: f32 = 1.0;
/// The scale above which a model may exceed a typical printer volume.
pub const MAX_TYPICAL_PRINT_DIMENSION_MM: f32 = 300.0;
/// The small tolerance allowed below the print-bed surface.
pub const BED_SURFACE_TOLERANCE_MM: f32 = 0.01;

/// Errors produced before a print-preparation report can be generated.
#[derive(Debug, Error)]
pub enum PrintPrepError {
    /// The requested bed size was not a finite positive distance.
    #[error("bed size must be a positive finite number")]
    InvalidBedSize,
    /// A scene object's transformed mesh could not be inspected.
    #[error("print-preparation mesh inspection failed: {0}")]
    Core(#[from] CoreError),
}

/// Human-readable print-preparation findings in reference order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PrintPrepReport {
    /// Non-fatal findings that should be shown before printing.
    pub warnings: Vec<String>,
    /// Findings that make the current print set invalid.
    pub errors: Vec<String>,
}

impl PrintPrepReport {
    /// Returns whether no blocking findings were reported.
    #[must_use]
    pub fn ok(&self) -> bool {
        self.errors.is_empty()
    }

    /// Adds all findings from another report while preserving check order.
    pub fn extend(&mut self, other: Self) {
        self.warnings.extend(other.warnings);
        self.errors.extend(other.errors);
    }

    /// Formats the report using the desktop dialog's stable line format.
    #[must_use]
    pub fn summary(&self) -> String {
        let mut lines = Vec::with_capacity(self.errors.len() + self.warnings.len());
        lines.extend(
            self.errors
                .iter()
                .map(|message| format!("ERROR: {message}")),
        );
        lines.extend(
            self.warnings
                .iter()
                .map(|message| format!("WARNING: {message}")),
        );
        if lines.is_empty() {
            String::from("No issues found.")
        } else {
            lines.join("\n")
        }
    }
}

/// Checks visible objects against the square bed footprint and height.
pub fn check_bed_fit(scene: &Scene, bed_size_mm: f32) -> Result<PrintPrepReport, PrintPrepError> {
    validate_bed_size(bed_size_mm)?;
    let bed_half = bed_size_mm / 2.0;
    let mut report = PrintPrepReport::default();
    for object in scene.objects() {
        if !object.is_visible() {
            continue;
        }
        let Some((minimum, maximum)) = transformed_bounds(object)? else {
            continue;
        };
        if minimum[0] < -bed_half || maximum[0] > bed_half {
            report.errors.push(format!(
                "{}: outside bed width (X bounds {:.1}..{:.1}mm; bed is {:.1}..{:.1}mm)",
                object.name(),
                minimum[0],
                maximum[0],
                -bed_half,
                bed_half
            ));
        }
        if minimum[2] < -bed_half || maximum[2] > bed_half {
            report.errors.push(format!(
                "{}: outside bed depth (Z bounds {:.1}..{:.1}mm; bed is {:.1}..{:.1}mm)",
                object.name(),
                minimum[2],
                maximum[2],
                -bed_half,
                bed_half
            ));
        }
        let height = maximum[1] - minimum[1];
        if height > bed_size_mm {
            report.warnings.push(format!(
                "{}: exceeds typical print height ({height:.1}mm > {bed_size_mm:.1}mm)",
                object.name()
            ));
        }
        if minimum[1] < -BED_SURFACE_TOLERANCE_MM {
            report.warnings.push(format!(
                "{}: below print bed surface ({:.1}mm)",
                object.name(),
                minimum[1]
            ));
        }
    }
    Ok(report)
}

/// Warns when a visible object is unusually small or large for printing.
pub fn check_scale(scene: &Scene) -> Result<PrintPrepReport, PrintPrepError> {
    let mut report = PrintPrepReport::default();
    for object in scene.objects() {
        if !object.is_visible() {
            continue;
        }
        let Some((minimum, maximum)) = transformed_bounds(object)? else {
            continue;
        };
        let extents = [
            maximum[0] - minimum[0],
            maximum[1] - minimum[1],
            maximum[2] - minimum[2],
        ];
        let max_dimension = extents.into_iter().fold(0.0_f32, f32::max);
        if max_dimension < MIN_PRINTABLE_DIMENSION_MM {
            report.warnings.push(format!(
                "{}: very small ({max_dimension:.2}mm), may be hard to print",
                object.name()
            ));
        }
        if max_dimension > MAX_TYPICAL_PRINT_DIMENSION_MM {
            report.warnings.push(format!(
                "{}: very large ({max_dimension:.0}mm), may exceed printer volume",
                object.name()
            ));
        }
    }
    Ok(report)
}

/// Warns when visible meshes have topology issues or disconnected shells.
pub fn check_non_manifold(scene: &Scene) -> Result<PrintPrepReport, PrintPrepError> {
    let mut report = PrintPrepReport::default();
    for object in scene.objects() {
        if !object.is_visible() {
            continue;
        }
        let health = inspect_mesh(object.mesh())?;
        if !health.valid {
            report.warnings.push(format!(
                "{}: mesh issues ({})",
                object.name(),
                health.issues.join("; ")
            ));
        }
        // A hollow's inner cavity is part of the same piece (OF-54).
        let pieces = health.body_count - health.cavity_shells;
        if pieces > 1 {
            report
                .warnings
                .push(format!("{}: {} disconnected shells", object.name(), pieces));
        }
    }
    Ok(report)
}

/// Reports hidden objects and invalid empty meshes in the complete scene.
pub fn check_empty_hidden(scene: &Scene) -> Result<PrintPrepReport, PrintPrepError> {
    let mut report = PrintPrepReport::default();
    for object in scene.objects() {
        if !object.is_visible() {
            report
                .warnings
                .push(format!("{}: hidden, will not print", object.name()));
        }
        if object.mesh().bounds().is_none() {
            report.errors.push(format!("{}: empty mesh", object.name()));
        }
    }
    Ok(report)
}

/// Runs all print-preparation checks in the reference order.
pub fn full_check(scene: &Scene, bed_size_mm: f32) -> Result<PrintPrepReport, PrintPrepError> {
    validate_bed_size(bed_size_mm)?;
    let mut report = PrintPrepReport::default();
    report.extend(check_bed_fit(scene, bed_size_mm)?);
    report.extend(check_scale(scene)?);
    report.extend(check_non_manifold(scene)?);
    report.extend(check_empty_hidden(scene)?);
    Ok(report)
}

fn transformed_bounds(
    object: &builder3d_core::SceneObject,
) -> Result<Option<Bounds>, PrintPrepError> {
    Ok(object.transformed_mesh()?.bounds())
}

fn validate_bed_size(bed_size_mm: f32) -> Result<(), PrintPrepError> {
    if !bed_size_mm.is_finite() || bed_size_mm <= 0.0 {
        return Err(PrintPrepError::InvalidBedSize);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use builder3d_core::{PrimitiveKind, Scene};

    fn cube_scene() -> Scene {
        let mut scene = Scene::new();
        let result = scene.add_primitive(PrimitiveKind::Cube, Some([20.0, 20.0, 20.0]), None);
        assert!(result.is_ok());
        assert!(scene.translate(0, [0.0, 10.0, 0.0]).is_ok());
        scene
    }

    #[test]
    fn clean_cube_has_no_print_findings() {
        let scene = cube_scene();
        let report = full_check(&scene, DEFAULT_BED_SIZE_MM);
        assert!(report.is_ok());
        let report = report.unwrap_or_default();
        assert_eq!(report.summary(), "No issues found.");
    }

    #[test]
    fn bed_fit_reports_world_bounds_and_surface_warning() {
        let mut scene = cube_scene();
        assert!(scene.translate(0, [100.0, -20.0, 0.0]).is_ok());
        let report = check_bed_fit(&scene, 200.0);
        assert!(report.is_ok());
        let report = report.unwrap_or_default();
        assert!(
            report
                .errors
                .iter()
                .any(|line| line.contains("outside bed width"))
        );
        assert!(
            report
                .warnings
                .iter()
                .any(|line| line.contains("below print bed"))
        );
    }

    #[test]
    fn scale_and_hidden_checks_match_reference_severity() {
        let mut scene = cube_scene();
        assert!(scene.scale(0, [0.01, 0.01, 0.01]).is_ok());
        assert!(scene.set_visible(0, false).is_ok());
        let report = full_check(&scene, 200.0);
        assert!(report.is_ok());
        let report = report.unwrap_or_default();
        assert!(report.warnings.iter().any(|line| line.contains("hidden")));
        assert!(
            !report
                .warnings
                .iter()
                .any(|line| line.contains("very small"))
        );
    }

    #[test]
    fn invalid_bed_size_is_rejected_before_checks() {
        let scene = cube_scene();
        assert!(matches!(
            full_check(&scene, 0.0),
            Err(PrintPrepError::InvalidBedSize)
        ));
    }
}
