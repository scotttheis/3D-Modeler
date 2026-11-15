use builder3d_core::{
    BlobOptions, BrushDirection, BrushFalloff, BrushSymmetry, CreasePolishOptions,
    ElasticGrabOptions, Mesh, MeshOperationLimits, Scene, SceneObject, SmoothBrushOptions, blob,
    crease_polish, elastic_grab, make_cylinder, make_sphere, smooth_brush,
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
fn elastic_grab_preserves_topology_and_has_smooth_bounded_displacement() {
    let source = make_sphere(5.0, 2).expect("sphere fixture");
    let original = source.clone();
    let result = elastic_grab(
        &source,
        [5.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        ElasticGrabOptions {
            radius_mm: 2.5,
            strength: 1.0,
            ..ElasticGrabOptions::default()
        },
    )
    .expect("elastic grab");
    assert_eq!(result.mesh.faces, source.faces);
    assert_ne!(result.mesh.vertices, source.vertices);
    let maximum = source
        .vertices
        .iter()
        .zip(&result.mesh.vertices)
        .map(|(first, second)| {
            let delta = [
                second[0] - first[0],
                second[1] - first[1],
                second[2] - first[2],
            ];
            (delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2]).sqrt()
        })
        .fold(0.0_f32, f32::max);
    assert!((maximum - 1.0).abs() < 0.08);
    assert_eq!(source, original);
}

#[test]
fn elastic_grab_symmetry_mirrors_center_and_offset() {
    let source = Mesh::new(
        vec![[1.0, 0.0, 0.0], [-1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        vec![[0, 1, 2]],
    )
    .expect("triangle fixture");
    let result = elastic_grab(
        &source,
        [1.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        ElasticGrabOptions {
            radius_mm: 0.5,
            strength: 1.0,
            symmetry: BrushSymmetry::X,
            ..ElasticGrabOptions::default()
        },
    )
    .expect("symmetric grab");
    assert!((result.mesh.vertices[0][0] - 2.0).abs() < 1.0e-5);
    assert!((result.mesh.vertices[1][0] + 2.0).abs() < 1.0e-5);
}

#[test]
fn crease_polish_preserves_topology_and_supports_both_directions() {
    let source = make_sphere(5.0, 2).expect("sphere fixture");
    let inward = crease_polish(
        &source,
        &[[5.0, 0.0, 0.0], [4.5, 0.5, 0.0]],
        CreasePolishOptions {
            radius_mm: 3.0,
            strength: 1.0,
            direction: BrushDirection::Inward,
            falloff: BrushFalloff::Smooth,
            ..CreasePolishOptions::default()
        },
    )
    .expect("inward crease");
    let outward = crease_polish(
        &source,
        &[[5.0, 0.0, 0.0]],
        CreasePolishOptions {
            radius_mm: 3.0,
            strength: 1.0,
            direction: BrushDirection::Outward,
            falloff: BrushFalloff::Linear,
            ..CreasePolishOptions::default()
        },
    )
    .expect("outward crease");
    assert_eq!(inward.mesh.faces, source.faces);
    assert_ne!(inward.mesh.vertices, source.vertices);
    assert_ne!(outward.mesh.vertices, source.vertices);
}

#[test]
fn smooth_brush_and_blob_preserve_topology_and_blob_changes_volume_direction() {
    let source = make_sphere(10.0, 2).expect("sphere fixture");
    let smooth = smooth_brush(
        &source,
        &[[0.0, 0.0, 10.0], [0.0, 1.0, 9.5]],
        SmoothBrushOptions::default(),
    )
    .expect("smooth brush");
    assert_eq!(smooth.mesh.faces, source.faces);
    assert_ne!(smooth.mesh.vertices, source.vertices);
    assert!(smooth.report.maximum_displacement_mm <= 16.0 + 1.0e-5);
    let outward = blob(
        &source,
        &[[0.0, 0.0, 10.0], [0.0, 1.0, 9.5]],
        BlobOptions::default(),
    )
    .expect("outward blob");
    let inward = blob(
        &source,
        &[[0.0, 0.0, 10.0]],
        BlobOptions {
            direction: BrushDirection::Inward,
            falloff: BrushFalloff::Linear,
            ..BlobOptions::default()
        },
    )
    .expect("inward blob");
    assert_eq!(outward.mesh.faces, source.faces);
    assert!(volume(&outward.mesh) > volume(&source));
    assert!(volume(&inward.mesh) < volume(&source));
    assert!(outward.report.maximum_displacement_mm <= 16.0 + 1.0e-5);
}

#[test]
fn brushes_reject_bad_paths_offsets_and_limits_before_scene_mutation() {
    let source = make_sphere(2.0, 1).expect("sphere fixture");
    let too_many = vec![[0.0, 0.0, 0.0]; 257];
    let error =
        crease_polish(&source, &too_many, CreasePolishOptions::default()).expect_err("path bound");
    assert!(matches!(error, builder3d_core::CoreError::MeshOperation(_)));
    let error = elastic_grab(
        &source,
        [0.0, 0.0, 0.0],
        [2_001.0, 0.0, 0.0],
        ElasticGrabOptions::default(),
    )
    .expect_err("offset bound");
    assert!(matches!(error, builder3d_core::CoreError::MeshOperation(_)));
    let limits = MeshOperationLimits {
        max_work_units: 1,
        ..MeshOperationLimits::default()
    };
    let error = blob(
        &source,
        &[[0.0, 0.0, 0.0]],
        BlobOptions {
            limits,
            ..BlobOptions::default()
        },
    )
    .expect_err("work bound");
    assert!(matches!(
        error,
        builder3d_core::CoreError::OperationResourceLimit { .. }
    ));
}

#[test]
fn scene_brush_replacement_preserves_identity_and_undo() {
    let source = make_sphere(3.0, 1).expect("sphere fixture");
    let mut scene = Scene::new();
    scene
        .add_object(SceneObject::new(source.clone()))
        .expect("add sphere");
    let id = scene.objects()[0].object_id().to_owned();
    let (command, report) = scene
        .apply_blob(0, &[[0.0, 0.0, 3.0]], BlobOptions::default())
        .expect("scene blob");
    assert!(command.changed);
    assert!(report.changed);
    assert_eq!(scene.objects()[0].object_id(), id);
    scene.undo().expect("undo blob");
    assert_eq!(scene.objects()[0].mesh(), &source);
    scene.redo().expect("redo blob");
}

#[test]
fn crease_is_gentle_bounded_and_never_folds_the_surface() {
    // OF-49: a stroke across the top of a dense sphere. Like Blender's
    // Crease it sinks gently, pinches the surface toward the line, and a long
    // back-and-forth stroke digs no deeper than one pass.
    let source = make_sphere(10.0, 4).expect("sphere fixture");
    let path: Vec<[f32; 3]> = (0..=10)
        .map(|step| {
            let x = -5.0 + step as f32;
            [x, 0.0, (100.0 - x * x).sqrt()]
        })
        .collect();
    let mut long_path = path.clone();
    for _ in 0..10 {
        long_path.extend(path.iter().rev().copied());
        long_path.extend(path.iter().copied());
    }
    let options = CreasePolishOptions {
        radius_mm: 4.0,
        strength: 1.0,
        ..CreasePolishOptions::default()
    };
    let once = crease_polish(&source, &path, options).expect("crease");
    let many = crease_polish(&source, &long_path, options).expect("long crease");
    assert_eq!(once.mesh.faces, source.faces);
    for (first, second) in once.mesh.vertices.iter().zip(&many.mesh.vertices) {
        for axis in 0..3 {
            assert!(
                (first[axis] - second[axis]).abs() < 1.0e-3,
                "repeat passes do not dig deeper: {first:?} vs {second:?}"
            );
        }
    }
    assert!(once.report.changed);
    // Gentle: never more than a quarter of the radius deep plus the half-way pinch.
    assert!(
        once.report.maximum_displacement_mm <= 4.0 * 0.5 + 0.5,
        "{}",
        once.report.maximum_displacement_mm
    );
    let mut sank = false;
    for (before, after) in source.vertices.iter().zip(&once.mesh.vertices) {
        // Nothing crosses the stroke line (the y = 0 plane here).
        if before[1].abs() > 1.0e-4 {
            assert!(before[1] * after[1] > 0.0, "{before:?} -> {after:?}");
            assert!(
                after[1].abs() <= before[1].abs() + 1.0e-4,
                "pinch is toward the line"
            );
        }
        let radial_before =
            (before[0] * before[0] + before[1] * before[1] + before[2] * before[2]).sqrt();
        let radial_after = (after[0] * after[0] + after[1] * after[1] + after[2] * after[2]).sqrt();
        sank |= radial_after < radial_before - 0.2;
    }
    assert!(sank, "the crease sinks into the surface");
    // No face turns over.
    for face in &source.faces {
        let before = source.face_normal(*face);
        let after = once.mesh.face_normal(*face);
        let dot = before[0] * after[0] + before[1] * after[1] + before[2] * after[2];
        assert!(dot > 0.0, "face {face:?} folded");
    }
}

fn extent(mesh: &Mesh, axis: usize) -> f32 {
    let values = mesh.vertices.iter().map(|vertex| vertex[axis]);
    values.clone().fold(f32::MIN, f32::max) - values.fold(f32::MAX, f32::min)
}

#[test]
fn smooth_brush_keeps_a_sparse_stock_cylinder_from_melting() {
    // OF-50 / D9: the two-ring stock cylinder used to melt. Sharp rims now
    // only slide along themselves, so a strong stroke over the whole
    // cylinder keeps its height, width and volume.
    let source = make_cylinder(20.0, 40.0, 64).expect("cylinder fixture");
    let centers: Vec<[f32; 3]> = (0..8)
        .map(|step| {
            let angle = step as f32 * std::f32::consts::FRAC_PI_4;
            [
                20.0 * angle.cos(),
                20.0 * angle.sin(),
                40.0 * (step % 2) as f32,
            ]
        })
        .collect();
    let result = smooth_brush(
        &source,
        &centers,
        SmoothBrushOptions {
            radius_mm: 40.0,
            strength: 1.0,
            ..SmoothBrushOptions::default()
        },
    )
    .expect("smooth brush");
    for axis in 0..3 {
        let before = extent(&source, axis);
        let after = extent(&result.mesh, axis);
        assert!(after >= before * 0.97, "axis {axis}: {before} -> {after}");
    }
    assert!(volume(&result.mesh) >= volume(&source) * 0.95);
}

#[test]
fn smooth_brush_still_evens_out_a_bumpy_surface() {
    let mut source = make_sphere(10.0, 4).expect("sphere fixture");
    for (index, vertex) in source.vertices.iter_mut().enumerate() {
        let bump = if index % 3 == 0 { 1.02 } else { 0.99 };
        for value in vertex.iter_mut() {
            *value *= bump;
        }
    }
    let roughness = |mesh: &Mesh| -> f32 {
        mesh.vertices
            .iter()
            .filter(|vertex| vertex[2] > 9.5)
            .map(|vertex| {
                ((vertex[0] * vertex[0] + vertex[1] * vertex[1] + vertex[2] * vertex[2]).sqrt()
                    - 10.0)
                    .abs()
            })
            .sum()
    };
    let result = smooth_brush(
        &source,
        &[[0.0, 0.0, 10.0]],
        SmoothBrushOptions {
            radius_mm: 6.0,
            strength: 1.0,
            shrinkage: 1.0,
            ..SmoothBrushOptions::default()
        },
    )
    .expect("smooth brush");
    assert!(
        roughness(&result.mesh) < roughness(&source) * 0.8,
        "{} -> {}",
        roughness(&source),
        roughness(&result.mesh)
    );
}
