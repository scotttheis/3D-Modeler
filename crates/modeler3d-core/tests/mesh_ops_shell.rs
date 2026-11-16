use modeler3d_core::{
    EmbossFont, HollowOptions, Mesh, MeshOperationLimits, Scene, SceneObject, TextEmbossOptions,
    Transform, boolean_union, hollow_mesh, inspect_mesh, make_box, make_cylinder, make_sphere,
    make_torus, text_emboss,
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

fn box_mesh(extents: [f32; 3]) -> Mesh {
    match make_box(extents) {
        Ok(mesh) => mesh,
        Err(error) => panic!("box failed: {error}"),
    }
}

#[test]
fn hollow_convex_box_uses_exact_offset_and_preserves_solid_contract() {
    let source = box_mesh([10.0, 10.0, 10.0]);
    let result = match hollow_mesh(
        &source,
        HollowOptions {
            wall_thickness_mm: 1.0,
            ..HollowOptions::default()
        },
    ) {
        Ok(result) => result,
        Err(error) => panic!("hollow failed: {error}"),
    };
    assert_eq!(result.report.branch, "convex-exact");
    assert!((result.report.result_volume_mm3 - 488.0).abs() < 0.01);
    assert!((volume(&result.mesh) - 488.0).abs() < 0.01);
    let health = match inspect_mesh(&result.mesh) {
        Ok(health) => health,
        Err(error) => panic!("health failed: {error}"),
    };
    assert!(health.watertight);
    assert!(health.winding_consistent);
    assert!(health.is_volume);
    assert_eq!(source.extents(), [10.0, 10.0, 10.0]);
}

#[test]
fn text_emboss_changes_topology_through_one_scene_history_entry() {
    let source = box_mesh([40.0, 40.0, 40.0]);
    let result = match text_emboss(
        &source,
        TextEmbossOptions {
            text: "A".to_owned(),
            font_size: 10,
            depth_mm: 1.0,
            raised: true,
            font: EmbossFont::Default,
            offset_x_mm: 0.0,
            offset_z_mm: 0.0,
            limits: MeshOperationLimits::default(),
        },
    ) {
        Ok(result) => result,
        Err(error) => panic!("emboss failed: {error}"),
    };
    assert!(result.report.occupied_cells > 0);
    assert!(result.mesh.extents()[1] > source.extents()[1]);
    let health = match inspect_mesh(&result.mesh) {
        Ok(health) => health,
        Err(error) => panic!("health failed: {error}"),
    };
    assert!(health.is_closed_manifold());

    let mut scene = Scene::new();
    if let Err(error) = scene.add_object(SceneObject::new(source)) {
        panic!("add failed: {error}");
    }
    let object_id = scene.objects()[0].object_id().to_owned();
    let before_revision = scene.revision();
    let command = match scene.apply_text_emboss(
        0,
        TextEmbossOptions {
            text: "A".to_owned(),
            ..TextEmbossOptions::default()
        },
    ) {
        Ok((command, _)) => command,
        Err(error) => panic!("scene emboss failed: {error}"),
    };
    assert!(command.changed);
    assert_eq!(scene.objects()[0].object_id(), object_id);
    assert_eq!(scene.revision(), before_revision + 1);
    if let Err(error) = scene.undo() {
        panic!("undo failed: {error}");
    }
    assert_eq!(scene.objects()[0].mesh().extents(), [40.0, 40.0, 40.0]);
}

#[cfg(windows)]
#[test]
fn named_system_fonts_render_when_installed() {
    let source = box_mesh([40.0, 40.0, 40.0]);
    for font in [EmbossFont::Sans, EmbossFont::Serif, EmbossFont::Mono] {
        let result = text_emboss(
            &source,
            TextEmbossOptions {
                text: "A".to_owned(),
                font,
                ..TextEmbossOptions::default()
            },
        );
        let result = match result {
            Ok(result) => result,
            Err(error) => panic!("named font emboss failed for {font:?}: {error}"),
        };
        assert!(result.report.occupied_cells > 0);
        assert!(result.mesh.extents()[1] > source.extents()[1]);
    }
}

#[test]
fn shell_operations_reject_invalid_parameters_before_scene_mutation() {
    let source = box_mesh([10.0, 10.0, 10.0]);
    let bad_hollow = hollow_mesh(
        &source,
        HollowOptions {
            wall_thickness_mm: 5.0,
            ..HollowOptions::default()
        },
    );
    assert!(bad_hollow.is_err());
    let bad_emboss = text_emboss(
        &source,
        TextEmbossOptions {
            text: "A".to_owned(),
            depth_mm: 0.0,
            ..TextEmbossOptions::default()
        },
    );
    assert!(bad_emboss.is_err());
    assert_eq!(source.extents(), [10.0, 10.0, 10.0]);
}

#[test]
fn hollow_concave_union_uses_bounded_voxel_offset() {
    let first = box_mesh([10.0, 10.0, 10.0]);
    let second = box_mesh([10.0, 10.0, 10.0]);
    let first_transform = match Transform::identity().translated([-3.0, 0.0, 0.0]) {
        Ok(transform) => transform,
        Err(error) => panic!("first transform failed: {error}"),
    };
    let second_transform = match Transform::identity().translated([0.0, 3.0, 0.0]) {
        Ok(transform) => transform,
        Err(error) => panic!("second transform failed: {error}"),
    };
    let first = match first.transformed(&first_transform) {
        Ok(mesh) => mesh,
        Err(error) => panic!("first transform failed: {error}"),
    };
    let second = match second.transformed(&second_transform) {
        Ok(mesh) => mesh,
        Err(error) => panic!("second transform failed: {error}"),
    };
    let source = match boolean_union(&first, &second) {
        Ok(mesh) => mesh,
        Err(error) => panic!("union failed: {error}"),
    };
    let result = match hollow_mesh(
        &source,
        HollowOptions {
            wall_thickness_mm: 1.0,
            ..HollowOptions::default()
        },
    ) {
        Ok(result) => result,
        Err(error) => panic!("concave hollow failed: {error}"),
    };
    assert_eq!(result.report.branch, "concave-voxel");
    let health = match inspect_mesh(&result.mesh) {
        Ok(health) => health,
        Err(error) => panic!("health failed: {error}"),
    };
    assert!(health.watertight);
    assert!(health.winding_consistent);
    assert!(health.is_volume);
    assert!(result.report.result_volume_mm3 > 0.0);
}

#[test]
fn shell_hooks_cancel_before_geometry_commit() {
    let source = box_mesh([40.0, 40.0, 40.0]);
    let mut cancel = || {
        Err(modeler3d_core::CoreError::OperationCancelled(
            "emboss_text".to_owned(),
        ))
    };
    let result = modeler3d_core::text_emboss_with_hooks(
        &source,
        TextEmbossOptions {
            text: "A".to_owned(),
            ..TextEmbossOptions::default()
        },
        Some(&mut cancel),
    );
    assert!(matches!(
        result,
        Err(modeler3d_core::CoreError::OperationCancelled(_))
    ));
}

#[test]
fn hollow_works_on_normal_concave_models() {
    // OF-54: these failed with "exceeded its resource limit" because every
    // grid cell was checked against every triangle.
    let torus = make_torus(20.0, 8.0, 64, 32).expect("torus");
    let result = hollow_mesh(
        &torus,
        HollowOptions {
            wall_thickness_mm: 1.5,
            ..HollowOptions::default()
        },
    )
    .expect("torus hollow");
    assert_eq!(result.report.branch, "concave-voxel");
    let health = inspect_mesh(&result.mesh).expect("health");
    assert!(health.watertight && health.winding_consistent && health.is_volume);
    // The inner cavity is not a "disconnected shell" needing repair.
    assert_eq!(health.cavity_shells, 1);
    assert!(health.valid, "{:?}", health.issues);
    // Shell of a torus: outer minus a torus 1.5 mm thinner, within 25 %.
    let pi2 = std::f64::consts::PI * std::f64::consts::PI;
    let expected = 2.0 * pi2 * 20.0 * (64.0 - 6.5 * 6.5);
    let shell = result.report.result_volume_mm3;
    assert!(
        (shell - expected).abs() < expected * 0.25,
        "{shell} vs {expected}"
    );

    // A sphere merged with a cylinder, like the owner's model.
    let sphere = make_sphere(20.0, 4).expect("sphere");
    let cylinder = make_cylinder(10.0, 60.0, 64)
        .expect("cylinder")
        .transformed(
            &Transform::identity()
                .translated([25.0, 0.0, 0.0])
                .expect("move"),
        )
        .expect("moved");
    let merged = boolean_union(&sphere, &cylinder).expect("union");
    // Two real pieces are still reported.
    let apart = boolean_union(
        &sphere,
        &sphere
            .transformed(
                &Transform::identity()
                    .translated([100.0, 0.0, 0.0])
                    .expect("move"),
            )
            .expect("moved"),
    )
    .expect("two spheres");
    let apart_health = inspect_mesh(&apart).expect("health");
    assert_eq!(apart_health.cavity_shells, 0);
    assert!(
        apart_health
            .issues
            .iter()
            .any(|issue| issue == "2 disconnected shells")
    );
    let result = hollow_mesh(
        &merged,
        HollowOptions {
            wall_thickness_mm: 2.0,
            ..HollowOptions::default()
        },
    )
    .expect("merged hollow");
    let health = inspect_mesh(&result.mesh).expect("health");
    assert!(health.watertight && health.is_volume);
    assert!(result.report.result_volume_mm3 < result.report.source_volume_mm3 * 0.5);
    assert!(
        result.mesh.face_count() < 200_000,
        "inner shell kept simple: {} faces",
        result.mesh.face_count()
    );
}
