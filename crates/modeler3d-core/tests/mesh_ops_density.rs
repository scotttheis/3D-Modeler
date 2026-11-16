use modeler3d_core::{
    CommandRegistry, CommandRequest, CoreError, Mesh, MeshOperationLimits, NormalizeDensityOptions,
    Scene, SceneObject, make_box, make_sphere, normalize_face_density,
    normalize_face_density_with_hooks, select_face_region,
};

#[test]
fn select_face_region_returns_faces_near_each_center() {
    let source = make_box([10.0, 10.0, 10.0]).expect("box fixture");
    let selected = select_face_region(&source, &[[0.0, 0.0, 5.0]], 0.1).expect("selection");
    assert!(!selected.is_empty());
    assert!(selected.iter().all(|index| *index < source.face_count()));
}

#[test]
fn density_normalization_refines_a_partial_region_without_mutating_input() {
    let source = make_sphere(10.0, 2).expect("sphere fixture");
    let original = source.clone();
    let selected = (0..source.face_count() / 2).collect::<Vec<_>>();
    let result = normalize_face_density(
        &source,
        Some(&selected),
        None,
        NormalizeDensityOptions {
            target_edge_length_mm: Some(1.0),
            max_iterations: 2,
            ..NormalizeDensityOptions::default()
        },
    )
    .expect("partial density normalization");
    assert_eq!(source, original);
    assert!(result.report.changed);
    assert!(result.mesh.face_count() > source.face_count());
    assert!(result.report.subdivided_faces > 0);
    assert!(result.report.boundary_preserved);
    assert!(result.report.selection_boundary_preserved);
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
fn density_normalization_coarsens_a_dense_closed_region() {
    let source = make_sphere(3.0, 3).expect("sphere fixture");
    let selected = (0..source.face_count()).collect::<Vec<_>>();
    let result = normalize_face_density(
        &source,
        Some(&selected),
        None,
        NormalizeDensityOptions {
            target_edge_length_mm: Some(3.0),
            max_iterations: 2,
            ..NormalizeDensityOptions::default()
        },
    )
    .expect("dense density normalization");
    assert!(result.report.changed);
    assert!(result.mesh.face_count() < source.face_count());
    assert!(result.report.collapsed_edges > 0);
    assert!(result.report.boundary_preserved);
    assert!(result.report.geometric_deviation_mm.is_finite());
}

#[test]
fn density_normalization_reports_progress_and_honors_work_limits() {
    let source = make_box([2.0, 2.0, 2.0]).expect("box fixture");
    let selected = (0..source.face_count()).collect::<Vec<_>>();
    let mut progress_values = Vec::new();
    let mut cancel_calls = 0_usize;
    let mut cancel = || {
        cancel_calls = cancel_calls.saturating_add(1);
        Ok(())
    };
    let mut progress = |percent, _message: &str| progress_values.push(percent);
    let result = normalize_face_density_with_hooks(
        &source,
        Some(&selected),
        None,
        NormalizeDensityOptions {
            target_edge_length_mm: Some(1.0),
            max_iterations: 1,
            ..NormalizeDensityOptions::default()
        },
        Some(&mut cancel),
        Some(&mut progress),
    )
    .expect("hooked density normalization");
    assert!(cancel_calls > 1);
    assert_eq!(progress_values.first(), Some(&5));
    assert_eq!(progress_values.last(), Some(&100));
    assert!(result.report.resource_work_units > 0);
    assert!(result.report.topology_passes > 0);

    let limited = normalize_face_density(
        &source,
        Some(&selected),
        None,
        NormalizeDensityOptions {
            limits: MeshOperationLimits {
                max_work_units: 1,
                ..MeshOperationLimits::default()
            },
            ..NormalizeDensityOptions::default()
        },
    )
    .expect_err("work limit");
    assert!(matches!(limited, CoreError::OperationResourceLimit { .. }));
}

#[test]
fn density_normalization_can_cancel_before_scene_commit() {
    let source = make_box([4.0, 4.0, 4.0]).expect("box fixture");
    let selected = (0..source.face_count()).collect::<Vec<_>>();
    let mut calls = 0_usize;
    let mut cancel = || {
        calls = calls.saturating_add(1);
        if calls > 1 {
            Err(CoreError::OperationCancelled("density test".to_owned()))
        } else {
            Ok(())
        }
    };
    let error = normalize_face_density_with_hooks(
        &source,
        Some(&selected),
        None,
        NormalizeDensityOptions::default(),
        Some(&mut cancel),
        None,
    )
    .expect_err("cancellation");
    assert!(matches!(error, CoreError::OperationCancelled(_)));

    let mut scene = Scene::new();
    scene
        .add_object(SceneObject::new(source.clone()))
        .expect("add box");
    let id = scene.objects()[0].object_id().to_owned();
    let command = CommandRegistry::core()
        .dispatch(
            &mut scene,
            CommandRequest::NormalizeDensity {
                index: 0,
                face_indices: Some(selected),
                centers_mm: None,
                options: NormalizeDensityOptions {
                    target_edge_length_mm: Some(1.0),
                    max_iterations: 1,
                    ..NormalizeDensityOptions::default()
                },
            },
        )
        .expect("scene density command");
    assert!(command.changed);
    assert_eq!(scene.objects()[0].object_id(), id);
    scene.undo().expect("undo density");
    assert_eq!(scene.objects()[0].mesh(), &source);
    scene.redo().expect("redo density");
}

#[test]
fn density_selection_rejects_invalid_center_mesh_inputs() {
    let source = Mesh::new(vec![[0.0, 0.0, 0.0]; 3], vec![[0, 1, 2]]).expect("triangle");
    let error =
        select_face_region(&source, &[[f32::NAN, 0.0, 0.0]], 1.0).expect_err("non-finite center");
    assert!(matches!(error, CoreError::InvalidMesh(_)));
}
