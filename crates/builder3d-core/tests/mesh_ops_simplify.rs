use builder3d_core::{
    Mesh, MeshOperationLimits, Scene, SceneObject, make_box, make_sphere, simplify_mesh,
    simplify_mesh_with_options,
};

#[test]
fn simplify_reduces_a_sphere_to_the_requested_bounded_target() {
    let source = make_sphere(1.0, 3).expect("sphere fixture");
    let result = simplify_mesh(&source, Some(100)).expect("simplify sphere");
    assert!(result.report.changed);
    assert!(result.mesh.face_count() < source.face_count());
    assert!(result.mesh.face_count() <= 100);
    assert!(result.mesh.face_count() > 0);
    assert!(result.report.collapsed_edges > 0);
    assert!(
        result
            .mesh
            .vertices
            .iter()
            .flatten()
            .all(|value| value.is_finite())
    );
}

#[test]
fn simplify_is_a_no_op_for_small_meshes_and_preserves_input() {
    let source = make_box([1.0, 1.0, 1.0]).expect("box fixture");
    let original = source.clone();
    let result = simplify_mesh(&source, Some(100)).expect("small simplify");
    assert!(!result.report.changed);
    assert_eq!(result.mesh, source);
    assert_eq!(source, original);
}

#[test]
fn simplify_rejects_limits_before_mutation_and_reports_typed_errors() {
    let source = make_box([2.0, 2.0, 2.0]).expect("box fixture");
    let limits = MeshOperationLimits {
        max_vertices: 3,
        ..MeshOperationLimits::default()
    };
    let error = simplify_mesh_with_options(&source, Some(4), limits).expect_err("vertex limit");
    assert!(matches!(
        error,
        builder3d_core::CoreError::OperationResourceLimit { .. }
    ));
    assert_eq!(source.face_count(), 12);
}

#[test]
fn simplify_accepts_a_valid_mesh_contract_with_explicit_faces() {
    let mesh = Mesh::new(
        vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        vec![[0, 1, 2]],
    )
    .expect("triangle");
    let result = simplify_mesh(&mesh, None).expect("triangle no-op");
    assert_eq!(result.mesh, mesh);
    assert_eq!(result.report.after_faces, 1);
}

#[test]
fn scene_simplify_is_atomic_and_undoable_without_replacing_object_identity() {
    let source = make_sphere(1.0, 2).expect("sphere fixture");
    let mut scene = Scene::new();
    scene
        .add_object(SceneObject::new(source.clone()))
        .expect("add sphere");
    let object_id = scene.objects()[0].object_id().to_owned();
    let before_faces = scene.objects()[0].mesh().face_count();
    let (command, report) = scene.apply_simplify(0, Some(40)).expect("scene simplify");
    assert!(command.changed);
    assert!(report.changed);
    assert!(scene.objects()[0].mesh().face_count() <= 40);
    assert_eq!(scene.objects()[0].object_id(), object_id);
    scene.undo().expect("undo simplify");
    assert_eq!(scene.objects()[0].mesh().face_count(), before_faces);
    scene.redo().expect("redo simplify");
    assert!(scene.objects()[0].mesh().face_count() <= 40);
}
