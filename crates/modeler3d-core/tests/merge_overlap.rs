//! Merge must join overlapping objects into one solid and drop the faces
//! that end up inside the other object, whatever the primitive.

use modeler3d_core::{
    BooleanOperation, BooleanOptions, Mesh, PrimitiveKind, ProductBooleanOutcome, Scene,
    make_primitive,
};

fn signed_volume(mesh: &Mesh) -> f64 {
    mesh.faces
        .iter()
        .map(|face| {
            let [a, b, c] = face.map(|index| mesh.vertices[index as usize].map(f64::from));
            (a[0] * (b[1] * c[2] - b[2] * c[1])
                + a[1] * (b[2] * c[0] - b[0] * c[2])
                + a[2] * (b[0] * c[1] - b[1] * c[0]))
                / 6.0
        })
        .sum()
}

const ALL: [PrimitiveKind; 9] = [
    PrimitiveKind::Cube,
    PrimitiveKind::Cylinder,
    PrimitiveKind::Cone,
    PrimitiveKind::Sphere,
    PrimitiveKind::Pyramid,
    PrimitiveKind::Torus,
    PrimitiveKind::Hexagon,
    PrimitiveKind::Wedge,
    PrimitiveKind::Tetrahedron,
];

#[test]
fn every_primitive_faces_outwards() {
    for kind in ALL {
        let catalog = make_primitive(kind, None).expect("primitive");
        assert!(signed_volume(&catalog) > 0.0, "{kind:?} is inside out");
        let mut scene = Scene::new();
        scene.add_primitive(kind, None, None).expect("add");
        let added = scene.objects()[0].transformed_mesh().expect("mesh");
        assert!(signed_volume(&added) > 0.0, "added {kind:?} is inside out");
    }
}

/// A sphere with a torus sunk half way into its top.
fn sphere_with_sunk_torus() -> Scene {
    let mut scene = Scene::new();
    scene
        .add_primitive(PrimitiveKind::Sphere, None, None)
        .expect("sphere");
    scene
        .add_primitive(PrimitiveKind::Torus, None, None)
        .expect("torus");
    let sphere_top = scene.objects()[0]
        .transformed_mesh()
        .expect("sphere mesh")
        .bounds()
        .expect("bounds")
        .1[1];
    let (torus_min, torus_max) = scene.objects()[1]
        .transformed_mesh()
        .expect("torus mesh")
        .bounds()
        .expect("bounds");
    let half = (torus_max[1] - torus_min[1]) / 2.0;
    let lift = sphere_top - half - torus_min[1];
    let transform = scene.objects()[1]
        .translated_transform([0.0, lift, 0.0])
        .expect("transform");
    scene.set_transform(1, transform).expect("move");
    scene
}

fn assert_merged_into_one_solid(mut scene: Scene) {
    let input_faces: usize = scene
        .objects()
        .iter()
        .map(|object| object.mesh().faces.len())
        .sum();
    let outcome = scene
        .apply_product_boolean(0, 1, BooleanOperation::Union, BooleanOptions::default())
        .expect("merge");
    let ProductBooleanOutcome::Applied { report, .. } = outcome else {
        panic!("overlapping objects were grouped instead of merged");
    };
    assert_eq!(scene.objects().len(), 1);
    let merged = &scene.objects()[0];
    assert!(merged.children().is_empty(), "merge produced a group");
    assert_eq!(report.result_after.body_count, 1);
    assert!(report.result_after.is_volume);
    // The faces now inside the other object are gone.
    assert!(merged.mesh().faces.len() < input_faces);
    assert!(report.result_volume_mm3 < report.target_volume_mm3 + report.tool_volume_mm3);
    assert!(report.result_volume_mm3 > report.target_volume_mm3);
}

#[test]
fn torus_sunk_into_sphere_merges_into_one_solid() {
    assert_merged_into_one_solid(sphere_with_sunk_torus());
}

#[test]
fn inside_out_torus_from_an_older_project_still_merges() {
    let mut scene = sphere_with_sunk_torus();
    let mut inverted = scene.objects()[1].mesh().clone();
    for face in &mut inverted.faces {
        face.swap(1, 2);
    }
    scene.replace_mesh(1, inverted).expect("replace");
    assert_merged_into_one_solid(scene);
}
