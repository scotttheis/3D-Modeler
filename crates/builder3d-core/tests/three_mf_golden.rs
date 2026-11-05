use std::io::{Cursor, Read, Write};

use builder3d_core::{
    CoreError, MeshFormat, ThreeMfItem, ThreeMfLimits, ThreeMfUnit, Transform, export_3mf,
    load_3mf_bytes, load_3mf_bytes_with_limits, load_3mf_thumbnail_bytes, make_box,
};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

const ASSEMBLY_MODEL: &[u8] = include_bytes!("fixtures/3mf/assembly.model");
const EXTERNAL_ROOT_MODEL: &[u8] = include_bytes!("fixtures/3mf/external-root.model");
const EXTERNAL_PART_MODEL: &[u8] = include_bytes!("fixtures/3mf/external-part.model");

const ROOT_RELATIONSHIPS: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rModel" Type="http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel" Target="/3D/3dmodel.model" />
</Relationships>"#;

const MODEL_RELATIONSHIPS: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rThumbnail" Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/thumbnail" Target="../Metadata/thumbnail.png" />
</Relationships>"#;

const EXTERNAL_MODEL_RELATIONSHIPS: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rExternal" Type="http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel" Target="/3D/Objects/object_1.model" />
</Relationships>"#;

const CONTENT_TYPES: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml" />
  <Default Extension="model" ContentType="application/vnd.ms-package.3dmanufacturing-3dmodel+xml" />
  <Default Extension="png" ContentType="image/png" />
</Types>"#;

fn make_package(
    model: &[u8],
    model_relationships: Option<&[u8]>,
    external_part: Option<&[u8]>,
    thumbnail: Option<&[u8]>,
) -> Vec<u8> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    write_entry(&mut writer, options, "[Content_Types].xml", CONTENT_TYPES);
    write_entry(&mut writer, options, "_rels/.rels", ROOT_RELATIONSHIPS);
    write_entry(&mut writer, options, "3D/3dmodel.model", model);
    if let Some(relationships) = model_relationships {
        write_entry(
            &mut writer,
            options,
            "3D/_rels/3dmodel.model.rels",
            relationships,
        );
    }
    if let Some(external) = external_part {
        write_entry(&mut writer, options, "3D/Objects/object_1.model", external);
    }
    if let Some(thumbnail) = thumbnail {
        write_entry(&mut writer, options, "Metadata/thumbnail.png", thumbnail);
    }
    writer
        .finish()
        .expect("fixture package must finish")
        .into_inner()
}

fn write_entry<W: Write + std::io::Seek>(
    writer: &mut ZipWriter<W>,
    options: SimpleFileOptions,
    name: &str,
    bytes: &[u8],
) {
    writer
        .start_file(name, options)
        .expect("fixture entry must start");
    writer.write_all(bytes).expect("fixture entry must write");
}

fn must<T>(result: Result<T, CoreError>) -> T {
    result.expect("3MF fixture operation must succeed")
}

#[test]
fn python_assembly_fixture_preserves_units_graph_materials_metadata_and_thumbnail() {
    let package = make_package(
        ASSEMBLY_MODEL,
        Some(MODEL_RELATIONSHIPS),
        None,
        Some(b"png fixture"),
    );
    let imported = must(load_3mf_bytes(&package));

    assert_eq!(imported.unit, ThreeMfUnit::Centimeter);
    assert_eq!(imported.unit_scale_mm, 10.0);
    assert_eq!(
        imported.metadata.get("Title"),
        Some(&vec!["Assembly fixture".to_owned()])
    );
    assert_eq!(
        imported.metadata.get("Designer"),
        Some(&vec!["Builder3D tests".to_owned()])
    );
    assert_eq!(
        imported.thumbnail_path.as_deref(),
        Some("Metadata/thumbnail.png")
    );
    assert_eq!(
        imported.thumbnail_data.as_deref(),
        Some(b"png fixture".as_slice())
    );
    assert_eq!(imported.items.len(), 1);

    let item = &imported.items[0];
    assert_eq!(item.name, "Placed assembly");
    assert_eq!(item.object_id, "1");
    assert_eq!(item.mesh.extents(), [10.0, 20.0, 30.0]);
    assert_eq!(item.transform.position(), [70.0, 90.0, 110.0]);
    assert_eq!(
        item.color,
        [200.0 / 255.0, 100.0 / 255.0, 50.0 / 255.0, 204.0 / 255.0]
    );
    assert_eq!(item.face_colors, Some(vec![[200, 100, 50, 204]]));
    assert_eq!(item.metadata, imported.metadata);
    assert!(
        imported
            .package_parts
            .contains(&"3D/3dmodel.model".to_owned())
    );
}

#[test]
fn production_external_part_is_resolved_only_when_declared_by_relationships() {
    let package = make_package(
        EXTERNAL_ROOT_MODEL,
        Some(EXTERNAL_MODEL_RELATIONSHIPS),
        Some(EXTERNAL_PART_MODEL),
        None,
    );
    let imported = must(load_3mf_bytes(&package));
    assert_eq!(imported.items.len(), 1);
    let item = &imported.items[0];
    assert_eq!(item.name, "External placement");
    assert_eq!(item.object_id, "1");
    assert_eq!(item.mesh.extents(), [1.0, 2.0, 3.0]);
    assert_eq!(item.transform.position(), [7.0, 9.0, 11.0]);
    assert!(
        imported
            .package_parts
            .contains(&"3D/Objects/object_1.model".to_owned())
    );

    let undeclared = make_package(EXTERNAL_ROOT_MODEL, None, Some(EXTERNAL_PART_MODEL), None);
    let error = load_3mf_bytes(&undeclared).expect_err("undeclared part must fail");
    assert!(error.to_string().contains("not declared by the root model"));
}

#[test]
fn namespaced_numeric_attributes_use_the_reference_compatibility_fallback() {
    let model = br#"<?xml version="1.0" encoding="UTF-8"?>
<model xmlns="http://schemas.microsoft.com/3dmanufacturing/core/2015/02" xmlns:e="urn:builder3d:extension" unit="millimeter">
  <resources>
    <object id="1" name="Namespaced mesh">
      <mesh>
        <vertices>
          <vertex e:x="0" e:y="0" e:z="0" />
          <vertex e:x="1" e:y="2" e:z="3" />
          <vertex e:x="0" e:y="0" e:z="3" />
        </vertices>
        <triangles>
          <triangle e:v1="0" e:v2="1" e:v3="2" />
        </triangles>
      </mesh>
    </object>
  </resources>
  <build><item objectid="1" /></build>
</model>"#;
    let imported = must(load_3mf_bytes(&make_package(model, None, None, None)));
    assert_eq!(imported.items.len(), 1);
    assert_eq!(imported.items[0].mesh.extents(), [1.0, 2.0, 3.0]);
}

#[test]
fn thumbnail_loading_does_not_parse_model_geometry() {
    let malformed = b"<?xml version=\"1.0\"?><model><!DOCTYPE model [<!ENTITY x \"x\">]></model>";
    let package = make_package(malformed, Some(MODEL_RELATIONSHIPS), None, Some(b"preview"));
    assert_eq!(
        must(load_3mf_thumbnail_bytes(&package)),
        Some(b"preview".to_vec())
    );
    let error = load_3mf_bytes(&package).expect_err("entity model must fail");
    assert!(error.to_string().contains("Invalid 3MF model XML"));
}

#[test]
fn malformed_geometry_entities_and_package_paths_are_rejected() {
    let empty_mesh = String::from_utf8(ASSEMBLY_MODEL.to_vec())
        .expect("fixture XML is UTF-8")
        .replace(
            "<vertices>\n          <vertex x=\"0\" y=\"0\" z=\"0\" />\n          <vertex x=\"1\" y=\"2\" z=\"3\" />\n          <vertex x=\"0\" y=\"0\" z=\"3\" />\n        </vertices>",
            "<vertices />",
        );
    let error = load_3mf_bytes(&make_package(empty_mesh.as_bytes(), None, None, None))
        .expect_err("empty mesh must fail");
    assert!(error.to_string().contains("contains an empty mesh"));

    let nonfinite = String::from_utf8(ASSEMBLY_MODEL.to_vec())
        .expect("fixture XML is UTF-8")
        .replacen("x=\"0\" y=\"0\" z=\"0\"", "x=\"nan\" y=\"0\" z=\"0\"", 1);
    let error = load_3mf_bytes(&make_package(nonfinite.as_bytes(), None, None, None))
        .expect_err("non-finite vertex must fail");
    assert!(error.to_string().contains("finite float32"));

    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    write_entry(&mut writer, options, "3D/3dmodel.model", ASSEMBLY_MODEL);
    write_entry(&mut writer, options, "3d/3DMODEL.MODEL", ASSEMBLY_MODEL);
    let duplicate = writer
        .finish()
        .expect("duplicate fixture must finish")
        .into_inner();
    let error = load_3mf_bytes(&duplicate).expect_err("duplicate path must fail");
    assert!(error.to_string().contains("duplicate path"));

    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    write_entry(&mut writer, options, "../3D/3dmodel.model", ASSEMBLY_MODEL);
    let traversal = writer
        .finish()
        .expect("traversal fixture must finish")
        .into_inner();
    let error = load_3mf_bytes(&traversal).expect_err("traversal path must fail");
    assert!(error.to_string().contains("invalid path"));
}

#[test]
fn explicit_limits_cover_package_relationship_xml_metadata_and_mesh_boundaries() {
    let package = make_package(
        ASSEMBLY_MODEL,
        Some(MODEL_RELATIONSHIPS),
        None,
        Some(b"preview"),
    );

    let limits = ThreeMfLimits {
        archive_entries: 1,
        ..ThreeMfLimits::default()
    };
    let error = load_3mf_bytes_with_limits(&package, &limits).expect_err("entry limit must fail");
    assert!(error.to_string().contains("more than 1 entries"));

    let limits = ThreeMfLimits {
        member_bytes: 16,
        ..ThreeMfLimits::default()
    };
    let error = load_3mf_bytes_with_limits(&package, &limits).expect_err("member limit must fail");
    assert!(error.to_string().contains("ZIP entry exceeds"));

    let limits = ThreeMfLimits {
        model_xml_bytes: Some(16),
        ..ThreeMfLimits::default()
    };
    let error =
        load_3mf_bytes_with_limits(&package, &limits).expect_err("model XML limit must fail");
    assert!(error.to_string().contains("model XML exceeds"));

    let limits = ThreeMfLimits {
        relationships: 0,
        ..ThreeMfLimits::default()
    };
    let error =
        load_3mf_bytes_with_limits(&package, &limits).expect_err("relationship limit must fail");
    assert!(
        error
            .to_string()
            .contains("relationships contain more than 0 entries")
    );

    let limits = ThreeMfLimits {
        metadata_bytes: 1,
        ..ThreeMfLimits::default()
    };
    let error =
        load_3mf_bytes_with_limits(&package, &limits).expect_err("metadata limit must fail");
    assert!(error.to_string().contains("metadata exceeds"));

    let limits = ThreeMfLimits {
        mesh_vertices: Some(2),
        ..ThreeMfLimits::default()
    };
    let error = load_3mf_bytes_with_limits(&package, &limits).expect_err("vertex limit must fail");
    assert!(error.to_string().contains("more than 2 vertices"));

    let limits = ThreeMfLimits {
        objects: 1,
        ..ThreeMfLimits::default()
    };
    let error = load_3mf_bytes_with_limits(&package, &limits).expect_err("object limit must fail");
    assert!(error.to_string().contains("more than 1 object resources"));

    let limits = ThreeMfLimits {
        build_items: 0,
        ..ThreeMfLimits::default()
    };
    let error = load_3mf_bytes_with_limits(&package, &limits).expect_err("build limit must fail");
    assert!(error.to_string().contains("more than 0 items"));

    let limits = ThreeMfLimits {
        component_depth: 0,
        ..ThreeMfLimits::default()
    };
    let error = load_3mf_bytes_with_limits(&package, &limits).expect_err("depth limit must fail");
    assert!(error.to_string().contains("nesting exceeds 0 levels"));

    let limits = ThreeMfLimits {
        instances: 0,
        ..ThreeMfLimits::default()
    };
    let error =
        load_3mf_bytes_with_limits(&package, &limits).expect_err("instance limit must fail");
    assert!(error.to_string().contains("more than 0 mesh instances"));
}

#[test]
fn empty_build_does_not_promote_unbuilt_resources() {
    let model = String::from_utf8(ASSEMBLY_MODEL.to_vec())
        .expect("fixture XML is UTF-8")
        .replace(
            "<item objectid=\"2\" partnumber=\"Placed assembly\" transform=\"1 0 0 0 1 0 0 0 1 5 6 7\" />",
            "",
        );
    let imported = must(load_3mf_bytes(&make_package(
        model.as_bytes(),
        None,
        None,
        None,
    )));
    assert!(imported.items.is_empty());
}

#[test]
fn export_round_trip_preserves_geometry_transform_color_and_metadata() {
    let transform = Transform::from_rows([
        [0.0, -1.0, 0.0, 12.0],
        [1.0, 0.0, 0.0, 23.0],
        [0.0, 0.0, 1.0, 34.0],
        [0.0, 0.0, 0.0, 1.0],
    ])
    .expect("fixture transform is valid");
    let mut item = ThreeMfItem::new(
        make_box([2.0, 4.0, 6.0]).expect("fixture mesh is valid"),
        transform,
        "Exported fixture".to_owned(),
        [0.25, 0.5, 0.75, 0.8],
    )
    .expect("fixture item is valid");
    item.metadata
        .insert("Title".to_owned(), vec!["Round-trip fixture".to_owned()]);
    item.metadata
        .insert("Tags".to_owned(), vec!["one".to_owned(), "two".to_owned()]);

    let bytes = must(export_3mf(&[item], None, None));
    let mut archive = ZipArchive::new(Cursor::new(bytes.clone())).expect("export must be ZIP");
    assert!(archive.by_name("3D/3dmodel.model").is_ok());
    let mut model = Vec::new();
    archive
        .by_name("3D/3dmodel.model")
        .expect("model part exists")
        .read_to_end(&mut model)
        .expect("model part reads");
    assert!(
        model
            .windows(b"unit=\"millimeter\"".len())
            .any(|window| { window == b"unit=\"millimeter\"" })
    );
    assert!(
        model
            .windows(b"m:basematerials".len())
            .any(|window| window == b"m:basematerials")
    );

    let imported = must(load_3mf_bytes(&bytes));
    assert_eq!(imported.unit, ThreeMfUnit::Millimeter);
    assert_eq!(
        imported.metadata.get("Tags"),
        Some(&vec!["one".to_owned(), "two".to_owned()])
    );
    assert_eq!(imported.items.len(), 1);
    assert_eq!(imported.items[0].mesh.extents(), [2.0, 4.0, 6.0]);
    assert_eq!(imported.items[0].transform, transform);
    assert_eq!(
        imported.items[0].color,
        [64.0 / 255.0, 128.0 / 255.0, 191.0 / 255.0, 204.0 / 255.0]
    );
}

#[test]
fn single_mesh_format_bridge_reads_and_writes_3mf() {
    let mesh = make_box([2.0, 3.0, 4.0]).expect("fixture mesh is valid");
    let bytes = must(builder3d_core::export_mesh(&mesh, MeshFormat::ThreeMf));
    let asset = must(builder3d_core::load_mesh_asset_bytes(
        MeshFormat::ThreeMf,
        &bytes,
    ));
    assert_eq!(asset.mesh(), &mesh);
}
