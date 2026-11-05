use builder3d_core::{
    BooleanFaceSource, BooleanOperation, BooleanOptions, CoreError, Mesh, MeshOperationLimits,
    Scene, SceneObject, Transform, boolean_color_provenance, boolean_operation, inspect_mesh,
    make_box,
};

fn mesh(result: Result<Mesh, CoreError>) -> Mesh {
    match result {
        Ok(mesh) => mesh,
        Err(error) => panic!("unexpected mesh error: {error}"),
    }
}

fn translated_box(offset: [f32; 3]) -> Mesh {
    let source = mesh(make_box([1.0, 1.0, 1.0]));
    let transform = match Transform::identity().translated(offset) {
        Ok(transform) => transform,
        Err(error) => panic!("unexpected transform error: {error}"),
    };
    match source.transformed(&transform) {
        Ok(mesh) => mesh,
        Err(error) => panic!("unexpected transformed mesh error: {error}"),
    }
}

#[test]
fn boolean_results_match_manifold_volume_and_keep_inputs_unchanged() {
    let target = mesh(make_box([1.0, 1.0, 1.0]));
    let tool = translated_box([0.5, 0.0, 0.0]);
    let target_before = target.clone();
    let tool_before = tool.clone();

    let union = match boolean_operation(
        &target,
        &tool,
        BooleanOperation::Union,
        BooleanOptions::default(),
    ) {
        Ok(result) => result,
        Err(error) => panic!("union failed: {error}"),
    };
    let difference = match boolean_operation(
        &target,
        &tool,
        BooleanOperation::Difference,
        BooleanOptions::default(),
    ) {
        Ok(result) => result,
        Err(error) => panic!("difference failed: {error}"),
    };
    let intersection = match boolean_operation(
        &target,
        &tool,
        BooleanOperation::Intersection,
        BooleanOptions::default(),
    ) {
        Ok(result) => result,
        Err(error) => panic!("intersection failed: {error}"),
    };

    assert!((union.report.result_volume_mm3.abs() - 1.5).abs() < 0.01);
    assert!((difference.report.result_volume_mm3.abs() - 0.5).abs() < 0.01);
    assert!((intersection.report.result_volume_mm3.abs() - 0.5).abs() < 0.01);
    assert!(union.report.result_after.is_closed_manifold());
    assert!(difference.report.result_after.is_closed_manifold());
    assert!(intersection.report.result_after.is_closed_manifold());
    assert_eq!(target, target_before);
    assert_eq!(tool, tool_before);
    assert_eq!(union.face_sources.len(), union.mesh.face_count());
    assert!(union.face_sources.contains(&BooleanFaceSource::Target));
    assert!(union.face_sources.contains(&BooleanFaceSource::Tool));
}

#[test]
fn boolean_rejects_open_operands_and_reports_health() {
    let open = match Mesh::new(
        vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        vec![[0, 1, 2]],
    ) {
        Ok(mesh) => mesh,
        Err(error) => panic!("open mesh construction failed: {error}"),
    };
    let box_mesh = mesh(make_box([1.0, 1.0, 1.0]));
    let health = match inspect_mesh(&open) {
        Ok(report) => report,
        Err(error) => panic!("health inspection failed: {error}"),
    };
    assert!(!health.valid);
    assert!(!health.watertight);
    let error = boolean_operation(
        &open,
        &box_mesh,
        BooleanOperation::Union,
        BooleanOptions::default(),
    );
    assert!(matches!(error, Err(CoreError::MeshOperation(_))));
}

#[test]
fn boolean_resource_limit_fails_before_kernel_and_preserves_inputs() {
    let target = mesh(make_box([1.0, 1.0, 1.0]));
    let tool = translated_box([0.5, 0.0, 0.0]);
    let limits = MeshOperationLimits {
        max_boolean_input_faces: 1,
        ..MeshOperationLimits::default()
    };
    let error = boolean_operation(
        &target,
        &tool,
        BooleanOperation::Union,
        BooleanOptions {
            regularize_cut_caps: true,
            limits,
        },
    );
    assert!(matches!(
        error,
        Err(CoreError::OperationResourceLimit { .. })
    ));
}

#[test]
fn scene_boolean_preserves_target_identity_transform_and_history_atomically() {
    let target_mesh = mesh(make_box([1.0, 1.0, 1.0]));
    let tool_mesh = mesh(make_box([1.0, 1.0, 1.0]));
    let target = match SceneObject::with_details(
        target_mesh,
        Some("target-id".to_owned()),
        Some("Target".to_owned()),
        Some([0.2, 0.4, 0.6, 1.0]),
        Some(serde_json::json!({"role": "target"})),
    ) {
        Ok(object) => object,
        Err(error) => panic!("target construction failed: {error}"),
    };
    let tool = match SceneObject::with_details(
        tool_mesh,
        Some("tool-id".to_owned()),
        Some("Tool".to_owned()),
        Some([0.8, 0.2, 0.1, 1.0]),
        Some(serde_json::json!({"role": "tool"})),
    ) {
        Ok(object) => object,
        Err(error) => panic!("tool construction failed: {error}"),
    };
    let mut scene = Scene::new();
    if let Err(error) = scene.add_object(tool) {
        panic!("tool add failed: {error}");
    }
    if let Err(error) = scene.add_object(target) {
        panic!("target add failed: {error}");
    }
    let target_transform = match Transform::identity()
        .translated([5.0, 2.0, -1.0])
        .and_then(|transform| transform.rotated(30.0, [0.0, 0.0, 1.0]))
        .and_then(|transform| transform.scaled([1.5, 0.75, 2.0]))
    {
        Ok(transform) => transform,
        Err(error) => panic!("target transform failed: {error}"),
    };
    let tool_transform = match Transform::identity().translated([5.75, 2.0, -1.0]) {
        Ok(transform) => transform,
        Err(error) => panic!("tool transform failed: {error}"),
    };
    if let Err(error) = scene.set_transform(1, target_transform) {
        panic!("target transform command failed: {error}");
    }
    if let Err(error) = scene.set_transform(0, tool_transform) {
        panic!("tool transform command failed: {error}");
    }
    scene.set_selection([0, 1]);
    let before_revision = scene.revision();
    let applied = scene.apply_boolean(1, 0, BooleanOperation::Union, BooleanOptions::default());
    let (new_index, command, report) = match applied {
        Ok(result) => result,
        Err(error) => panic!("scene boolean failed: {error}"),
    };
    assert_eq!(new_index, 0);
    assert!(command.changed);
    assert_eq!(scene.objects().len(), 1);
    let result = &scene.objects()[0];
    assert_eq!(result.object_id(), "target-id");
    assert_eq!(result.name(), "Target");
    assert_eq!(result.color(), [0.2, 0.4, 0.6, 1.0]);
    assert_eq!(result.metadata()["role"], "target");
    let provenance = boolean_color_provenance(result.metadata(), result.mesh().face_count());
    let Some(provenance) = provenance else {
        panic!("boolean source face colors were not retained");
    };
    assert_eq!(provenance.operation, "merge");
    assert_eq!(
        provenance.source_colors,
        [[0.2, 0.4, 0.6, 1.0], [0.8, 0.2, 0.1, 1.0]]
    );
    assert!(provenance.tool_face_count > 0);
    assert_eq!(provenance.face_sources.len(), result.mesh().face_count());
    assert_eq!(result.transform(), &target_transform);
    assert_eq!(
        scene.selection().iter().copied().collect::<Vec<_>>(),
        vec![0]
    );
    assert!((report.target_volume_mm3.abs() - 2.25).abs() < 0.02);
    assert!(report.result_volume_mm3.abs() > report.target_volume_mm3.abs());
    assert_eq!(scene.revision(), before_revision + 1);

    let undone = match scene.undo() {
        Ok(Some(result)) => result,
        Ok(None) => panic!("boolean undo produced no history result"),
        Err(error) => panic!("boolean undo failed: {error}"),
    };
    assert!(undone.changed);
    assert_eq!(scene.objects().len(), 2);
    assert_eq!(scene.objects()[0].object_id(), "tool-id");
    assert_eq!(scene.objects()[1].object_id(), "target-id");
    assert_eq!(
        scene.objects()[1].metadata(),
        &serde_json::json!({"role": "target"})
    );
    assert_eq!(
        scene.selection().iter().copied().collect::<Vec<_>>(),
        vec![0, 1]
    );
    let redone = match scene.redo() {
        Ok(Some(result)) => result,
        Ok(None) => panic!("boolean redo produced no history result"),
        Err(error) => panic!("boolean redo failed: {error}"),
    };
    assert!(redone.changed);
    assert_eq!(scene.objects().len(), 1);
    assert_eq!(scene.objects()[0].object_id(), "target-id");
    assert_eq!(
        boolean_color_provenance(
            scene.objects()[0].metadata(),
            scene.objects()[0].mesh().face_count()
        ),
        Some(provenance)
    );
}

#[test]
fn boolean_face_colors_follow_paint_and_topology_history() {
    let target = SceneObject::with_details(
        mesh(make_box([2.0, 2.0, 2.0])),
        None,
        Some(String::from("Target")),
        Some([0.2, 0.4, 0.6, 1.0]),
        None,
    );
    let tool = SceneObject::with_details(
        mesh(make_box([1.0, 1.0, 1.0])),
        None,
        Some(String::from("Tool")),
        Some([0.8, 0.2, 0.1, 1.0]),
        None,
    );
    let (target, tool) = match (target, tool) {
        (Ok(target), Ok(tool)) => (target, tool),
        (Err(error), _) | (_, Err(error)) => panic!("boolean object construction failed: {error}"),
    };
    let mut scene = Scene::new();
    assert!(scene.add_object(target).is_ok());
    assert!(scene.add_object(tool).is_ok());
    let tool_transform = match Transform::identity().translated([0.5, 0.0, 0.0]) {
        Ok(transform) => transform,
        Err(error) => panic!("tool transform failed: {error}"),
    };
    assert!(scene.set_transform(1, tool_transform).is_ok());
    assert!(
        scene
            .apply_boolean(
                0,
                1,
                BooleanOperation::Difference,
                BooleanOptions::default()
            )
            .is_ok()
    );
    let original = boolean_color_provenance(
        scene.objects()[0].metadata(),
        scene.objects()[0].mesh().face_count(),
    );
    let Some(original) = original else {
        panic!("difference source face colors were not retained");
    };
    assert!(original.tool_face_count > 0);

    let painted = [0.1, 0.7, 0.3, 1.0];
    assert!(scene.set_color(0, painted).is_ok());
    let updated = boolean_color_provenance(
        scene.objects()[0].metadata(),
        scene.objects()[0].mesh().face_count(),
    );
    let Some(updated) = updated else {
        panic!("painting removed the boolean source palette");
    };
    assert_eq!(updated.source_colors[0], painted);
    assert_eq!(updated.source_colors[1], original.source_colors[1]);
    assert_eq!(updated.face_sources, original.face_sources);
    assert!(scene.undo().is_ok_and(|result| result.is_some()));
    assert_eq!(
        boolean_color_provenance(
            scene.objects()[0].metadata(),
            scene.objects()[0].mesh().face_count()
        ),
        Some(original.clone())
    );

    let replacement = mesh(make_box([1.8, 1.8, 1.8]));
    assert!(scene.replace_mesh(0, replacement).is_ok());
    assert!(
        scene.objects()[0]
            .metadata()
            .get("boolean_color_provenance")
            .is_none()
    );
    assert!(scene.undo().is_ok_and(|result| result.is_some()));
    assert_eq!(
        boolean_color_provenance(
            scene.objects()[0].metadata(),
            scene.objects()[0].mesh().face_count()
        ),
        Some(original)
    );
}

#[test]
fn difference_can_regularize_a_flat_tool_cap_without_changing_volume() {
    let target = mesh(make_box([40.0, 40.0, 40.0]));
    let cutter = match mesh(make_box([20.0, 60.0, 20.0])).transformed(&match Transform::identity()
        .translated([12.0, 0.0, 0.0])
    {
        Ok(transform) => transform,
        Err(error) => panic!("cutter transform failed: {error}"),
    }) {
        Ok(mesh) => mesh,
        Err(error) => panic!("cutter transformation failed: {error}"),
    };
    let plain = match boolean_operation(
        &target,
        &cutter,
        BooleanOperation::Difference,
        BooleanOptions {
            regularize_cut_caps: false,
            limits: MeshOperationLimits::default(),
        },
    ) {
        Ok(result) => result,
        Err(error) => panic!("plain difference failed: {error}"),
    };
    let regularized = match boolean_operation(
        &target,
        &cutter,
        BooleanOperation::Difference,
        BooleanOptions::default(),
    ) {
        Ok(result) => result,
        Err(error) => panic!("regularized difference failed: {error}"),
    };
    let plain_cap_faces = plain
        .mesh
        .faces
        .iter()
        .filter(|face| {
            face.iter().all(|index| {
                plain
                    .mesh
                    .vertices
                    .get(*index as usize)
                    .is_some_and(|vertex| (vertex[0] - 2.0).abs() < 1.0e-4)
            })
        })
        .count();
    let regularized_cap_faces = regularized
        .mesh
        .faces
        .iter()
        .filter(|face| {
            face.iter().all(|index| {
                regularized
                    .mesh
                    .vertices
                    .get(*index as usize)
                    .is_some_and(|vertex| (vertex[0] - 2.0).abs() < 1.0e-4)
            })
        })
        .count();
    assert!(regularized.report.regularized_cap_patches > 0);
    assert!(regularized_cap_faces > plain_cap_faces);
    assert!(
        (regularized.report.result_volume_mm3.abs() - plain.report.result_volume_mm3.abs())
            < 1.0e-4
    );
    assert!(regularized.report.result_after.is_closed_manifold());
}
