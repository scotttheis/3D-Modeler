use builder3d_core::{
    CoreError, Mesh, MeshAsset, MeshFormat, export_mesh, export_mesh_asset, load_mesh_asset_bytes,
    make_box,
};

const BOX_STL_ASCII: &[u8] = include_bytes!("fixtures/formats/python_box.stl");
const BOX_STL_BINARY_B64: &str = include_str!("fixtures/formats/python_box_binary.stl.b64");
const BOX_PLY_BINARY_B64: &str = include_str!("fixtures/formats/python_box_binary.ply.b64");
const BOX_OBJ: &[u8] = include_bytes!("fixtures/formats/python_box.obj");
const BOX_PLY_ASCII: &[u8] = include_bytes!("fixtures/formats/python_box.ply");
const COLORED_OBJ: &[u8] = include_bytes!("fixtures/formats/colored_triangle.obj");
const COLORED_PLY: &[u8] = include_bytes!("fixtures/formats/colored_triangle.ply");

fn must<T>(result: Result<T, CoreError>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("unexpected file-I/O error: {error}"),
    }
}

fn box_mesh() -> Mesh {
    must(make_box([2.0, 3.0, 4.0]))
}

fn triangle_positions(mesh: &Mesh) -> Vec<[[f32; 3]; 3]> {
    mesh.faces
        .iter()
        .map(|face| {
            [
                mesh.vertices[face[0] as usize],
                mesh.vertices[face[1] as usize],
                mesh.vertices[face[2] as usize],
            ]
        })
        .collect()
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
            b'=' | b'\r' | b'\n' | b' ' | b'\t' => {
                if byte == b'=' {
                    break;
                }
                continue;
            }
            _ => panic!("invalid base64 fixture byte {byte}"),
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

#[test]
fn python_stl_ascii_golden_loads_with_reference_triangle_order() {
    let asset = must(load_mesh_asset_bytes(MeshFormat::Stl, BOX_STL_ASCII));
    assert_eq!(
        triangle_positions(asset.mesh()),
        triangle_positions(&box_mesh())
    );
    assert_eq!(asset.vertex_colors(), None);
}

#[test]
fn python_stl_binary_export_is_byte_compatible() {
    let expected = decode_base64(BOX_STL_BINARY_B64);
    let actual = must(export_mesh(&box_mesh(), MeshFormat::Stl));
    assert_eq!(actual, expected);
}

#[test]
fn python_obj_golden_round_trips_byte_for_byte() {
    let asset = must(load_mesh_asset_bytes(MeshFormat::Obj, BOX_OBJ));
    assert_eq!(asset.mesh(), &box_mesh());
    let actual = must(export_mesh_asset(&asset, MeshFormat::Obj));
    assert_eq!(actual, BOX_OBJ);
}

#[test]
fn python_ply_ascii_golden_loads_and_binary_output_reopens() {
    let asset = must(load_mesh_asset_bytes(MeshFormat::Ply, BOX_PLY_ASCII));
    assert_eq!(asset.mesh(), &box_mesh());
    let bytes = must(export_mesh_asset(&asset, MeshFormat::Ply));
    assert_eq!(bytes, decode_base64(BOX_PLY_BINARY_B64));
    assert_eq!(
        &bytes[..b"ply\nformat binary_little_endian 1.0\n".len()],
        b"ply\nformat binary_little_endian 1.0\n"
    );
    let reopened = must(load_mesh_asset_bytes(MeshFormat::Ply, &bytes));
    assert_eq!(reopened.mesh(), &box_mesh());
    assert_eq!(bytes.len(), 464);
}

#[test]
fn python_colored_obj_and_ply_fixtures_preserve_display_colors() {
    let obj = must(load_mesh_asset_bytes(MeshFormat::Obj, COLORED_OBJ));
    assert_eq!(obj.mesh().extents(), [2.0, 3.0, 4.0]);
    assert_eq!(obj.vertex_colors(), Some(&[[51, 102, 153, 255]; 3][..]));
    let obj_color = obj.display_color().expect("OBJ fixture has a color");
    assert!((obj_color[0] - 0.2).abs() < 1.0 / 255.0);
    assert!((obj_color[1] - 0.4).abs() < 1.0 / 255.0);
    assert!((obj_color[2] - 0.6).abs() < 1.0 / 255.0);
    assert!((obj_color[3] - 1.0).abs() < 1.0 / 255.0);

    let ply = must(load_mesh_asset_bytes(MeshFormat::Ply, COLORED_PLY));
    assert_eq!(ply.mesh().extents(), [2.0, 3.0, 4.0]);
    assert_eq!(ply.vertex_colors(), Some(&[[51, 102, 153, 204]; 3][..]));
    let ply_color = ply.display_color().expect("PLY fixture has a color");
    assert!((ply_color[3] - 0.8).abs() < 1.0 / 255.0);
}

#[test]
fn format_selection_is_case_insensitive_and_rejects_unknown_extensions() {
    assert_eq!(must(MeshFormat::from_path("fixture.STL")), MeshFormat::Stl);
    assert_eq!(must(MeshFormat::from_path("fixture.obj")), MeshFormat::Obj);
    assert!(matches!(
        MeshFormat::from_path("fixture.xyz"),
        Err(CoreError::UnsupportedMeshFormat(_))
    ));
}

#[test]
fn mesh_asset_rejects_mismatched_vertex_colors() {
    let error = MeshAsset::new(box_mesh(), Some(vec![[0, 0, 0, 255]])).expect_err("must reject");
    assert!(matches!(error, CoreError::MeshFile(_)));
}
