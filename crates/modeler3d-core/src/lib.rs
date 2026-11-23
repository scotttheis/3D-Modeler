#![deny(unsafe_code)]

//! Portable 3D Modeler scene, geometry, command, history, and project services.
//!
//! Desktop and automation adapters share the same command and project
//! boundaries. The optional Windows repair adapter is isolated from the
//! portable geometry services and uses only the public WinRT API.

mod brushes;
mod commands;
mod density;
mod error;
mod file_io;
mod gltf;
mod material;
mod measure;
mod mesh;
mod mesh_ops;
mod model_import;
mod paint;
mod primitives;
mod project;
mod repair;
mod repair_native;
mod scene;
mod shell_ops;
mod simplify;
mod smooth;
mod subdivide;
mod texture;
mod three_mf;
mod transform;
mod windows_repair;
mod wrl;
mod wrl_import;
pub use model_import::{PreparedModelImport, import_model, prepare_model_import};
pub use wrl_import::{load_wrl, load_wrl_bytes};

mod placement;
pub use placement::{MAX_PLACEMENT_POINTS, placement_rotation, settle_on_floor};

pub use brushes::{
    BlobOptions, BrushDirection, BrushFalloff, BrushReport, BrushResult, BrushSymmetry,
    CreasePolishOptions, ElasticGrabOptions, SmoothBrushOptions, blob, blob_mesh, crease_polish,
    crease_polish_mesh, elastic_grab, elastic_grab_mesh, smooth_brush, smooth_brush_mesh,
};
pub use commands::{
    AddObjectCommand, Command, CommandRegistry, CommandRequest, CommandSpec, GroupedCommand,
    RemoveObjectCommand, RenameObjectCommand, ReplaceMeshCommand, SetColorCommand,
    SetMetadataCommand, SetVisibilityCommand, TransformObjectCommand,
};
pub use density::{
    NORMALIZE_DENSITY_DEFAULT_RADIUS_MM, NORMALIZE_DENSITY_MAX_COLLAPSES_PER_ITERATION,
    NORMALIZE_DENSITY_MAX_EDGE_REBUILDS, NORMALIZE_DENSITY_MAX_FACES,
    NORMALIZE_DENSITY_MAX_ITERATIONS, NORMALIZE_DENSITY_MAX_REGION_FACES,
    NORMALIZE_DENSITY_MAX_SOURCE_FACES, NORMALIZE_DENSITY_MAX_WORK_UNITS,
    NORMALIZE_DENSITY_MAX_WORKING_REGION_FACES, NORMALIZE_DENSITY_RADIUS_MAX_MM,
    NORMALIZE_DENSITY_RADIUS_MIN_MM, NORMALIZE_DENSITY_TARGET_MAX_MM,
    NORMALIZE_DENSITY_TARGET_MIN_MM, NormalizeDensityCancel, NormalizeDensityOptions,
    NormalizeDensityProgress, NormalizeDensityReport, NormalizeDensityResult,
    normalize_face_density, normalize_face_density_with_hooks, select_face_region,
};
pub use error::CoreError;
pub use file_io::{
    MAX_MESH_FACES, MAX_MESH_FILE_BYTES, MAX_MESH_VERTICES, MeshAsset, MeshFormat, export_mesh,
    export_mesh_asset, load_mesh, load_mesh_asset, load_mesh_asset_bytes, load_mesh_bytes,
    save_mesh, save_mesh_asset,
};
pub use gltf::{
    GltfExportReport, GltfItem, GltfLimits, MAX_GLTF_ACCESSORS, MAX_GLTF_BUFFER_BYTES,
    MAX_GLTF_BUFFER_VIEWS, MAX_GLTF_BUFFERS, MAX_GLTF_CHUNKS, MAX_GLTF_FACES, MAX_GLTF_FILE_BYTES,
    MAX_GLTF_ITEMS, MAX_GLTF_JSON_BYTES, MAX_GLTF_MATERIALS, MAX_GLTF_MESHES, MAX_GLTF_NODE_DEPTH,
    MAX_GLTF_NODES, MAX_GLTF_TOTAL_BUFFER_BYTES, MAX_GLTF_VERTICES, export_glb, export_gltf,
    load_gltf, load_gltf_bytes, load_gltf_bytes_from_directory, load_gltf_bytes_with_limits,
    load_gltf_with_limits, save_glb, save_gltf,
};
pub use material::{
    BOOLEAN_COLOR_PROVENANCE_KEY, BooleanColorProvenance, boolean_color_provenance, color_rgba8,
    with_boolean_color_provenance, with_boolean_target_color, without_boolean_color_provenance,
};
pub use measure::{
    MEASURE_MAX_PAIR_CHECKS, Measurement, closest_point_on_mesh, closest_points_between_meshes,
    measure_points,
};
pub use mesh::Mesh;
pub use mesh_ops::{
    BooleanFaceSource, BooleanOperation, BooleanOptions, BooleanReport, BooleanResult,
    MeshHealthReport, MeshOperationLimits, boolean_difference, boolean_intersection,
    boolean_operation, boolean_operation_observed, boolean_union, inspect_mesh,
};
pub use paint::{
    FACE_PAINT_KEY, FacePaint, MAX_FACE_PAINT_COLORS, PaintFacesCommand, face_paint,
    object_face_colors, paint_region_faces, without_face_paint,
};
pub use primitives::{
    DEFAULT_PRIMITIVE_RADIUS_MM, DEFAULT_PRIMITIVE_SIZE_MM, PrimitiveKind, PrimitiveMetadata,
    TORUS_RADIUS_MAX_MM, TORUS_RADIUS_MIN_MM, TORUS_TUBE_MAX_MM, TORUS_TUBE_MIN_MM,
    TorusParameters, build_parametric_torus, make_box, make_cone, make_cylinder, make_hexagon,
    make_primitive, make_pyramid, make_sphere, make_tetrahedron, make_torus, make_wedge,
    normalize_torus_parameters, primitive_default_color, torus_metadata,
    torus_parameters_from_metadata,
};
pub use project::{
    LEGACY_PROJECT_SCHEMA_VERSION, MAX_PROJECT_ARCHIVE_BYTES, MAX_PROJECT_ARCHIVE_ENTRIES,
    MAX_PROJECT_COMPRESSION_CHECK_BYTES, MAX_PROJECT_COMPRESSION_RATIO, MAX_PROJECT_GROUP_DEPTH,
    MAX_PROJECT_MEMBER_BYTES, MAX_PROJECT_MESH_FACES, MAX_PROJECT_MESH_VERTICES,
    MAX_PROJECT_METADATA_BYTES, MAX_PROJECT_OBJECT_METADATA_BYTES, MAX_PROJECT_OBJECTS,
    MAX_PROJECT_SCENE_JSON_BYTES, MAX_PROJECT_TOTAL_MESH_FACES, MAX_PROJECT_TOTAL_MESH_VERTICES,
    MAX_PROJECT_UNCOMPRESSED_BYTES, PROJECT_FORMAT, PROJECT_SCHEMA_VERSION, load_project,
    load_project_bytes, referenced_texture_bytes, referenced_texture_ids, save_project,
    save_project_bytes,
};
pub use repair::{
    PlanarCleanupDetail, PlanarCleanupReport, REPAIR_DEFAULT_PROFILE,
    REPAIR_NOTHING_TO_FIX_MESSAGE, REPAIR_REFUSED_MESSAGE, REPAIR_TINY_SHELL_TOLERANCE_MM,
    REPAIR_TOLERANCE_MM, RepairEngine, RepairProfile, RepairReport, RepairResult,
    RepairRouteControl, RepairScore, RepairStageReport, RepairVerdict, repair_mesh,
    repair_mesh_routed, repair_mesh_with_options, repair_mesh_with_profile, repair_time_estimate,
    repair_time_limit,
};
pub use scene::{
    COPY_GAP_MM, CommandResult, DEFAULT_MESH_COLOR, MERGE_NOT_TOUCHING_MESSAGE, ObjectId,
    PLACEMENT_MIN_OBJECTS, PLACEMENT_NEEDS_TWO_MESSAGE, PreparedBoolean, ProductBooleanOutcome,
    Scene, SceneChild, SceneObject, SceneSnapshot,
};
pub use shell_ops::{
    EMBOSS_DEPTH_MAX_MM, EMBOSS_FONT_SIZE_MAX, EMBOSS_FONT_SIZE_MIN, EMBOSS_MAX_TEXT_LENGTH,
    EMBOSS_OFFSET_MAX_MM, EmbossFont, HOLLOW_MAX_AXIS_CELLS, HOLLOW_MAX_VOXELS, HollowOptions,
    HollowReport, HollowResult, TextEmbossOptions, TextEmbossReport, TextEmbossResult, hollow_mesh,
    hollow_mesh_with_hooks, text_emboss, text_emboss_with_hooks,
};
pub use simplify::{
    DEFAULT_SIMPLIFY_MIN_FACES, SIMPLIFY_SHAPE_TOLERANCE, SimplifyReport, SimplifyResult,
    simplify_mesh, simplify_mesh_by_percent, simplify_mesh_by_percent_in_area,
    simplify_mesh_with_options, simplify_target_for_percent,
};
pub use smooth::{
    DEFAULT_SMOOTH_MAX_ITERATIONS, SmoothReport, SmoothResult, smooth_mesh,
    smooth_mesh_with_options,
};
pub use subdivide::{
    SubdivideReport, SubdivideResult, faces_near_path, subdivide_mesh, subdivide_mesh_even,
    subdivide_mesh_with_options,
};
pub use texture::{
    BOX_BLEND_POWER, DEFAULT_TEXTURE_TILE_MM, MAX_PROJECT_TEXTURE_BYTES, MAX_TEXTURE_SIDE_PX,
    MAX_TEXTURE_SOURCE_BYTES, MAX_TEXTURE_STORED_BYTES, MAX_TEXTURE_TILE_MM, MIN_TEXTURE_TILE_MM,
    ObjectTexture, TEXTURE_METADATA_KEY, TextureEncoding, TextureImage, box_axis, box_uv,
    box_weights, clamp_tile_mm, import_texture_file, object_texture, register_texture,
    resolved_texture, texture_by_id, with_texture, without_texture,
};
pub use three_mf::{
    MAX_3MF_ARCHIVE_BYTES, MAX_3MF_ARCHIVE_ENTRIES, MAX_3MF_BUILD_ITEMS, MAX_3MF_COMPONENT_DEPTH,
    MAX_3MF_COMPRESSION_CHECK_BYTES, MAX_3MF_COMPRESSION_RATIO, MAX_3MF_INSTANCES,
    MAX_3MF_MATERIAL_COLORS, MAX_3MF_MEMBER_BYTES, MAX_3MF_MESH_FACES, MAX_3MF_MESH_VERTICES,
    MAX_3MF_METADATA_BYTES, MAX_3MF_MODEL_XML_BYTES, MAX_3MF_OBJECTS, MAX_3MF_RELATIONSHIPS,
    MAX_3MF_RELATIONSHIPS_XML_BYTES, MAX_3MF_THUMBNAIL_BYTES, MAX_3MF_TOTAL_MESH_FACES,
    MAX_3MF_TOTAL_MESH_VERTICES, MAX_3MF_UNCOMPRESSED_BYTES, ThreeMfImport, ThreeMfItem,
    ThreeMfLimits, ThreeMfMetadata, ThreeMfRelationship, ThreeMfUnit, export_3mf, load_3mf,
    load_3mf_bytes, load_3mf_bytes_with_limits, load_3mf_thumbnail, load_3mf_thumbnail_bytes,
    load_3mf_thumbnail_bytes_with_limits, load_3mf_thumbnail_with_limits, load_3mf_with_limits,
    save_3mf,
};
pub use transform::Transform;
pub use wrl::{
    MAX_WRL_FACES, MAX_WRL_FILE_BYTES, MAX_WRL_OBJECTS, MAX_WRL_VERTICES, WRL_SUFFIX,
    WrlExportReport, WrlItem, WrlLimits, export_wrl, export_wrl_mesh, export_wrl_with_limits,
    save_wrl, save_wrl_mesh,
};

#[cfg(test)]
mod tests {
    use super::*;

    fn mesh(result: Result<Mesh, CoreError>) -> Mesh {
        match result {
            Ok(mesh) => mesh,
            Err(error) => panic!("unexpected mesh error: {error}"),
        }
    }

    #[test]
    fn primitive_catalog_keeps_reference_counts_and_extents() {
        let box_mesh = mesh(make_box([40.0, 40.0, 40.0]));
        assert_eq!(box_mesh.vertex_count(), 8);
        assert_eq!(box_mesh.face_count(), 12);
        assert_eq!(box_mesh.extents(), [40.0, 40.0, 40.0]);

        let sphere = mesh(make_sphere(20.0, 4));
        assert_eq!(sphere.vertex_count(), 2_562);
        assert_eq!(sphere.face_count(), 5_120);

        let cylinder = mesh(make_cylinder(20.0, 40.0, 119));
        assert_eq!(cylinder.vertex_count(), 240);
        assert_eq!(cylinder.face_count(), 476);

        let cone = mesh(make_cone(20.0, 40.0, 192));
        assert_eq!(cone.vertex_count(), 194);
        assert_eq!(cone.face_count(), 384);

        let torus = mesh(make_torus(25.0, 10.0, 100, 100));
        assert_eq!(torus.vertex_count(), 10_000);
        assert_eq!(torus.face_count(), 20_000);

        assert_eq!(mesh(make_pyramid(20.0, 40.0)).face_count(), 6);
        assert_eq!(mesh(make_hexagon(28.7935, 15.48893)).face_count(), 20);
        assert_eq!(mesh(make_wedge(40.002, 40.002, 19.998)).face_count(), 8);
        assert_eq!(mesh(make_tetrahedron(20.0)).face_count(), 4);
    }

    #[test]
    fn commands_are_noop_aware_and_undoable() {
        let mut scene = Scene::new();
        let added = match scene.add_object(SceneObject::new(mesh(make_box([2.0, 2.0, 2.0])))) {
            Ok(result) => result,
            Err(error) => panic!("add failed: {error}"),
        };
        assert!(added.changed);
        assert_eq!(scene.revision(), 1);
        let same = match scene.set_transform(0, Transform::identity()) {
            Ok(result) => result,
            Err(error) => panic!("transform failed: {error}"),
        };
        assert!(!same.changed);
        assert_eq!(scene.revision(), 1);
        let moved = match scene.translate(0, [10.0, 2.0, -3.0]) {
            Ok(result) => result,
            Err(error) => panic!("translate failed: {error}"),
        };
        assert!(moved.changed);
        let position = match scene.object(0) {
            Ok(object) => object.position(),
            Err(error) => panic!("object lookup failed: {error}"),
        };
        assert_eq!(position, [10.0, 2.0, -3.0]);
        let undone = match scene.undo() {
            Ok(Some(result)) => result,
            Ok(None) => panic!("undo did not produce a result"),
            Err(error) => panic!("undo failed: {error}"),
        };
        assert!(undone.changed);
        let position = match scene.object(0) {
            Ok(object) => object.position(),
            Err(error) => panic!("object lookup failed: {error}"),
        };
        assert_eq!(position, [0.0, 0.0, 0.0]);
        let redone = match scene.redo() {
            Ok(Some(result)) => result,
            Ok(None) => panic!("redo did not produce a result"),
            Err(error) => panic!("redo failed: {error}"),
        };
        assert!(redone.changed);
        let position = match scene.object(0) {
            Ok(object) => object.position(),
            Err(error) => panic!("object lookup failed: {error}"),
        };
        assert_eq!(position, [10.0, 2.0, -3.0]);
    }

    #[test]
    fn project_round_trip_preserves_bytes_after_load() {
        let mut scene = Scene::new();
        let object = match SceneObject::with_details(
            mesh(make_box([2.0, 2.0, 2.0])),
            Some("fixture-object".to_owned()),
            Some("Fixture box".to_owned()),
            Some([33.0 / 255.0, 187.0 / 255.0, 76.0 / 255.0, 1.0]),
            Some(serde_json::json!({ "source": "r1" })),
        ) {
            Ok(object) => object,
            Err(error) => panic!("object failed: {error}"),
        };
        if let Err(error) = scene.add_object(object) {
            panic!("add failed: {error}");
        }
        let bytes = match save_project_bytes(&scene) {
            Ok(bytes) => bytes,
            Err(error) => panic!("save failed: {error}"),
        };
        let loaded = match load_project_bytes(&bytes) {
            Ok(scene) => scene,
            Err(error) => panic!("load failed: {error}"),
        };
        let round_trip = match save_project_bytes(&loaded) {
            Ok(bytes) => bytes,
            Err(error) => panic!("round trip failed: {error}"),
        };
        assert_eq!(bytes, round_trip);
        assert_eq!(loaded.project_id(), scene.project_id());
        assert_eq!(loaded.objects()[0].object_id(), "fixture-object");
    }

    #[test]
    fn command_registry_dispatches_primitives() {
        let registry = CommandRegistry::core();
        assert!(registry.contains("add_primitive"));
        let mut scene = Scene::new();
        let result = registry.dispatch(
            &mut scene,
            CommandRequest::AddPrimitive {
                kind: PrimitiveKind::Cube,
                dimensions_mm: Some([12.0, 18.0, 24.0]),
                name: Some("Registry cube".to_owned()),
            },
        );
        match result {
            Ok(result) => assert!(result.changed),
            Err(error) => panic!("dispatch failed: {error}"),
        }
        assert_eq!(scene.objects()[0].name(), "Registry cube");
        assert_eq!(scene.objects()[0].mesh().extents(), [12.0, 18.0, 24.0]);
    }

    #[test]
    fn scene_graph_group_duplicate_and_ungroup_are_undoable() {
        let mut scene = Scene::new();
        let first = match SceneObject::with_details(
            mesh(make_box([2.0, 2.0, 2.0])),
            Some("first".to_owned()),
            Some("First".to_owned()),
            Some([0.2, 0.4, 0.6, 1.0]),
            Some(serde_json::json!({ "slot": 1 })),
        ) {
            Ok(object) => object,
            Err(error) => panic!("first object failed: {error}"),
        };
        let second = match SceneObject::with_details(
            mesh(make_box([3.0, 3.0, 3.0])),
            Some("second".to_owned()),
            Some("Second".to_owned()),
            Some([0.2, 0.4, 0.6, 1.0]),
            None,
        ) {
            Ok(object) => object,
            Err(error) => panic!("second object failed: {error}"),
        };
        if let Err(error) = scene.add_object(first) {
            panic!("first add failed: {error}");
        }
        if let Err(error) = scene.add_object(second) {
            panic!("second add failed: {error}");
        }
        if let Err(error) = scene.translate(1, [5.0, 0.0, 0.0]) {
            panic!("translation failed: {error}");
        }
        scene.set_selection([0, 1]);
        let group_result = match scene.group_selected() {
            Ok(result) => result,
            Err(error) => panic!("group failed: {error}"),
        };
        assert!(group_result.changed);
        assert_eq!(scene.objects().len(), 1);
        assert_eq!(scene.objects()[0].children().len(), 2);
        assert_eq!(scene.objects()[0].mesh().vertex_count(), 16);
        assert_eq!(
            scene.selection().iter().copied().collect::<Vec<_>>(),
            vec![0]
        );
        assert_eq!(
            scene.objects()[0].children()[1].transform().position(),
            [5.0, 0.0, 0.0]
        );

        let ungroup_result = match scene.ungroup_selected() {
            Ok(result) => result,
            Err(error) => panic!("ungroup failed: {error}"),
        };
        assert!(ungroup_result.changed);
        assert_eq!(scene.objects().len(), 2);
        assert!(scene.selection().is_empty());
        assert_eq!(scene.objects()[1].position(), [5.0, 0.0, 0.0]);

        let undone = match scene.undo() {
            Ok(Some(result)) => result,
            Ok(None) => panic!("ungroup undo did not produce a result"),
            Err(error) => panic!("ungroup undo failed: {error}"),
        };
        assert!(undone.changed);
        assert_eq!(scene.objects().len(), 1);
        let redone = match scene.redo() {
            Ok(Some(result)) => result,
            Ok(None) => panic!("ungroup redo did not produce a result"),
            Err(error) => panic!("ungroup redo failed: {error}"),
        };
        assert!(redone.changed);
        assert_eq!(scene.objects().len(), 2);

        scene.set_selection([0]);
        let duplicate_result = match scene.duplicate_selected() {
            Ok(result) => result,
            Err(error) => panic!("duplicate failed: {error}"),
        };
        assert!(duplicate_result.changed);
        assert_eq!(scene.objects().len(), 3);
        assert_eq!(
            scene.selection().iter().copied().collect::<Vec<_>>(),
            vec![2]
        );
        assert_ne!(
            scene.objects()[0].object_id(),
            scene.objects()[2].object_id()
        );
        // OF-43: beside the 2 mm box with a 10 mm gap (was 1 mm in Python).
        assert_eq!(scene.objects()[2].position(), [12.0, 0.0, 0.0]);
    }

    #[test]
    fn visibility_command_reports_id_and_round_trips() {
        let mut scene = Scene::new();
        if let Err(error) = scene.add_object(SceneObject::new(mesh(make_box([2.0, 2.0, 2.0])))) {
            panic!("add failed: {error}");
        }
        let object_id = scene.objects()[0].object_id().to_owned();
        let result = match scene.set_visible(0, false) {
            Ok(result) => result,
            Err(error) => panic!("visibility failed: {error}"),
        };
        assert_eq!(result.affected_object_ids, vec![object_id]);
        assert!(!scene.objects()[0].is_visible());
        if let Err(error) = scene.undo() {
            panic!("visibility undo failed: {error}");
        }
        assert!(scene.objects()[0].is_visible());
        if let Err(error) = scene.redo() {
            panic!("visibility redo failed: {error}");
        }
        assert!(!scene.objects()[0].is_visible());
    }

    #[test]
    fn path_save_replaces_existing_archive_atomically() {
        let path =
            std::env::temp_dir().join(format!("modeler3d-r1-atomic-{}.3dp", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let mut scene = Scene::new();
        if let Err(error) = scene.add_object(SceneObject::new(mesh(make_box([2.0, 2.0, 2.0])))) {
            panic!("add failed: {error}");
        }
        if let Err(error) = save_project(&scene, &path) {
            panic!("first path save failed: {error}");
        }
        let first = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) => panic!("first archive read failed: {error}"),
        };
        if let Err(error) = scene.set_visible(0, false) {
            panic!("visibility failed: {error}");
        }
        if let Err(error) = save_project(&scene, &path) {
            panic!("replace path save failed: {error}");
        }
        let second = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) => panic!("second archive read failed: {error}"),
        };
        assert_ne!(first, second);
        let loaded = match load_project(&path) {
            Ok(scene) => scene,
            Err(error) => panic!("replaced archive load failed: {error}"),
        };
        assert!(!loaded.objects()[0].is_visible());
        let _ = std::fs::remove_file(path);
    }
}
