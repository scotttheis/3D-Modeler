use modeler3d_core::{
    CoreError, DEFAULT_MESH_COLOR, PROJECT_FORMAT, PROJECT_SCHEMA_VERSION, load_project_bytes,
    save_project_bytes,
};

const PYTHON_CURRENT: &[u8] = include_bytes!("fixtures/projects/python-current.3dp");
const PYTHON_LEGACY: &[u8] = include_bytes!("fixtures/projects/python-legacy.3dp");
const PYTHON_FUTURE: &[u8] = include_bytes!("fixtures/projects/python-future.3dp");
const PYTHON_MALFORMED_JSON: &[u8] = include_bytes!("fixtures/projects/python-malformed-json.3dp");
const PYTHON_INVALID_TRANSFORM: &[u8] =
    include_bytes!("fixtures/projects/python-invalid-transform.3dp");
const PYTHON_MISSING_MESH: &[u8] = include_bytes!("fixtures/projects/python-missing-mesh.3dp");

#[test]
fn python_current_project_restores_fields_geometry_and_original_archive_bytes() {
    assert_eq!(PROJECT_FORMAT, "Builder3D");
    assert_eq!(PROJECT_SCHEMA_VERSION, 1);

    let scene = load_project_bytes(PYTHON_CURRENT).expect("Python current project should load");
    assert_eq!(scene.project_id(), "python-rb03-project");
    assert_eq!(scene.revision(), 7);
    assert_eq!(scene.objects().len(), 1);

    let object = &scene.objects()[0];
    assert_eq!(object.object_id(), "python-rb03-object");
    assert_eq!(object.name(), "Python golden fixture");
    assert!(!object.is_visible());
    assert_eq!(
        object.metadata(),
        &serde_json::json!({
            "origin": "python-reference",
            "tags": ["golden", "rb03"]
        })
    );
    assert_eq!(object.color(), [0.1, 0.25, 0.75, 0.5]);
    assert_eq!(
        object.transform().rows(),
        [
            [1.5, 0.0, 0.0, 11.5],
            [0.0, 2.0, 0.0, -4.0],
            [0.0, 0.0, 0.5, 7.25],
            [0.0, 0.0, 0.0, 1.0],
        ]
    );
    assert_eq!(object.mesh().extents(), [2.0, 4.0, 6.0]);
    assert_eq!(
        object.mesh().bounds(),
        Some(([-1.0, -2.0, -3.0], [1.0, 2.0, 3.0]))
    );

    let saved = save_project_bytes(&scene).expect("untouched project should serialize");
    assert_eq!(saved, PYTHON_CURRENT);
}

#[test]
fn python_legacy_project_uses_schema_zero_defaults() {
    let scene = load_project_bytes(PYTHON_LEGACY).expect("Python legacy project should load");
    assert!(!scene.project_id().is_empty());
    assert_eq!(scene.revision(), 0);
    assert_eq!(scene.objects().len(), 1);

    let object = &scene.objects()[0];
    assert!(!object.object_id().is_empty());
    assert_eq!(object.name(), "Python golden fixture");
    assert_eq!(
        object.transform().rows(),
        modeler3d_core::Transform::identity().rows()
    );
    assert_eq!(object.color(), DEFAULT_MESH_COLOR);
    assert!(object.is_visible());
    assert_eq!(object.metadata(), &serde_json::json!({}));
    assert_eq!(object.mesh().extents(), [2.0, 4.0, 6.0]);
}

#[test]
fn future_project_schema_version_is_rejected() {
    assert!(matches!(
        load_project_bytes(PYTHON_FUTURE),
        Err(CoreError::UnsupportedProjectVersion(99))
    ));
}

#[test]
fn malformed_project_inputs_return_project_format_errors() {
    let malformed = load_project_bytes(PYTHON_MALFORMED_JSON);
    assert!(matches!(
        malformed,
        Err(CoreError::ProjectFormat(message)) if message.contains("scene.json is not valid UTF-8 JSON")
    ));

    let corrupt = load_project_bytes(b"not a 3D Modeler ZIP archive");
    assert!(matches!(
        corrupt,
        Err(CoreError::ProjectFormat(message)) if message.contains("not a readable ZIP archive")
    ));
}

#[test]
fn invalid_transform_and_missing_mesh_are_rejected() {
    let transform = load_project_bytes(PYTHON_INVALID_TRANSFORM);
    assert!(matches!(
        transform,
        Err(CoreError::ProjectFormat(message)) if message.contains("objects[0].transform")
    ));

    let missing_mesh = load_project_bytes(PYTHON_MISSING_MESH);
    assert!(matches!(
        missing_mesh,
        Err(CoreError::ProjectFormat(message)) if message.contains("meshes/mesh_0.stl")
    ));
}
