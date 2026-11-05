use builder3d_core::{
    CoreError, Mesh, RepairProfile, inspect_mesh, make_box, repair_mesh, repair_mesh_with_profile,
};

fn box_mesh(extents: [f32; 3]) -> Mesh {
    make_box(extents).expect("box fixture")
}

fn signed_volume(mesh: &Mesh) -> f64 {
    mesh.faces
        .iter()
        .map(|face| {
            let a = mesh.vertices[face[0] as usize];
            let b = mesh.vertices[face[1] as usize];
            let c = mesh.vertices[face[2] as usize];
            let cross = [
                f64::from(b[1]) * f64::from(c[2]) - f64::from(b[2]) * f64::from(c[1]),
                f64::from(b[2]) * f64::from(c[0]) - f64::from(b[0]) * f64::from(c[2]),
                f64::from(b[0]) * f64::from(c[1]) - f64::from(b[1]) * f64::from(c[0]),
            ];
            (f64::from(a[0]) * cross[0] + f64::from(a[1]) * cross[1] + f64::from(a[2]) * cross[2])
                / 6.0
        })
        .sum::<f64>()
        .abs()
}

fn remove_top_of_box(mesh: &Mesh) -> Mesh {
    let top = mesh
        .vertices
        .iter()
        .map(|vertex| vertex[1])
        .fold(f32::NEG_INFINITY, f32::max);
    let faces = mesh
        .faces
        .iter()
        .copied()
        .filter(|face| {
            !face
                .iter()
                .all(|index| (mesh.vertices[*index as usize][1] - top).abs() <= 1.0e-5)
        })
        .collect();
    Mesh::new(mesh.vertices.clone(), faces).expect("open box fixture")
}

fn flat_fan_box() -> Mesh {
    let source = box_mesh([4.0, 4.0, 4.0]);
    let top = source
        .faces
        .iter()
        .copied()
        .filter(|face| {
            face.iter()
                .all(|index| (source.vertices[*index as usize][1] - 2.0).abs() <= 1.0e-5)
        })
        .collect::<Vec<_>>();
    let corners = [
        source
            .vertices
            .iter()
            .position(|vertex| vertex == &[-2.0, 2.0, -2.0])
            .expect("bottom-left"),
        source
            .vertices
            .iter()
            .position(|vertex| vertex == &[-2.0, 2.0, 2.0])
            .expect("top-left"),
        source
            .vertices
            .iter()
            .position(|vertex| vertex == &[2.0, 2.0, 2.0])
            .expect("top-right"),
        source
            .vertices
            .iter()
            .position(|vertex| vertex == &[2.0, 2.0, -2.0])
            .expect("bottom-right"),
    ];
    let center = u32::try_from(source.vertices.len()).expect("center index");
    let mut vertices = source.vertices.clone();
    vertices.push([0.0, 2.0, 0.0]);
    let mut faces = source
        .faces
        .iter()
        .copied()
        .filter(|face| !top.contains(face))
        .collect::<Vec<_>>();
    faces.extend([
        [corners[0] as u32, corners[1] as u32, center],
        [corners[1] as u32, corners[2] as u32, center],
        [corners[2] as u32, corners[3] as u32, center],
        [corners[3] as u32, corners[0] as u32, center],
    ]);
    Mesh::new(vertices, faces).expect("flat fan")
}

fn flat_grid(divisions: u32) -> Mesh {
    let width = divisions + 1;
    let mut vertices = Vec::new();
    for row in 0..=divisions {
        for column in 0..=divisions {
            vertices.push([column as f32, 0.0, row as f32]);
        }
    }
    let mut faces = Vec::new();
    for row in 0..divisions {
        for column in 0..divisions {
            let bottom_left = row * width + column;
            let bottom_right = bottom_left + 1;
            let top_left = bottom_left + width;
            let top_right = top_left + 1;
            faces.push([bottom_left, top_left, bottom_right]);
            faces.push([bottom_right, top_left, top_right]);
        }
    }
    Mesh::new(vertices, faces).expect("flat grid")
}

#[test]
fn repair_fixes_winding_without_mutating_source_and_keeps_stage_contract() {
    let source = box_mesh([1.0, 1.0, 1.0]);
    let mut flipped = source.clone();
    for face in &mut flipped.faces {
        *face = [face[0], face[2], face[1]];
    }
    let original = flipped.clone();
    let result = repair_mesh(&flipped).expect("repair flipped box");
    assert!(result.report.after.valid);
    assert_eq!(result.report.profile, "extended");
    assert_eq!(result.report.stages.len(), 12);
    assert_eq!(flipped, original);
    assert!((signed_volume(&result.mesh) - 1.0).abs() < 1.0e-4);
}

#[test]
fn extended_repair_closes_a_trivial_open_box_and_reports_exact_stage_names() {
    let source = remove_top_of_box(&box_mesh([2.0, 2.0, 2.0]));
    let result = repair_mesh(&source).expect("repair open box");
    assert!(!result.report.before.valid);
    assert!(result.report.after.valid);
    assert_eq!(result.report.after.boundary_edges, 0);
    let names = result
        .report
        .stages
        .iter()
        .map(|stage| stage.name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        vec![
            "StitchGaps (insert vertices)",
            "CloseTrivialHoles",
            "RemoveDoubleTriangles",
            "FixFlippedTriangles",
            "CloseTrivialHoles (second pass)",
            "StitchGaps (reuse vertices)",
            "FixFlippedTriangles (second pass)",
            "CloseAllHoles",
            "Wrap",
            "RemoveDegeneratedFaces",
            "RemoveTinyShells",
            "PrepareForExport",
        ]
    );
}

#[test]
fn repair_removes_duplicate_and_tiny_shell_but_keeps_safe_best_effort_multi_shell() {
    let main = box_mesh([2.0, 2.0, 2.0]);
    let tiny = box_mesh([0.001, 0.001, 0.001]);
    let mut vertices = main.vertices.clone();
    let offset = u32::try_from(vertices.len()).expect("offset");
    vertices.extend(
        tiny.vertices
            .iter()
            .map(|vertex| [vertex[0] + 5.0, vertex[1], vertex[2]]),
    );
    let mut faces = main.faces.clone();
    faces.extend(
        tiny.faces
            .iter()
            .map(|face| [face[0] + offset, face[1] + offset, face[2] + offset]),
    );
    faces.push(main.faces[0]);
    let source = Mesh::new(vertices, faces).expect("multi-shell duplicate");
    let result = repair_mesh(&source).expect("repair duplicate shells");
    assert!(result.report.candidate_safe);
    assert!(result.report.candidate_changed);
    assert!(result.report.after.valid);
    assert_eq!(result.report.after.body_count, 1);
    assert_eq!(result.mesh.face_count(), main.face_count());
}

#[test]
fn planar_cleanup_changes_a_fan_and_leaves_dense_grid_open_and_duplicate_free() {
    let source = flat_fan_box();
    let source_volume = signed_volume(&source);
    let result = repair_mesh(&source).expect("repair flat fan");
    assert!(result.report.planar_cleanup.changed);
    assert!(result.report.after.valid);
    assert!(result.report.planar_cleanup.details[0].faces_after > 4);
    assert!((signed_volume(&result.mesh) - source_volume).abs() < 1.0e-4);

    let grid = flat_grid(4);
    let grid_result = repair_mesh(&grid).expect("repair flat grid");
    assert!(grid_result.report.planar_cleanup.changed);
    assert_eq!(grid_result.report.after.duplicate_faces, 0);
    assert_eq!(grid_result.report.after.non_manifold_edges, 0);
    assert!(grid_result.report.after.boundary_edges > 0);
    assert!(grid_result.mesh.face_count() <= grid.face_count());
}

#[test]
fn repair_profiles_and_health_are_typed_and_bounded() {
    let source = box_mesh([1.0, 1.0, 1.0]);
    let simple = repair_mesh_with_profile(&source, "simple").expect("simple repair");
    assert!(simple.report.after.valid);
    assert_eq!(simple.report.stages.len(), 4);
    assert_eq!(
        RepairProfile::parse("extended").expect("profile"),
        RepairProfile::Extended
    );
    let error = repair_mesh_with_profile(&source, "cloud").expect_err("unknown profile");
    assert!(
        matches!(error, CoreError::MeshOperation(message) if message.contains("Unknown repair profile"))
    );

    let report = inspect_mesh(&grid_for_health()).expect("health report");
    assert_eq!(report.boundary_edges, 4);
    assert_eq!(report.boundary_edge_loops, 1);
    assert!(!report.valid);
}

fn grid_for_health() -> Mesh {
    Mesh::new(
        vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
            [1.0, 0.0, 1.0],
        ],
        vec![[0, 1, 2], [1, 3, 2]],
    )
    .expect("health grid")
}
