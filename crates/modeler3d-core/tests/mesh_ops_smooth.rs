use modeler3d_core::{
    DEFAULT_SMOOTH_MAX_ITERATIONS, Mesh, MeshOperationLimits, Scene, SceneObject, make_box,
    make_cylinder, make_hexagon, make_sphere, smooth_mesh, smooth_mesh_with_options,
};

fn extent(mesh: &Mesh, axis: usize) -> f32 {
    let values = mesh.vertices.iter().map(|vertex| vertex[axis]);
    let low = values.clone().fold(f32::INFINITY, f32::min);
    let high = values.fold(f32::NEG_INFINITY, f32::max);
    high - low
}

fn volume(mesh: &Mesh) -> f64 {
    mesh.faces
        .iter()
        .map(|face| {
            let [a, b, c] = face.map(|index| mesh.vertices[index as usize].map(f64::from));
            (a[0] * (b[1] * c[2] - b[2] * c[1])
                + a[1] * (b[2] * c[0] - b[0] * c[2])
                + a[2] * (b[0] * c[1] - b[1] * c[0]))
                / 6.0
        })
        .sum::<f64>()
        .abs()
}

fn bumpy_sphere() -> Mesh {
    let mut source = make_sphere(10.0, 4).expect("sphere fixture");
    for (index, vertex) in source.vertices.iter_mut().enumerate() {
        let bump = if index % 3 == 0 { 1.02 } else { 0.99 };
        for value in vertex.iter_mut() {
            *value *= bump;
        }
    }
    source
}

fn radii(mesh: &Mesh) -> Vec<f64> {
    mesh.vertices
        .iter()
        .map(|vertex| {
            vertex
                .iter()
                .map(|value| f64::from(*value).powi(2))
                .sum::<f64>()
                .sqrt()
        })
        .collect()
}

fn mean_radius(mesh: &Mesh) -> f64 {
    let radii = radii(mesh);
    radii.iter().sum::<f64>() / radii.len() as f64
}

/// Standard deviation of the vertex distance from the origin, divided by the
/// mean distance: zero for a perfect sphere of any size, so it measures bumps
/// and ignores a uniform shrink.
fn roughness(mesh: &Mesh) -> f64 {
    let radii = radii(mesh);
    let mean = mean_radius(mesh);
    (radii
        .iter()
        .map(|radius| (radius - mean).powi(2))
        .sum::<f64>()
        / radii.len() as f64)
        .sqrt()
        / mean
}

#[test]
fn smooth_keeps_a_sparse_stock_cylinder_from_melting() {
    // H1 / D9: whole-object Smooth follows Blender's sculpt Smooth with the
    // same hard-edge rule as the Smooth Brush, so the two-ring stock
    // cylinder keeps its height, width and volume even at the most passes.
    let source = make_cylinder(20.0, 40.0, 64).expect("cylinder fixture");
    let result = smooth_mesh(&source, DEFAULT_SMOOTH_MAX_ITERATIONS).expect("smooth cylinder");
    for axis in 0..3 {
        let before = extent(&source, axis);
        let after = extent(&result.mesh, axis);
        assert!(after >= before * 0.97, "axis {axis}: {before} -> {after}");
    }
    assert!(volume(&result.mesh) >= volume(&source) * 0.95);
    assert_eq!(result.mesh.face_count(), source.face_count());
}

#[test]
fn smooth_leaves_box_corners_in_place() {
    // Every box vertex is a corner where three sharp edges meet, so Smooth
    // has nothing to move and reports no change (no history entry).
    let source = make_box([20.0, 30.0, 40.0]).expect("box fixture");
    let result = smooth_mesh(&source, 3).expect("smooth box");
    assert!(!result.report.changed);
    assert_eq!(result.mesh, source);
}

#[test]
fn smooth_still_evens_out_a_bumpy_surface() {
    let source = bumpy_sphere();
    let result = smooth_mesh(&source, 1).expect("smooth sphere");
    assert!(result.report.changed);
    let (before, after) = (roughness(&source), roughness(&result.mesh));
    assert!(after < before * 0.6, "roughness {before} -> {after}");
    let (before, after) = (mean_radius(&source), mean_radius(&result.mesh));
    assert!(after > before * 0.97, "mean radius {before} -> {after}");
}

#[test]
fn smooth_works_the_same_on_a_tiny_mesh() {
    // Sharp-edge detection uses relative normals, so a bumpy sphere shrunk to
    // a hundred-thousandth of its size still smooths (no all-corner no-op).
    let mut source = bumpy_sphere();
    for vertex in &mut source.vertices {
        for value in vertex.iter_mut() {
            *value *= 1.0e-5;
        }
    }
    let result = smooth_mesh(&source, 1).expect("smooth tiny sphere");
    assert!(result.report.changed);
    let (before, after) = (roughness(&source), roughness(&result.mesh));
    assert!(after < before * 0.6, "roughness {before} -> {after}");
}

#[test]
fn smooth_keeps_open_border_corners() {
    // A flat 2 x 2 mm sheet of eight triangles: its border is one sharp edge
    // line that turns 90° at each corner, so the corners stay and the sheet
    // keeps its size; mid-border points only slide along the border.
    let vertices = (0..9)
        .map(|index| [(index % 3) as f32, (index / 3) as f32, 0.0])
        .collect::<Vec<_>>();
    let mut faces = Vec::new();
    for row in 0..2_u32 {
        for column in 0..2_u32 {
            let corner = row * 3 + column;
            faces.push([corner, corner + 1, corner + 4]);
            faces.push([corner, corner + 4, corner + 3]);
        }
    }
    let source = Mesh::new(vertices, faces).expect("sheet fixture");
    let result = smooth_mesh(&source, DEFAULT_SMOOTH_MAX_ITERATIONS).expect("smooth sheet");
    for corner in [0, 2, 6, 8] {
        assert_eq!(result.mesh.vertices[corner], source.vertices[corner]);
    }
    for axis in 0..2 {
        assert_eq!(extent(&result.mesh, axis), 2.0);
    }
}

#[test]
fn smooth_keeps_a_hexagon_prism() {
    // The hexagon's rims turn 60° at each vertex, so every rim vertex is a
    // corner and the prism keeps its exact shape.
    let source = make_hexagon(28.7935, 15.48893).expect("hexagon fixture");
    let result = smooth_mesh(&source, DEFAULT_SMOOTH_MAX_ITERATIONS).expect("smooth hexagon");
    assert_eq!(result.mesh, source);
}

#[test]
fn smooth_ignores_a_degenerate_face_on_a_bumpy_surface() {
    // A zero-area face [v, v, v] must not pin v as a corner or give it
    // itself as a neighbour.
    let source = bumpy_sphere();
    let mut faces = source.faces.clone();
    faces.push([0, 0, 0]);
    let with_degenerate = Mesh::new(source.vertices.clone(), faces).expect("degenerate fixture");
    let plain = smooth_mesh(&source, 1).expect("smooth plain");
    let result = smooth_mesh(&with_degenerate, 1).expect("smooth with degenerate face");
    assert_eq!(result.mesh.vertices, plain.mesh.vertices);
}

#[test]
fn smooth_runs_a_bounded_iteration_and_keeps_topology_and_finite_positions() {
    let source = bumpy_sphere();
    let result = smooth_mesh(&source, 1).expect("smooth sphere");
    assert!(result.report.changed);
    assert_eq!(result.mesh.face_count(), source.face_count());
    assert_eq!(result.mesh.vertex_count(), source.vertex_count());
    assert!(
        result
            .mesh
            .vertices
            .iter()
            .flatten()
            .all(|value| value.is_finite())
    );
    assert!(result.report.maximum_displacement_mm > 0.0);
}

#[test]
fn smooth_zero_iterations_is_an_exact_no_op() {
    let source = make_box([2.0, 4.0, 6.0]).expect("box fixture");
    let result = smooth_mesh(&source, 0).expect("zero smooth");
    assert!(!result.report.changed);
    assert_eq!(result.mesh, source);
    assert_eq!(result.report.applied_iterations, 0);
}

#[test]
fn smooth_rejects_excess_iterations_and_resource_work_before_commit() {
    let source = make_box([1.0, 1.0, 1.0]).expect("box fixture");
    let error = smooth_mesh(&source, 11).expect_err("iteration limit");
    assert!(matches!(
        error,
        modeler3d_core::CoreError::OperationResourceLimit { .. }
    ));
    let limits = MeshOperationLimits {
        max_work_units: 1,
        ..MeshOperationLimits::default()
    };
    let error = smooth_mesh_with_options(&source, 1, limits).expect_err("work limit");
    assert!(matches!(
        error,
        modeler3d_core::CoreError::OperationResourceLimit { .. }
    ));
}

#[test]
fn scene_smooth_preserves_identity_and_history() {
    let source = bumpy_sphere();
    let mut scene = Scene::new();
    scene
        .add_object(SceneObject::new(source.clone()))
        .expect("add box");
    let id = scene.objects()[0].object_id().to_owned();
    let (command, report) = scene.apply_smooth(0, 1).expect("scene smooth");
    assert!(command.changed);
    assert!(report.changed);
    assert_eq!(scene.objects()[0].object_id(), id);
    scene.undo().expect("undo smooth");
    assert_eq!(scene.objects()[0].mesh(), &source);
    scene.redo().expect("redo smooth");
    assert_eq!(scene.objects()[0].object_id(), id);
}
