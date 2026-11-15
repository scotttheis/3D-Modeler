use builder3d_core::{
    Mesh, MeshOperationLimits, Scene, SceneObject, faces_near_path, make_box, make_sphere,
    subdivide_mesh, subdivide_mesh_even, subdivide_mesh_with_options,
};

fn volume(mesh: &Mesh) -> f64 {
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

#[test]
fn subdivide_doubles_box_faces_preserves_volume_bounds_and_watertightness() {
    let source = make_box([2.0, 2.0, 2.0]).expect("box fixture");
    let result = subdivide_mesh(&source).expect("subdivide box");
    assert_eq!(result.mesh.face_count(), source.face_count() * 2);
    assert_eq!(result.mesh.extents(), source.extents());
    assert!((volume(&result.mesh) - volume(&source)).abs() < 1.0e-4);
    let health = builder3d_core::inspect_mesh(&result.mesh).expect("health");
    assert!(health.valid);
    assert!(health.watertight);
    assert_eq!(result.report.selected_boundary_edges, 0);
}

#[test]
fn subdivide_is_atomic_in_scene_history_and_preserves_object_identity() {
    let source = make_box([2.0, 4.0, 6.0]).expect("box fixture");
    let mut scene = Scene::new();
    scene
        .add_object(SceneObject::new(source.clone()))
        .expect("add box");
    let id = scene.objects()[0].object_id().to_owned();
    let (command, report) = scene.apply_subdivide(0).expect("scene subdivide");
    assert!(command.changed);
    assert!(report.changed);
    assert_eq!(scene.objects()[0].object_id(), id);
    scene.undo().expect("undo subdivide");
    assert_eq!(scene.objects()[0].mesh().face_count(), source.face_count());
    scene.redo().expect("redo subdivide");
    assert_eq!(
        scene.objects()[0].mesh().face_count(),
        source.face_count() * 2
    );
}

#[test]
fn subdivide_checks_result_limits_before_allocating_midpoints() {
    let source = make_box([2.0, 2.0, 2.0]).expect("box fixture");
    let limits = MeshOperationLimits {
        max_faces: 12,
        ..MeshOperationLimits::default()
    };
    let error = subdivide_mesh_with_options(&source, limits).expect_err("face limit");
    assert!(matches!(
        error,
        builder3d_core::CoreError::OperationResourceLimit { .. }
    ));
}

#[test]
fn subdivide_rejects_empty_face_sets_with_a_typed_error() {
    let empty = Mesh::new(vec![[0.0, 0.0, 0.0]], Vec::new()).expect("empty mesh fixture");
    let error = subdivide_mesh(&empty).expect_err("empty subdivision");
    assert!(matches!(error, builder3d_core::CoreError::InvalidMesh(_)));
}

fn edge_lengths(mesh: &Mesh) -> Vec<f32> {
    let mut lengths = Vec::new();
    for face in &mesh.faces {
        for (first, second) in [(face[0], face[1]), (face[1], face[2]), (face[2], face[0])] {
            let a = mesh.vertices[first as usize];
            let b = mesh.vertices[second as usize];
            lengths.push(
                ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt(),
            );
        }
    }
    lengths
}

/// Every edge is shared by exactly two faces (closed, no cracks).
fn is_closed(mesh: &Mesh) -> bool {
    let mut counts = std::collections::BTreeMap::<(u32, u32), usize>::new();
    for face in &mesh.faces {
        for (first, second) in [(face[0], face[1]), (face[1], face[2]), (face[2], face[0])] {
            *counts
                .entry((first.min(second), first.max(second)))
                .or_default() += 1;
        }
    }
    counts.values().all(|count| *count == 2)
}

#[test]
fn even_subdivision_cuts_every_face_into_four_and_keeps_the_shape() {
    // OF-52: the old doubling split only some edges, so detail was uneven.
    let source = make_sphere(10.0, 2).expect("sphere");
    let result = subdivide_mesh_even(&source, None, MeshOperationLimits::default()).expect("even");
    assert_eq!(result.mesh.face_count(), source.face_count() * 4);
    assert!(is_closed(&result.mesh));
    assert!((volume(&result.mesh) - volume(&source)).abs() < 1.0e-3 * volume(&source));
    // Every new edge is half an old one: the spread of edge lengths is the
    // same as before, just halved.
    let before = edge_lengths(&source);
    let after = edge_lengths(&result.mesh);
    let spread = |lengths: &[f32]| {
        let max = lengths.iter().copied().fold(f32::MIN, f32::max);
        let min = lengths.iter().copied().fold(f32::MAX, f32::min);
        max / min
    };
    assert!((spread(&after) - spread(&before)).abs() < 0.01);
}

#[test]
fn even_subdivision_of_a_brushed_area_stays_closed_and_local() {
    let source = make_sphere(10.0, 2).expect("sphere");
    let faces = faces_near_path(&source, &[[0.0, 0.0, 10.0], [3.0, 0.0, 9.5]], 3.0);
    assert!(!faces.is_empty());
    let result =
        subdivide_mesh_even(&source, Some(&faces), MeshOperationLimits::default()).expect("area");
    assert!(is_closed(&result.mesh));
    assert!(result.mesh.face_count() >= source.face_count() + faces.len() * 3);
    assert!(
        result.mesh.face_count() < source.face_count() * 2,
        "only the area is refined"
    );
    // The far side of the sphere is untouched.
    let far = |mesh: &Mesh| {
        mesh.vertices
            .iter()
            .filter(|vertex| vertex[2] < -5.0)
            .count()
    };
    assert_eq!(far(&result.mesh), far(&source));
    let error = subdivide_mesh_even(
        &source,
        Some(&[source.face_count()]),
        MeshOperationLimits::default(),
    )
    .expect_err("out of range face");
    assert!(error.to_string().contains("outside"));
}
