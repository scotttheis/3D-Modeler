use std::fs;
use std::path::PathBuf;

use modeler3d_core::{
    CoreError, MeshFormat, Transform, WrlItem, WrlLimits, export_wrl, export_wrl_with_limits,
    load_mesh_asset_bytes, make_box, save_wrl,
};

const PYTHON_WRL: &[u8] = include_bytes!("fixtures/wrl/python_reference.wrl");

fn temporary_directory(label: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("modeler3d-r2d-{label}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).expect("temporary directory should be creatable");
    path
}

fn reference_items() -> Vec<WrlItem> {
    let first_transform = Transform::identity()
        .translated([7.0, 11.0, 13.0])
        .expect("translation should be valid")
        .rotated(90.0, [0.0, 0.0, 1.0])
        .expect("rotation should be valid")
        .scaled([2.0, 3.0, 4.0])
        .expect("scale should be valid");
    let first = WrlItem::new(
        make_box([2.0, 3.0, 4.0]).expect("box should be valid"),
        first_transform,
        "Red Gear".to_owned(),
        Some([0.2, 0.4, 0.6, 0.8]),
    )
    .expect("first WRL item should be valid");
    let second = WrlItem::new(
        make_box([1.0, 1.0, 1.0]).expect("box should be valid"),
        Transform::identity()
            .translated([-5.0, 2.0, 1.0])
            .expect("translation should be valid"),
        "Second Object".to_owned(),
        Some([0.9, 0.1, 0.0, 1.0]),
    )
    .expect("second WRL item should be valid");
    vec![first, second]
}

#[test]
fn python_wrl_fixture_matches_scene_export_byte_for_byte() {
    assert_eq!(
        export_wrl(&reference_items()).expect("WRL should export"),
        PYTHON_WRL
    );
}

#[test]
fn wrl_preserves_names_colors_transforms_and_indexed_geometry() {
    let bytes = export_wrl(&reference_items()).expect("WRL should export");
    let text = String::from_utf8(bytes).expect("WRL should be UTF-8");
    assert!(text.contains("DEF Red_Gear Transform {"));
    assert!(text.contains("translation 7 11 13"));
    assert!(text.contains("rotation 0 0 1 1.57079633"));
    assert!(text.contains("scale 2 3 4"));
    assert!(text.contains("diffuseColor 0.2 0.4 0.6"));
    assert!(text.contains("transparency 0.2"));
    assert!(text.contains("coordIndex ["));
    assert_eq!(text.matches("Shape {").count(), 2);
}

#[test]
fn wrl_sanitizes_duplicate_names_and_rejects_empty_scene() {
    let mesh = make_box([2.0, 2.0, 2.0]).expect("box should be valid");
    let first = WrlItem::new(
        mesh.clone(),
        Transform::identity(),
        "123 weird".to_owned(),
        None,
    )
    .expect("first item should be valid");
    let second = WrlItem::new(mesh, Transform::identity(), "123 weird".to_owned(), None)
        .expect("second item should be valid");
    let text = String::from_utf8(export_wrl(&[first, second]).expect("WRL should export"))
        .expect("WRL should be UTF-8");
    assert!(text.contains("DEF Object_123_weird Transform {"));
    assert!(text.contains("DEF Object_123_weird_2 Transform {"));
    assert!(matches!(
        MeshFormat::from_path("scene.WRL"),
        Ok(MeshFormat::Wrl)
    ));
    let error = load_mesh_asset_bytes(MeshFormat::Wrl, b"#VRML V2.0 utf8")
        .expect_err("WRL import should be unsupported");
    assert!(matches!(error, CoreError::Wrl(_)));
}

#[test]
fn wrl_rejects_shear_bad_colors_and_resource_limits() {
    let mesh = make_box([2.0, 2.0, 2.0]).expect("box should be valid");
    let mut shear = WrlItem::new(
        mesh.clone(),
        Transform::identity(),
        "Shear".to_owned(),
        None,
    )
    .expect("item should be valid");
    let mut rows = shear.transform.rows();
    rows[0][1] = 0.25;
    shear.transform = Transform::from_rows(rows).expect("shear matrix should be finite");
    let error = export_wrl(&[shear]).expect_err("shear should be rejected");
    assert!(matches!(error, CoreError::Wrl(_)));

    let mut bad_color = WrlItem::new(
        mesh.clone(),
        Transform::identity(),
        "Bad color".to_owned(),
        None,
    )
    .expect("item should be valid");
    bad_color.color = Some([1.1, 0.0, 0.0, 1.0]);
    let error = export_wrl(&[bad_color]).expect_err("bad color should be rejected");
    assert!(matches!(error, CoreError::Wrl(_)));

    let limited = WrlLimits {
        objects: 0,
        ..WrlLimits::default()
    };
    let error = export_wrl_with_limits(
        &[WrlItem::new(
            mesh.clone(),
            Transform::identity(),
            "Limited".to_owned(),
            None,
        )
        .expect("item should be valid")],
        &limited,
    )
    .expect_err("zero limits should be rejected");
    assert!(matches!(error, CoreError::Wrl(_)));

    let limited = WrlLimits {
        file_bytes: 1,
        ..WrlLimits::default()
    };
    let item = WrlItem::new(mesh, Transform::identity(), "Limited".to_owned(), None)
        .expect("item should be valid");
    let error = export_wrl_with_limits(&[item], &limited).expect_err("size limit should apply");
    assert!(matches!(error, CoreError::Wrl(_)));
}

#[test]
fn wrl_path_export_is_atomic_when_validation_fails() {
    let directory = temporary_directory("atomic");
    let path = directory.join("scene.wrl");
    let item = WrlItem::new(
        make_box([2.0, 2.0, 2.0]).expect("box should be valid"),
        Transform::identity(),
        "Stable".to_owned(),
        None,
    )
    .expect("item should be valid");
    save_wrl(std::slice::from_ref(&item), &path).expect("initial WRL should save");
    let original = fs::read(&path).expect("initial output should be readable");

    let mut invalid = item;
    invalid.mesh.vertices[0][0] = f32::NAN;
    let error = save_wrl(&[invalid], &path).expect_err("invalid output should fail");
    assert!(matches!(error, CoreError::Wrl(_)));
    assert_eq!(
        fs::read(&path).expect("existing output should remain"),
        original
    );
    let _ = fs::remove_dir_all(directory);
}
