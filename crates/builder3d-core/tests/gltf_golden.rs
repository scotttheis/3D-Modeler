use std::fs;
use std::path::{Path, PathBuf};

use builder3d_core::{
    CoreError, GltfItem, GltfLimits, MeshFormat, Transform, export_glb, export_gltf, load_gltf,
    load_gltf_bytes, load_gltf_bytes_from_directory, load_gltf_bytes_with_limits, make_box,
    save_gltf,
};

const PYTHON_GLTF: &str = include_str!("fixtures/gltf/python_reference.gltf");
const PYTHON_BUFFERS: [&str; 4] = [
    include_str!("fixtures/gltf/python_reference_buffer_0.b64"),
    include_str!("fixtures/gltf/python_reference_buffer_1.b64"),
    include_str!("fixtures/gltf/python_reference_buffer_2.b64"),
    include_str!("fixtures/gltf/python_reference_buffer_3.b64"),
];

fn temporary_directory(label: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("builder3d-r2c-{label}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).expect("temporary directory should be creatable");
    path
}

fn decode_base64(input: &str) -> Vec<u8> {
    let mut output = Vec::new();
    let mut accumulator = 0_u32;
    let mut bits = 0_u8;
    for byte in input.bytes() {
        let value = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' | b'\r' | b'\n' | b' ' | b'\t' => continue,
            other => panic!("invalid fixture base64 byte {other}"),
        };
        accumulator = (accumulator << 6) | u32::from(value);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            output.push(((accumulator >> bits) & 0xff) as u8);
            if bits == 0 {
                accumulator = 0;
            } else {
                accumulator &= (1_u32 << bits) - 1;
            }
        }
    }
    output
}

fn write_python_fixture(directory: &Path) -> PathBuf {
    let path = directory.join("python-reference.gltf");
    fs::write(&path, PYTHON_GLTF).expect("fixture JSON should be writable");
    for (index, encoded) in PYTHON_BUFFERS.iter().enumerate() {
        let path = directory.join(format!("reference_gltf_buffer_{index}.bin"));
        fs::write(path, decode_base64(encoded)).expect("fixture buffer should be writable");
    }
    path
}

fn assert_close(actual: [f32; 3], expected: [f32; 3]) {
    for (actual, expected) in actual.into_iter().zip(expected) {
        assert!((actual - expected).abs() < 1.0e-4, "{actual} != {expected}");
    }
}

#[test]
fn python_export_gltf_fixture_resolves_external_buffers_and_scene_transforms() {
    let directory = temporary_directory("python-golden");
    let path = write_python_fixture(&directory);
    let items = load_gltf(&path).expect("Python glTF fixture should load");
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].name, "First part");
    assert_eq!(items[1].name, "Second part");
    assert_close(items[0].mesh.extents(), [10.0, 20.0, 30.0]);
    assert_close(items[1].mesh.extents(), [2.0, 3.0, 4.0]);
    assert_close(items[0].transform.position(), [4.0, 5.0, 6.0]);
    assert_close(items[1].transform.position(), [-2.0, 1.0, 3.0]);
    assert_eq!(items[0].color, Some([0.2, 0.4, 0.8, 0.5019608]));
    assert_eq!(items[1].color, Some([1.0, 0.0, 0.0, 1.0]));
    let _ = fs::remove_dir_all(directory);
}

#[test]
fn export_gltf_and_glb_round_trip_materials_normals_and_transforms() {
    let first = GltfItem::new(
        make_box([10.0, 20.0, 30.0]).expect("box should be valid"),
        Transform::identity()
            .translated([4.0, 5.0, 6.0])
            .expect("translation should be valid"),
        "First part".to_owned(),
        Some([0.2, 0.4, 0.8, 0.5]),
    )
    .expect("item should be valid");
    let second = GltfItem::new(
        make_box([2.0, 3.0, 4.0]).expect("box should be valid"),
        Transform::identity()
            .translated([-2.0, 1.0, 3.0])
            .expect("translation should be valid"),
        "Second part".to_owned(),
        Some([1.0, 0.0, 0.0, 1.0]),
    )
    .expect("item should be valid");
    let source = [first, second];

    let gltf = export_gltf(&source).expect("embedded glTF should export");
    let document: serde_json::Value = serde_json::from_slice(&gltf).expect("JSON should parse");
    assert!(
        document["materials"]
            .as_array()
            .is_some_and(|items| items.len() == 2)
    );
    assert!(document["meshes"][0]["primitives"][0]["attributes"]["NORMAL"].is_number());
    assert!(
        document["buffers"][0]["uri"]
            .as_str()
            .is_some_and(|uri| uri.starts_with("data:application/octet-stream;base64,"))
    );
    let imported = load_gltf_bytes(&gltf).expect("embedded glTF should reload");
    assert_eq!(imported.len(), 2);
    assert_close(imported[0].mesh.extents(), [10.0, 20.0, 30.0]);
    assert_close(imported[0].transform.position(), [4.0, 5.0, 6.0]);
    assert_close(imported[1].transform.position(), [-2.0, 1.0, 3.0]);
    assert!(imported[0].color.is_some());

    let glb = export_glb(&source).expect("GLB should export");
    assert_eq!(&glb[..4], b"glTF");
    let declared = u32::from_le_bytes(glb[8..12].try_into().expect("GLB header length"));
    assert_eq!(usize::try_from(declared).expect("GLB length"), glb.len());
    let imported_glb = load_gltf_bytes(&glb).expect("GLB should reload");
    assert_eq!(imported_glb.len(), 2);
    assert_close(imported_glb[1].mesh.extents(), [2.0, 3.0, 4.0]);
    assert_close(imported_glb[1].transform.position(), [-2.0, 1.0, 3.0]);
}

#[test]
fn path_export_writes_safe_external_sidecar_and_replaces_existing_files() {
    let directory = temporary_directory("path-export");
    let path = directory.join("scene.gltf");
    let item = GltfItem::new(
        make_box([4.0, 5.0, 6.0]).expect("box should be valid"),
        Transform::identity(),
        "Repeated".to_owned(),
        None,
    )
    .expect("item should be valid");
    let report = save_gltf(std::slice::from_ref(&item), &path).expect("path export should work");
    assert_eq!(report.objects_exported, 1);
    assert_eq!(report.sidecars.len(), 1);
    assert!(Path::new(&report.sidecars[0]).is_file());
    let loaded = load_gltf(&path).expect("sidecar export should reload");
    assert_eq!(loaded[0].name, "Repeated");
    assert_close(loaded[0].mesh.extents(), [4.0, 5.0, 6.0]);
    let second_report = save_gltf(std::slice::from_ref(&item), &path)
        .expect("existing path should be replaced atomically");
    assert_eq!(second_report.sidecars, report.sidecars);
    let _ = fs::remove_dir_all(directory);
}

#[test]
fn embedded_external_boundary_and_resource_limits_are_typed_errors() {
    let directory = temporary_directory("limits");
    let outside = directory.parent().expect("temp directory has a parent");
    fs::write(outside.join("builder3d-r2c-outside.bin"), [0_u8; 4])
        .expect("outside marker should be writable");
    let traversal = br#"{"asset":{"version":"2.0"},"buffers":[{"uri":"../builder3d-r2c-outside.bin","byteLength":4}],"bufferViews":[],"accessors":[],"meshes":[],"nodes":[],"scenes":[{"nodes":[]}]}"#;
    let error = load_gltf_bytes_from_directory(traversal, &directory, &GltfLimits::default())
        .expect_err("parent traversal must be rejected");
    assert!(matches!(error, CoreError::Gltf(_)));

    let limits = GltfLimits {
        file_bytes: 4,
        ..GltfLimits::default()
    };
    let error =
        load_gltf_bytes_with_limits(br#"{}"#, &limits).expect_err("file limit must be enforced");
    assert!(matches!(error, CoreError::Gltf(_)));
    assert!(matches!(
        MeshFormat::from_path("scene.GLTF"),
        Ok(MeshFormat::Gltf)
    ));
    assert!(matches!(
        MeshFormat::from_path("scene.GLB"),
        Ok(MeshFormat::Glb)
    ));

    let _ = fs::remove_file(outside.join("builder3d-r2c-outside.bin"));
    let _ = fs::remove_dir_all(directory);
}

#[test]
fn malformed_glb_header_and_missing_embedded_resources_are_rejected() {
    let mut malformed = b"glTF\x02\0\0\0\x0c\0\0\0".to_vec();
    let error = load_gltf_bytes(&malformed).expect_err("GLB without JSON must fail");
    assert!(matches!(error, CoreError::Gltf(_)));
    malformed[8..12].copy_from_slice(&16_u32.to_le_bytes());
    let error = load_gltf_bytes(&malformed).expect_err("mismatched GLB length must fail");
    assert!(matches!(error, CoreError::Gltf(_)));

    let missing_external = br#"{"asset":{"version":"2.0"},"buffers":[{"uri":"missing.bin","byteLength":4}],"bufferViews":[],"accessors":[],"meshes":[],"nodes":[],"scenes":[{"nodes":[]}]}"#;
    let error = load_gltf_bytes(missing_external).expect_err("external bytes need a directory");
    assert!(matches!(error, CoreError::Gltf(_)));
}
