use builder3d_core::{
    Mesh, MeshOperationLimits, Scene, SceneObject, make_box, subdivide_mesh,
    subdivide_mesh_with_options,
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
