//! Bounded mesh repair derived from the Python repair profiles.
//!
//! Repair is deliberately non-mutating. It records the ordered filter
//! stages, keeps a structurally safe best-effort result when a mesh remains
//! open, and only claims a valid manifold when inspect_mesh can verify it.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::time::Duration;

use crate::repair_native::{self as native, RepairGrid};
use crate::windows_repair::{WindowsRepairError, run_windows_repair};
use crate::{CoreError, Mesh, MeshHealthReport, MeshOperationLimits, inspect_mesh};

/// The repair tolerance used by the Python reference, in millimetres.
pub const REPAIR_TOLERANCE_MM: f32 = 0.001;
/// The maximum component scale considered a disposable tiny shell.
pub const REPAIR_TINY_SHELL_TOLERANCE_MM: f32 = 0.01;
/// The default repair profile.
pub const REPAIR_DEFAULT_PROFILE: RepairProfile = RepairProfile::Extended;

/// Engine that produced a repair candidate; identifiers are portable report values.
#[must_use]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RepairEngine {
    /// The optional Windows public WinRT repair service.
    Windows,
    /// The portable built-in repair pipeline.
    Native,
}

impl RepairEngine {
    /// Returns the same stable report identifier on every platform.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Windows => "windows",
            Self::Native => "native",
        }
    }
}

/// Portable routing controls. Windows availability is supplied by the platform adapter.
pub struct RepairRouteControl<'a> {
    /// Permits attempting WinRT; unavailable engines fall back to native repair.
    pub allow_windows: bool,
    /// Cooperative user cancellation, checked while waiting and before applying a result.
    pub cancelled: &'a dyn Fn() -> bool,
    /// Windows wait progress as elapsed time and an estimated duration, about every 100 ms.
    pub progress: Option<&'a mut dyn FnMut(Duration, Duration)>,
}

/// Estimates Windows repair time from measured face scaling, identically on every host.
#[must_use]
pub fn repair_time_estimate(faces: usize) -> Duration {
    Duration::try_from_secs_f64(0.1 * (faces as f64 / 4000.0).powf(1.28)).unwrap_or(Duration::MAX)
}

/// Bounds Windows repair waits to three estimates, clamped to 30 seconds..10 minutes.
#[must_use]
pub fn repair_time_limit(faces: usize) -> Duration {
    repair_time_estimate(faces)
        .saturating_mul(3)
        .clamp(Duration::from_secs(30), Duration::from_secs(600))
}

/// Ordered repair profile selection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RepairProfile {
    /// Winding, trivial/all hole closure, and export cleanup.
    Simple,
    /// The enhanced profile uses the bounded NMR-compatible cleanup family.
    Enhanced,
    /// The complete captured repair pipeline.
    Extended,
}

impl RepairProfile {
    /// Parses the Python profile names without accepting an ambiguous value.
    pub fn parse(value: &str) -> Result<Self, CoreError> {
        match value.trim().to_ascii_lowercase().as_str() {
            "simple" => Ok(Self::Simple),
            "enhanced" => Ok(Self::Enhanced),
            "extended" => Ok(Self::Extended),
            other => Err(CoreError::MeshOperation(format!(
                "Unknown repair profile: {other}. Choose: simple, enhanced, extended"
            ))),
        }
    }

    /// Returns the stable profile identifier used in reports.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Simple => "simple",
            Self::Enhanced => "enhanced",
            Self::Extended => "extended",
        }
    }
}

/// One captured repair-stage transition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepairStageReport {
    /// Reference-stage name.
    pub name: String,
    /// Vertex count before the stage.
    pub before_vertices: usize,
    /// Triangle count before the stage.
    pub before_triangles: usize,
    /// Vertex count after the stage.
    pub after_vertices: usize,
    /// Triangle count after the stage.
    pub after_triangles: usize,
    /// Whether the indexed counts changed.
    pub changed: bool,
    /// Recoverable stage error, if a best-effort stage was skipped.
    pub error: Option<String>,
}

/// Detail for one planar cleanup candidate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanarCleanupDetail {
    /// Number of candidate faces before replacement.
    pub faces_before: usize,
    /// Number of emitted faces after replacement.
    pub faces_after: usize,
    /// Number of boundary vertices in the candidate patch.
    pub boundary_vertices: usize,
    /// Number of duplicate faces removed with the patch.
    pub faces_removed: usize,
}

/// Bounded planar-cleanup evidence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanarCleanupReport {
    /// Whether at least one eligible planar region was inspected/rebuilt.
    pub changed: bool,
    /// Number of eligible patches.
    pub patches: usize,
    /// Total candidate faces before cleanup.
    pub faces_before: usize,
    /// Total emitted candidate faces after cleanup.
    pub faces_after: usize,
    /// Per-patch evidence.
    pub details: Vec<PlanarCleanupDetail>,
}

/// Portable topology defects used to compare repair candidates; lower is better.
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RepairScore {
    /// Boundary edges.
    pub open_edges: usize,
    /// Edges with more than two owners.
    pub non_manifold_edges: usize,
    /// Two-owner edges with agreeing directions.
    pub flipped_edges: usize,
    /// Triangles below the shared health threshold.
    pub degenerate_faces: usize,
    /// Repeated triangles, ignoring winding.
    pub duplicate_faces: usize,
    /// Inward shells when the entire model is inside out.
    pub inside_out_shells: usize,
    /// Components below the tiny-shell scale.
    pub tiny_shells: usize,
    /// Connected components excluding cavity surfaces.
    pub shells: usize,
}

impl RepairScore {
    /// Builds the same repair score on every platform, excluding correct cavities.
    pub fn from_health(health: &MeshHealthReport) -> Self {
        Self {
            open_edges: health.boundary_edges,
            non_manifold_edges: health.non_manifold_edges,
            flipped_edges: health.flipped_edges,
            degenerate_faces: health.degenerate_faces,
            duplicate_faces: health.duplicate_faces,
            inside_out_shells: health.inside_out_shells,
            tiny_shells: health.tiny_shells,
            shells: health
                .body_count
                .saturating_sub(health.cavity_shells)
                .max(0) as usize,
        }
    }

    fn fields(self) -> [usize; 8] {
        [
            self.open_edges,
            self.non_manifold_edges,
            self.flipped_edges,
            self.degenerate_faces,
            self.duplicate_faces,
            self.inside_out_shells,
            self.tiny_shells,
            self.shells,
        ]
    }
}

/// A candidate must reduce a defect without increasing any other. Shells may
/// increase when separating strictly fewer non-manifold edges into parts.
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RepairVerdict {
    /// A score improved and none got worse, allowing non-manifold separation.
    Improved,
    /// Every score stayed the same.
    NoBetter,
    /// At least one score got worse, even if another improved.
    Worse,
}

fn gate(before: RepairScore, after: RepairScore) -> RepairVerdict {
    let mut better = false;
    for (field, (old, new)) in before.fields().into_iter().zip(after.fields()).enumerate() {
        if new > old && !(field == 7 && after.non_manifold_edges < before.non_manifold_edges) {
            return RepairVerdict::Worse;
        }
        better |= new < old;
    }
    if better {
        RepairVerdict::Improved
    } else {
        RepairVerdict::NoBetter
    }
}

/// Health and commit-safety evidence for a repair attempt.
#[derive(Clone, Debug, PartialEq)]
pub struct RepairReport {
    /// Engine that produced this candidate (native on hosts without Windows repair).
    pub engine: RepairEngine,
    /// Plain-language reason for using the native fallback, when applicable.
    pub engine_note: Option<String>,
    /// Profile identifier.
    pub profile: String,
    /// Weld/degenerate tolerance in millimetres.
    pub tolerance_mm: f32,
    /// Tiny-shell scale threshold in millimetres.
    pub tiny_shell_tolerance_mm: f32,
    /// Health before the first stage.
    pub before: MeshHealthReport,
    /// Ordered stage evidence.
    pub stages: Vec<RepairStageReport>,
    /// Planar cleanup evidence.
    pub planar_cleanup: PlanarCleanupReport,
    /// Health after all stages.
    pub after: MeshHealthReport,
    /// Topology score before repair.
    pub score_before: RepairScore,
    /// Topology score of the proposed candidate.
    pub score_after: RepairScore,
    /// Whether the candidate improves health without worsening another defect.
    pub verdict: RepairVerdict,
    /// Recoverable stage errors.
    pub errors: Vec<String>,
    /// Whether the candidate has usable finite indexed geometry.
    pub candidate_safe: bool,
    /// Whether indexed geometry changed.
    pub candidate_changed: bool,
    /// Whether the candidate is safe and strictly improves the topology score.
    pub can_apply: bool,
}

/// What the user is told when Repair finds nothing to fix (OF-40). The
/// desktop shows it in the Repair dialog; MCP returns it as `message`.
pub const REPAIR_NOTHING_TO_FIX_MESSAGE: &str =
    "No problems found — your model was already clean. Nothing was changed.";

/// Shared desktop/MCP message when a flawed mesh cannot be improved safely.
pub const REPAIR_REFUSED_MESSAGE: &str = "Repair couldn't improve this model. Nothing was changed.";

impl RepairReport {
    /// Whether the mesh had no problems to begin with (OF-40). Adapters then
    /// apply nothing, so Repair on a clean model leaves no history entry.
    #[must_use]
    pub fn nothing_to_fix(&self) -> bool {
        self.before.valid
    }

    /// Whether adapters must leave a flawed source unchanged, including its history.
    #[must_use]
    pub fn refused(&self) -> bool {
        !self.nothing_to_fix() && !self.can_apply
    }
}

/// Non-mutating repair result containing the candidate mesh and report.
#[derive(Clone, Debug, PartialEq)]
pub struct RepairResult {
    /// Repaired or best-effort candidate mesh.
    pub mesh: Mesh,
    /// Repair evidence.
    pub report: RepairReport,
}

/// Repairs a mesh with the default extended profile and default limits.
pub fn repair_mesh(mesh: &Mesh) -> Result<RepairResult, CoreError> {
    repair_mesh_with_options(mesh, REPAIR_DEFAULT_PROFILE, MeshOperationLimits::default())
}

/// Repairs through the optional Windows service or the portable native pipeline.
/// Both routes share the topology gate; Windows candidates also retain bounds and
/// closed single-shell volume. Correct cavities always use the native pipeline.
pub fn repair_mesh_routed(
    mesh: &Mesh,
    mut control: RepairRouteControl<'_>,
) -> Result<RepairResult, CoreError> {
    let check_cancelled = || {
        if (control.cancelled)() {
            Err(CoreError::MeshOperation("Repair cancelled".into()))
        } else {
            Ok(())
        }
    };
    check_cancelled()?;
    let before = inspect_mesh(mesh)?;
    if before.valid {
        let result = repair_mesh(mesh)?;
        check_cancelled()?;
        return Ok(result);
    }
    let mut windows_collapsed = false;
    let mut note = if before.cavity_shells > 0 && has_closed_cavity(mesh) {
        Some("Repair kept the hollow inside, so the native engine was used.".to_owned())
    } else if !control.allow_windows
        || std::env::var("MODELER3D_REPAIR_ENGINE").is_ok_and(|value| value == "native")
    {
        None
    } else {
        MeshOperationLimits::default().check_mesh("repair", mesh)?;
        let estimate = repair_time_estimate(mesh.face_count());
        let mut elapsed = |duration| {
            if let Some(progress) = control.progress.as_deref_mut() {
                progress(duration, estimate);
            }
        };
        let attempt = run_windows_repair(
            mesh,
            repair_time_limit(mesh.face_count()),
            control.cancelled,
            &mut elapsed,
        );
        check_cancelled()?;
        let reason = match attempt {
            Ok(candidate) => {
                windows_collapsed = collapsed(mesh, &before, &candidate).is_some();
                match windows_candidate(mesh, &before, candidate) {
                    Ok(result) => {
                        check_cancelled()?;
                        return Ok(result);
                    }
                    Err(reason) => reason,
                }
            }
            Err(WindowsRepairError::Unavailable) => "Windows repair was unavailable".to_owned(),
            Err(WindowsRepairError::TimedOut) => "Windows repair took too long".to_owned(),
            Err(WindowsRepairError::Cancelled) => {
                "Windows repair stopped before it finished".to_owned()
            }
            Err(WindowsRepairError::Failed(reason)) => format!("Windows repair failed ({reason})"),
        };
        Some(format!("{reason}, so the built-in engine was used."))
    };
    check_cancelled()?;
    let mut result = repair_mesh(mesh)?;
    check_cancelled()?;
    // A rejected Windows collapse must not be replaced by the native
    // pipeline's closed, almost zero-volume triangle sheets. This extra
    // catastrophic-loss check is limited to that fallback; native-only APIs
    // and the Windows volume rule for open/multiple shells stay unchanged.
    if windows_collapsed && nearly_empty_fallback(mesh, &result.mesh) {
        result.report.candidate_safe = false;
        result.report.can_apply = false;
        let reason =
            "The built-in repair also lost almost all of the model's volume. Nothing was changed.";
        result.report.errors.push(reason.to_owned());
        if let Some(note) = &mut note {
            note.push(' ');
            note.push_str(reason);
        }
    }
    result.report.engine_note = note;
    Ok(result)
}

fn windows_candidate(
    mesh: &Mesh,
    before: &MeshHealthReport,
    candidate: Mesh,
) -> Result<RepairResult, String> {
    let after = inspect_mesh(&candidate)
        .map_err(|error| format!("Windows repair returned an invalid model ({error})"))?;
    let score_before = RepairScore::from_health(before);
    let score_after = RepairScore::from_health(&after);
    let verdict = gate(score_before, score_after);
    let candidate_safe = is_safe_candidate(&candidate);
    let candidate_changed = candidate != *mesh;
    let collapse = collapsed(mesh, before, &candidate);
    let result = RepairResult {
        report: RepairReport {
            engine: RepairEngine::Windows,
            engine_note: None,
            profile: REPAIR_DEFAULT_PROFILE.as_str().to_owned(),
            tolerance_mm: REPAIR_TOLERANCE_MM,
            tiny_shell_tolerance_mm: REPAIR_TINY_SHELL_TOLERANCE_MM,
            before: before.clone(),
            after,
            score_before,
            score_after,
            verdict,
            candidate_safe,
            candidate_changed,
            can_apply: candidate_safe && verdict == RepairVerdict::Improved,
            errors: Vec::new(),
            stages: vec![RepairStageReport {
                name: "Windows repair engine".into(),
                before_vertices: mesh.vertex_count(),
                before_triangles: mesh.face_count(),
                after_vertices: candidate.vertex_count(),
                after_triangles: candidate.face_count(),
                changed: candidate_changed,
                error: None,
            }],
            planar_cleanup: PlanarCleanupReport {
                changed: false,
                patches: 0,
                faces_before: 0,
                faces_after: 0,
                details: Vec::new(),
            },
        },
        mesh: candidate,
    };
    if let Some(reason) = collapse {
        return Err(reason);
    }
    if !result.report.can_apply {
        return Err("Windows repair could not safely improve this model".to_owned());
    }
    Ok(result)
}

fn collapsed(
    before_mesh: &Mesh,
    before_health: &MeshHealthReport,
    after_mesh: &Mesh,
) -> Option<String> {
    let (before_min, before_max) = before_mesh.bounds()?;
    let Some((after_min, after_max)) = after_mesh.bounds() else {
        return Some("Windows repair returned an empty model".into());
    };
    // Open or non-manifold input may lose loose fins and sheets, which
    // legitimately shrinks the box, so only a large loss counts there.
    let closed = before_health.boundary_edges == 0 && before_health.non_manifold_edges == 0;
    for axis in 0..3 {
        let lost = (f64::from(after_min[axis]) - f64::from(before_min[axis]))
            .max(f64::from(before_max[axis]) - f64::from(after_max[axis]));
        let extent = f64::from(before_max[axis]) - f64::from(before_min[axis]);
        let extent_lost = extent - (f64::from(after_max[axis]) - f64::from(after_min[axis]));
        if (closed && lost > 0.1) || (!closed && lost > 0.1 && extent_lost > extent * 0.5) {
            return Some("Windows repair shrank the model by more than 0.1 mm".into());
        }
    }
    if before_health.body_count == 1
        && before_health.watertight
        && before_health.winding_consistent
        && before_health.non_manifold_edges == 0
        && before_health.duplicate_faces == 0
        && before_health.degenerate_faces == 0
    {
        let before_volume = repair_volume(before_mesh);
        if before_volume > 0.0 && repair_volume(after_mesh) < before_volume * 0.97 {
            return Some("Windows repair reduced the enclosed volume by more than 3%".into());
        }
    }
    // Whatever the input, losing over half the volume is a collapse (the
    // jittered-soup case), never a legitimate fin or overlap removal.
    let before_volume = repair_volume(before_mesh).abs();
    if before_volume > 1.0e-9 && repair_volume(after_mesh).abs() < before_volume * 0.5 {
        return Some("Windows repair removed over half of the model's volume".into());
    }
    None
}

/// Whether a closed, inward-facing shell sits beside an outward one. Open
/// pieces (unwelded soup) can have a negative partial volume but are not
/// cavities, so only fully closed shells count.
fn has_closed_cavity(mesh: &Mesh) -> bool {
    let faces = mesh.faces.len();
    let mut parent: Vec<usize> = (0..faces).collect();
    fn root(parent: &mut [usize], mut face: usize) -> usize {
        while parent[face] != face {
            parent[face] = parent[parent[face]];
            face = parent[face];
        }
        face
    }
    let mut edges: BTreeMap<(u32, u32), (usize, u32)> = BTreeMap::new();
    for (index, face) in mesh.faces.iter().enumerate() {
        for corner in 0..3 {
            let (a, b) = (face[corner], face[(corner + 1) % 3]);
            let entry = edges.entry((a.min(b), a.max(b))).or_insert((index, 0));
            entry.1 += 1;
            let (first, second) = (root(&mut parent, entry.0), root(&mut parent, index));
            if first != second {
                parent[second] = first;
            }
        }
    }
    let mut open = BTreeSet::new();
    for &(owner, count) in edges.values() {
        if count != 2 {
            open.insert(root(&mut parent, owner));
        }
    }
    let mut volumes: BTreeMap<usize, f64> = BTreeMap::new();
    for (index, face) in mesh.faces.iter().enumerate() {
        let corners = face.map(|vertex| mesh.vertices[vertex as usize].map(f64::from));
        let shell = root(&mut parent, index);
        *volumes.entry(shell).or_insert(0.0) +=
            dot64(corners[0], cross64(corners[1], corners[2])) / 6.0;
    }
    // The outer shell may itself still have a hole; only the cavity must be closed.
    volumes.values().any(|volume| *volume > 0.0)
        && volumes
            .iter()
            .any(|(shell, volume)| *volume < 0.0 && !open.contains(shell))
}

fn repair_volume(mesh: &Mesh) -> f64 {
    mesh.faces
        .iter()
        .map(|face| {
            let a = mesh.vertices[face[0] as usize].map(f64::from);
            let b = mesh.vertices[face[1] as usize].map(f64::from);
            let c = mesh.vertices[face[2] as usize].map(f64::from);
            dot64(a, cross64(b, c)) / 6.0
        })
        .sum()
}

fn nearly_empty_fallback(before: &Mesh, after: &Mesh) -> bool {
    let before_volume = repair_volume(before).abs();
    before_volume > 1.0e-12 && repair_volume(after).abs() < before_volume * 0.01
}

/// Repairs a mesh using a named Python-compatible profile.
pub fn repair_mesh_with_profile(mesh: &Mesh, profile: &str) -> Result<RepairResult, CoreError> {
    let profile = RepairProfile::parse(profile)?;
    repair_mesh_with_options(mesh, profile, MeshOperationLimits::default())
}

/// Repairs a mesh with explicit profile and resource limits.
pub fn repair_mesh_with_options(
    mesh: &Mesh,
    profile: RepairProfile,
    limits: MeshOperationLimits,
) -> Result<RepairResult, CoreError> {
    mesh.validate()?;
    limits.validate("repair")?;
    limits.check_mesh("repair", mesh)?;
    if mesh.is_empty() {
        return Err(CoreError::MeshOperation(
            "repair requires a non-empty triangle mesh".to_owned(),
        ));
    }

    let before = inspect_mesh(mesh)?;
    let grid = RepairGrid::from_mesh(mesh);
    let mut current = mesh.clone();
    let mut stages = Vec::new();
    let mut errors = Vec::new();
    let pipeline = stage_names(profile);
    for name in pipeline {
        let operation = |value: &Mesh| -> Result<Mesh, CoreError> {
            let result = match name {
                "StitchGaps (insert vertices)" => native::stitch(value, grid, true),
                "StitchGaps (reuse vertices)" => native::stitch(value, grid, false),
                "CloseTrivialHoles" | "CloseTrivialHoles (second pass)" => {
                    close_holes(value, grid, limits, 4)
                }
                "CloseAllHoles" => close_holes(value, grid, limits, usize::MAX),
                "RemoveDoubleTriangles" => native::clean_faces(value),
                "SplitNonManifoldEdges" => native::split_non_manifold(value, grid),
                "FixFlippedTriangles" | "FixFlippedTriangles (second pass)" => fix_winding(value),
                "Wrap" => wrap_mesh(value),
                "RemoveDegeneratedFaces" => {
                    remove_degenerate_faces(&native::collapse(value, grid)?)
                }
                "RemoveTinyShells" => remove_tiny_shells(value),
                "PrepareForExport" => prepare_for_export(value),
                _ => Err(CoreError::MeshOperation(format!(
                    "unknown internal repair stage: {name}"
                ))),
            }?;
            limits.check_mesh("repair", &result)?;
            Ok(result)
        };
        current = run_stage(current, name, operation, &mut stages, &mut errors);
    }

    // Flat-patch re-triangulation was removed from Repair in R4: it added
    // faces and fixed nothing (repair plan, fix plan Step 3 item 10).
    let planar_cleanup = PlanarCleanupReport {
        changed: false,
        patches: 0,
        faces_before: 0,
        faces_after: 0,
        details: Vec::new(),
    };
    let after = inspect_mesh(&current)?;
    let candidate_safe = is_safe_candidate(&current);
    let candidate_changed = current != *mesh;
    let score_before = RepairScore::from_health(&before);
    let score_after = RepairScore::from_health(&after);
    let verdict = gate(score_before, score_after);
    Ok(RepairResult {
        mesh: current,
        report: RepairReport {
            engine: RepairEngine::Native,
            engine_note: None,
            profile: profile.as_str().to_owned(),
            tolerance_mm: REPAIR_TOLERANCE_MM,
            tiny_shell_tolerance_mm: REPAIR_TINY_SHELL_TOLERANCE_MM,
            before,
            stages,
            planar_cleanup,
            after,
            score_before,
            score_after,
            verdict,
            errors,
            candidate_safe,
            candidate_changed,
            can_apply: candidate_safe && verdict == RepairVerdict::Improved,
        },
    })
}

fn stage_names(profile: RepairProfile) -> Vec<&'static str> {
    match profile {
        RepairProfile::Simple => vec![
            "FixFlippedTriangles",
            "CloseTrivialHoles",
            "CloseAllHoles",
            "PrepareForExport",
        ],
        RepairProfile::Enhanced | RepairProfile::Extended => vec![
            "StitchGaps (insert vertices)",
            "CloseTrivialHoles",
            "RemoveDoubleTriangles",
            "SplitNonManifoldEdges",
            "FixFlippedTriangles",
            "CloseTrivialHoles (second pass)",
            "StitchGaps (reuse vertices)",
            "FixFlippedTriangles (second pass)",
            "CloseAllHoles",
            "Wrap",
            "RemoveDegeneratedFaces",
            "RemoveTinyShells",
            "PrepareForExport",
        ],
    }
}

fn run_stage<F>(
    current: Mesh,
    name: &str,
    operation: F,
    stages: &mut Vec<RepairStageReport>,
    errors: &mut Vec<String>,
) -> Mesh
where
    F: FnOnce(&Mesh) -> Result<Mesh, CoreError>,
{
    let before_vertices = current.vertex_count();
    let before_triangles = current.face_count();
    let result = operation(&current);
    let (next, error) = match result {
        Ok(mesh) => (mesh, None),
        Err(error) => {
            let text = error.to_string();
            errors.push(format!("{name}: {text}"));
            (current.clone(), Some(text))
        }
    };
    let after_vertices = next.vertex_count();
    let after_triangles = next.face_count();
    stages.push(RepairStageReport {
        name: name.to_owned(),
        before_vertices,
        before_triangles,
        after_vertices,
        after_triangles,
        changed: before_vertices != after_vertices || before_triangles != after_triangles,
        error,
    });
    next
}

fn remove_duplicate_faces(mesh: &Mesh) -> Result<Mesh, CoreError> {
    let mut seen = BTreeSet::<[u32; 3]>::new();
    let mut faces = Vec::with_capacity(mesh.face_count());
    for face in &mesh.faces {
        let mut key = *face;
        key.sort_unstable();
        if seen.insert(key) {
            faces.push(*face);
        }
    }
    compact_mesh(&mesh.vertices, &faces)
}

fn remove_degenerate_faces(mesh: &Mesh) -> Result<Mesh, CoreError> {
    // Geometrically thin triangles can seal a solid. Only welding's repeated
    // indices may be discarded here; health reports retain the shared area rule.
    let faces = mesh
        .faces
        .iter()
        .copied()
        .filter(|[a, b, c]| a != b && b != c && c != a)
        .collect::<Vec<_>>();
    compact_mesh(&mesh.vertices, &faces)
}

fn fix_winding(mesh: &Mesh) -> Result<Mesh, CoreError> {
    if mesh.faces.is_empty() {
        return Ok(mesh.clone());
    }
    let edges = native::edge_records(&mesh.faces);
    let mut adjacency = vec![Vec::<(usize, bool, bool)>::new(); mesh.face_count()];
    for owners in edges.chunk_by(|a, b| a.0 == b.0) {
        for first_index in 0..owners.len() {
            for second_index in (first_index + 1)..owners.len() {
                let (_, first_face, first_direction) = owners[first_index];
                let (_, second_face, second_direction) = owners[second_index];
                adjacency[first_face].push((second_face, first_direction, second_direction));
                adjacency[second_face].push((first_face, second_direction, first_direction));
            }
        }
    }
    let mut orientation = vec![None::<bool>; mesh.face_count()];
    let mut components = Vec::<Vec<usize>>::new();
    for start in 0..mesh.face_count() {
        if orientation[start].is_some() {
            continue;
        }
        orientation[start] = Some(true);
        let mut component = Vec::new();
        let mut queue = VecDeque::from([start]);
        while let Some(face_index) = queue.pop_front() {
            component.push(face_index);
            let current_orientation = orientation[face_index].unwrap_or(true);
            for &(neighbor, current_direction, neighbor_direction) in &adjacency[face_index] {
                let current_final = if current_orientation {
                    current_direction
                } else {
                    !current_direction
                };
                let wanted = !current_final;
                let neighbor_orientation = neighbor_direction == wanted;
                if orientation[neighbor].is_none() {
                    orientation[neighbor] = Some(neighbor_orientation);
                    queue.push_back(neighbor);
                }
            }
        }
        components.push(component);
    }

    let mut faces = mesh.faces.clone();
    for (face_index, state) in orientation.iter().enumerate() {
        if state == &Some(false)
            && let Some(face) = faces.get_mut(face_index)
        {
            *face = [face[0], face[2], face[1]];
        }
    }
    for component in components {
        let volume = signed_volume_for_faces(&mesh.vertices, &faces, &component);
        if volume < -1.0e-12 {
            for face_index in component {
                if let Some(face) = faces.get_mut(face_index) {
                    *face = [face[0], face[2], face[1]];
                }
            }
        }
    }
    Mesh::new(mesh.vertices.clone(), faces)
}

fn close_holes(
    mesh: &Mesh,
    grid: RepairGrid,
    limits: MeshOperationLimits,
    maximum: usize,
) -> Result<Mesh, CoreError> {
    let loops = boundary_loops(mesh);
    native::fill_holes(mesh, grid, limits, &loops, maximum)
}

fn wrap_mesh(mesh: &Mesh) -> Result<Mesh, CoreError> {
    let compacted = compact_mesh(&mesh.vertices, &mesh.faces)?;
    fix_winding(&compacted)
}

fn remove_tiny_shells(mesh: &Mesh) -> Result<Mesh, CoreError> {
    let components = face_components(mesh);
    if components.len() < 2 {
        return Ok(mesh.clone());
    }
    let mut scales = Vec::with_capacity(components.len());
    for component in &components {
        let mut minimum = [f32::INFINITY; 3];
        let mut maximum = [f32::NEG_INFINITY; 3];
        for face_index in component {
            let Some(face) = mesh.faces.get(*face_index) else {
                continue;
            };
            for vertex_index in face {
                let Some(vertex) = mesh.vertices.get(*vertex_index as usize) else {
                    continue;
                };
                for axis in 0..3 {
                    minimum[axis] = minimum[axis].min(vertex[axis]);
                    maximum[axis] = maximum[axis].max(vertex[axis]);
                }
            }
        }
        let scale = (0..3)
            .map(|axis| maximum[axis] - minimum[axis])
            .fold(0.0_f32, f32::max);
        scales.push(scale);
    }
    let mut keep = vec![false; components.len()];
    for (index, scale) in scales.iter().copied().enumerate() {
        keep[index] = scale.is_finite() && scale > REPAIR_TINY_SHELL_TOLERANCE_MM;
    }
    if !keep.iter().any(|value| *value)
        && let Some((largest, _)) = scales
            .iter()
            .copied()
            .enumerate()
            .max_by(|(_, first), (_, second)| first.total_cmp(second))
    {
        keep[largest] = true;
    }
    let faces = components
        .iter()
        .enumerate()
        .filter(|(index, _)| keep[*index])
        .flat_map(|(_, faces)| faces.iter().filter_map(|index| mesh.faces.get(*index)))
        .copied()
        .collect::<Vec<_>>();
    compact_mesh(&mesh.vertices, &faces)
}

fn prepare_for_export(mesh: &Mesh) -> Result<Mesh, CoreError> {
    let cleaned = remove_degenerate_faces(mesh)?;
    let cleaned = remove_duplicate_faces(&cleaned)?;
    let cleaned = compact_mesh(&cleaned.vertices, &cleaned.faces)?;
    fix_winding(&cleaned)
}

fn compact_mesh(vertices: &[[f32; 3]], faces: &[[u32; 3]]) -> Result<Mesh, CoreError> {
    let mut remap = vec![None::<u32>; vertices.len()];
    let mut compact_vertices = Vec::new();
    for face in faces {
        for index in face {
            let slot = remap.get_mut(*index as usize).ok_or_else(|| {
                CoreError::InvalidMesh("repair face references a missing vertex".to_owned())
            })?;
            if slot.is_none() {
                let new_index = u32::try_from(compact_vertices.len()).map_err(|_| {
                    CoreError::OperationResourceLimit {
                        operation: "repair".to_owned(),
                        reason: "compact vertex index overflow".to_owned(),
                    }
                })?;
                *slot = Some(new_index);
                compact_vertices.push(vertices[*index as usize]);
            }
        }
    }
    let mut compact_faces = Vec::with_capacity(faces.len());
    for face in faces {
        let first = remap[face[0] as usize].ok_or_else(|| {
            CoreError::MeshOperation("repair lost a face vertex while compacting".to_owned())
        })?;
        let second = remap[face[1] as usize].ok_or_else(|| {
            CoreError::MeshOperation("repair lost a face vertex while compacting".to_owned())
        })?;
        let third = remap[face[2] as usize].ok_or_else(|| {
            CoreError::MeshOperation("repair lost a face vertex while compacting".to_owned())
        })?;
        compact_faces.push([first, second, third]);
    }
    Mesh::new(compact_vertices, compact_faces)
}

fn face_components(mesh: &Mesh) -> Vec<Vec<usize>> {
    let edges = native::edge_records(&mesh.faces);
    let mut adjacency = vec![Vec::<usize>::new(); mesh.face_count()];
    for owners in edges.chunk_by(|a, b| a.0 == b.0) {
        for first in 0..owners.len() {
            for second in (first + 1)..owners.len() {
                adjacency[owners[first].1].push(owners[second].1);
                adjacency[owners[second].1].push(owners[first].1);
            }
        }
    }
    let mut seen = vec![false; mesh.face_count()];
    let mut result = Vec::new();
    for start in 0..mesh.face_count() {
        if seen[start] {
            continue;
        }
        seen[start] = true;
        let mut component = Vec::new();
        let mut queue = VecDeque::from([start]);
        while let Some(face) = queue.pop_front() {
            component.push(face);
            for neighbor in &adjacency[face] {
                if !seen[*neighbor] {
                    seen[*neighbor] = true;
                    queue.push_back(*neighbor);
                }
            }
        }
        result.push(component);
    }
    result
}

fn boundary_loops(mesh: &Mesh) -> Vec<Vec<u32>> {
    let edges = native::edge_records(&mesh.faces);
    let mut outgoing = BTreeMap::<u32, Vec<u32>>::new();
    for owners in edges.chunk_by(|a, b| a.0 == b.0) {
        if owners.len() != 1 {
            continue;
        }
        let (key, face_index, _) = owners[0];
        let Some(face) = mesh.faces.get(face_index) else {
            continue;
        };
        let directed = [(face[0], face[1]), (face[1], face[2]), (face[2], face[0])]
            .into_iter()
            .find(|(first, second)| ordered_edge(*first, *second) == key);
        let Some((first, second)) = directed else {
            continue;
        };
        outgoing.entry(first).or_default().push(second);
    }
    let mut visited = BTreeSet::new();
    let mut loops = Vec::new();
    for (&start, neighbors) in &outgoing {
        for &first in neighbors {
            if visited.contains(&(start, first)) {
                continue;
            }
            let mut path = vec![start];
            let mut positions = BTreeMap::from([(start, 0)]);
            let mut previous = start;
            let mut current = first;
            visited.insert((start, first));
            loop {
                // Split a closed subloop even when a walk meets a shared
                // vertex before returning to its starting edge.
                if let Some(&position) = positions.get(&current) {
                    let cycle = path.split_off(position);
                    for vertex in &cycle {
                        positions.remove(vertex);
                    }
                    if cycle.len() >= 3 {
                        loops.push(cycle);
                    }
                }
                positions.insert(current, path.len());
                path.push(current);
                let Some(next) = outgoing.get(&current).and_then(|neighbors| {
                    neighbors
                        .iter()
                        .copied()
                        .filter(|next| !visited.contains(&(current, *next)))
                        .min_by(|left, right| {
                            // Smallest angle to the reverse incoming edge:
                            // maximize the normalized dot product. Compare in
                            // f64 so short boundary edges retain their direction.
                            let turn = |next: u32| {
                                let origin = mesh.vertices[current as usize].map(f64::from);
                                let incoming = mesh.vertices[previous as usize].map(f64::from);
                                let target = mesh.vertices[next as usize].map(f64::from);
                                let back = std::array::from_fn::<_, 3, _>(|axis| {
                                    incoming[axis] - origin[axis]
                                });
                                let forward = std::array::from_fn::<_, 3, _>(|axis| {
                                    target[axis] - origin[axis]
                                });
                                let length = (dot64(back, back) * dot64(forward, forward)).sqrt();
                                if length > 0.0 {
                                    dot64(back, forward) / length
                                } else {
                                    -1.0
                                }
                            };
                            turn(*right)
                                .total_cmp(&turn(*left))
                                .then_with(|| left.cmp(right))
                        })
                }) else {
                    break;
                };
                visited.insert((current, next));
                previous = current;
                current = next;
            }
        }
    }
    loops
}

fn is_safe_candidate(mesh: &Mesh) -> bool {
    !mesh.is_empty()
        && mesh.vertices.len() >= 3
        && mesh
            .vertices
            .iter()
            .flatten()
            .all(|value| value.is_finite())
        && mesh.faces.iter().all(|face| {
            face.iter()
                .all(|index| (*index as usize) < mesh.vertices.len())
        })
}

fn signed_volume_for_faces(vertices: &[[f32; 3]], faces: &[[u32; 3]], indices: &[usize]) -> f64 {
    indices
        .iter()
        .filter_map(|index| faces.get(*index))
        .filter_map(|face| {
            let first = vertices.get(face[0] as usize)?;
            let second = vertices.get(face[1] as usize)?;
            let third = vertices.get(face[2] as usize)?;
            let a = [
                f64::from(first[0]),
                f64::from(first[1]),
                f64::from(first[2]),
            ];
            let b = [
                f64::from(second[0]),
                f64::from(second[1]),
                f64::from(second[2]),
            ];
            let c = [
                f64::from(third[0]),
                f64::from(third[1]),
                f64::from(third[2]),
            ];
            Some(dot64(a, cross64(b, c)) / 6.0)
        })
        .sum()
}

fn ordered_edge(first: u32, second: u32) -> (u32, u32) {
    if first <= second {
        (first, second)
    } else {
        (second, first)
    }
}

pub(super) fn cross64(first: [f64; 3], second: [f64; 3]) -> [f64; 3] {
    [
        first[1] * second[2] - first[2] * second[1],
        first[2] * second[0] - first[0] * second[2],
        first[0] * second[1] - first[1] * second[0],
    ]
}

pub(super) fn dot64(first: [f64; 3], second: [f64; 3]) -> f64 {
    first[0] * second[0] + first[1] * second[1] + first[2] * second[2]
}

#[cfg(test)]
mod gate_tests {
    use super::{RepairScore, RepairVerdict, gate};

    #[test]
    fn more_shells_are_allowed_only_when_non_manifold_edges_decrease() {
        let before = RepairScore {
            open_edges: 0,
            non_manifold_edges: 2,
            flipped_edges: 0,
            degenerate_faces: 0,
            duplicate_faces: 0,
            inside_out_shells: 0,
            tiny_shells: 0,
            shells: 1,
        };
        let mut after = RepairScore {
            shells: 2,
            ..before
        };
        assert_eq!(gate(before, after), RepairVerdict::Worse);
        after.non_manifold_edges = 1;
        assert_eq!(gate(before, after), RepairVerdict::Improved);
        after.open_edges = 1;
        assert_eq!(gate(before, after), RepairVerdict::Worse);
    }

    #[test]
    fn every_defect_is_gated_and_improvements_cannot_mask_damage() {
        let score = |fields: [usize; 8]| RepairScore {
            open_edges: fields[0],
            non_manifold_edges: fields[1],
            flipped_edges: fields[2],
            degenerate_faces: fields[3],
            duplicate_faces: fields[4],
            inside_out_shells: fields[5],
            tiny_shells: fields[6],
            shells: fields[7],
        };
        let before = score([2; 8]);
        assert_eq!(gate(before, before), RepairVerdict::NoBetter);
        for field in 0..8 {
            let mut after = [2; 8];
            after[field] -= 1;
            assert_eq!(gate(before, score(after)), RepairVerdict::Improved);
            after[field] += 2;
            assert_eq!(gate(before, score(after)), RepairVerdict::Worse);
            after[(field + 1) % 8] -= 1;
            assert_eq!(gate(before, score(after)), RepairVerdict::Worse);
        }
    }
}

#[cfg(test)]
mod route_tests {
    use super::*;

    fn box_mesh(size: f32) -> Mesh {
        crate::make_box([size; 3]).expect("box")
    }

    #[test]
    fn time_estimate_and_limits_match_the_measured_curve() {
        assert_eq!(repair_time_estimate(0), Duration::ZERO);
        assert!((repair_time_estimate(4_000).as_secs_f64() - 0.1).abs() < 1.0e-8);
        assert!((repair_time_estimate(40_000).as_secs_f64() - 1.90546).abs() < 0.0001);
        assert_eq!(repair_time_limit(4_000), Duration::from_secs(30));
        assert_eq!(repair_time_limit(0), Duration::from_secs(30));
        assert_eq!(repair_time_limit(10_000_000), Duration::from_secs(600));
        assert_eq!(repair_time_limit(usize::MAX), Duration::from_secs(600));
        assert_eq!(
            repair_time_limit(1_000_000),
            repair_time_estimate(1_000_000).saturating_mul(3)
        );
    }

    #[test]
    fn collapse_checks_both_ends_of_every_axis() {
        let before = box_mesh(10.0);
        let health = inspect_mesh(&before).expect("health");
        for axis in 0..3 {
            for end in [-1.0, 1.0] {
                let mut after = before.clone();
                for point in &mut after.vertices {
                    if point[axis].signum() == end {
                        point[axis] -= end * 0.11;
                    }
                }
                assert!(collapsed(&before, &health, &after).is_some());
            }
        }
        let mut shifted = before.clone();
        for point in &mut shifted.vertices {
            point[0] += 0.09;
        }
        assert!(collapsed(&before, &health, &shifted).is_none());
        assert!(collapsed(&before, &health, &before).is_none());
        assert!(
            collapsed(
                &before,
                &health,
                &Mesh {
                    vertices: vec![],
                    faces: vec![]
                }
            )
            .is_some()
        );
    }

    #[test]
    fn collapse_volume_guard_only_applies_to_positive_closed_single_manifolds() {
        // Small bounds losses stay below 0.1 mm while volume drops by 5.9%.
        let before = box_mesh(1.0);
        let after = box_mesh(0.98);
        let health = inspect_mesh(&before).expect("health");
        assert!(collapsed(&before, &health, &after).is_some());
        assert!(collapsed(&before, &health, &box_mesh(0.995)).is_none());
        let mut open = before.clone();
        open.faces.pop();
        let mut multiple = before.clone();
        multiple = Mesh::concatenate(&[multiple, box_mesh(0.5)]).expect("parts");
        let mut non_manifold = before.clone();
        non_manifold.faces.push(non_manifold.faces[0]);
        let mut inward = before.clone();
        for face in &mut inward.faces {
            face.swap(1, 2);
        }
        for source in [open, multiple, non_manifold, inward] {
            assert!(collapsed(&source, &inspect_mesh(&source).expect("health"), &after).is_none());
        }
    }

    #[test]
    fn bounding_box_guard_also_protects_open_and_multi_shell_inputs() {
        let mut open = box_mesh(10.0);
        open.faces.pop();
        let multi = Mesh::concatenate(&[box_mesh(10.0), box_mesh(5.0)]).expect("parts");
        // Closed multi-shell input is held to 0.1 mm; open input only to a
        // large loss, because loose sheets and fins legitimately go.
        assert!(
            collapsed(
                &multi,
                &inspect_mesh(&multi).expect("health"),
                &box_mesh(9.7)
            )
            .is_some()
        );
        assert!(collapsed(&open, &inspect_mesh(&open).expect("health"), &box_mesh(4.0)).is_some());
        assert!(collapsed(&open, &inspect_mesh(&open).expect("health"), &box_mesh(9.7)).is_none());
    }

    #[test]
    fn windows_candidate_requires_improvement_and_preserved_bounds() {
        let mut damaged = box_mesh(10.0);
        damaged.faces.push(damaged.faces[0]);
        let before = inspect_mesh(&damaged).expect("health");
        assert!(windows_candidate(&damaged, &before, damaged.clone()).is_err());
        let shrunken = windows_candidate(&damaged, &before, box_mesh(4.0)).expect_err("collapsed");
        assert!(shrunken.contains("shrank"));
        let fixed = windows_candidate(&damaged, &before, box_mesh(10.0)).expect("improved");
        assert!(fixed.report.can_apply);
        assert_eq!(fixed.report.engine, RepairEngine::Windows);
    }

    #[test]
    fn catastrophic_fallback_loss_is_distinct_from_legitimate_volume_changes() {
        let mut open = box_mesh(10.0);
        open.faces.pop();
        assert!(nearly_empty_fallback(&open, &box_mesh(0.1)));
        assert!(!nearly_empty_fallback(&open, &box_mesh(9.0)));
        let mut sheet = box_mesh(10.0);
        for point in &mut sheet.vertices {
            point[1] = 0.0;
        }
        assert!(nearly_empty_fallback(&open, &sheet));
        assert!(!nearly_empty_fallback(&sheet, &sheet));
    }

    #[test]
    fn disabled_windows_matches_native_and_cancellation_returns_no_candidate() {
        let mut mesh = box_mesh(10.0);
        mesh.faces.pop();
        let native = repair_mesh(&mesh).expect("native");
        let routed = repair_mesh_routed(
            &mesh,
            RepairRouteControl {
                allow_windows: false,
                cancelled: &|| false,
                progress: None,
            },
        )
        .expect("routed");
        assert_eq!(routed, native);
        let error = repair_mesh_routed(
            &mesh,
            RepairRouteControl {
                allow_windows: true,
                cancelled: &|| true,
                progress: None,
            },
        )
        .expect_err("cancelled");
        assert!(
            matches!(error, CoreError::MeshOperation(message) if message == "Repair cancelled")
        );
    }

    #[cfg(not(windows))]
    #[test]
    fn non_windows_route_falls_back_to_the_same_native_result() {
        let mut mesh = box_mesh(10.0);
        mesh.faces.pop();
        let native = repair_mesh(&mesh).expect("native");
        let mut routed = repair_mesh_routed(
            &mesh,
            RepairRouteControl {
                allow_windows: true,
                cancelled: &|| false,
                progress: None,
            },
        )
        .expect("routed");
        assert!(routed.report.engine_note.take().is_some());
        assert_eq!(routed, native);
    }
}
