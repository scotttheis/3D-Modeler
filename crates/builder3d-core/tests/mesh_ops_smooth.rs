use builder3d_core::{
    MeshOperationLimits, Scene, SceneObject, make_box, smooth_mesh, smooth_mesh_with_options,
};

#[test]
fn smooth_runs_a_bounded_iteration_and_keeps_topology_and_finite_positions() {
    let source = make_box([2.0, 2.0, 2.0]).expect("box fixture");
    let result = smooth_mesh(&source, 1).expect("smooth box");
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
        builder3d_core::CoreError::OperationResourceLimit { .. }
    ));
    let limits = MeshOperationLimits {
        max_work_units: 1,
        ..MeshOperationLimits::default()
    };
    let error = smooth_mesh_with_options(&source, 1, limits).expect_err("work limit");
    assert!(matches!(
        error,
        builder3d_core::CoreError::OperationResourceLimit { .. }
    ));
}

#[test]
fn scene_smooth_preserves_identity_and_history() {
    let source = make_box([2.0, 2.0, 2.0]).expect("box fixture");
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
