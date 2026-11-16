//! Application export selection and format routing.
//!
//! The core crate owns individual format writers. This module owns the
//! application contract for which scene objects are exported and how their
//! world transforms, names, and display colors cross that boundary.

use std::path::Path;

use modeler3d_core::{
    CoreError, GltfItem, Mesh, MeshAsset, Scene, ThreeMfItem, WrlItem, save_3mf, save_gltf,
    save_mesh, save_mesh_asset, save_wrl,
};
use thiserror::Error;

/// A completed application export.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExportReport {
    /// Requested output path.
    pub path: String,
    /// Number of visible scene objects written.
    pub objects_exported: usize,
    /// Whether the application had to write individual fallback files.
    pub individual_fallback: bool,
}

/// Errors at the application export boundary.
#[derive(Debug, Error)]
pub enum ExportError {
    /// A core format writer rejected the scene.
    #[error(transparent)]
    Core(#[from] CoreError),
    /// The output path is empty or too long.
    #[error("invalid export path: {0}")]
    InvalidPath(String),
    /// No visible object was eligible for export.
    #[error("no visible objects to export")]
    NoVisibleObjects,
    /// A document save was asked for a scene with no objects.
    #[error("the scene has no objects to save")]
    EmptyScene,
    /// The application does not route this extension.
    #[error("unsupported export extension: {0}")]
    UnsupportedExtension(String),
}

/// Returns selected visible objects, or all visible objects when there is no
/// active selection, matching the Python workflow service.
pub fn export_indices(scene: &Scene) -> Result<Vec<usize>, ExportError> {
    let candidates = if scene.selection().is_empty() {
        (0..scene.objects().len()).collect::<Vec<_>>()
    } else {
        scene.selection().iter().copied().collect::<Vec<_>>()
    };
    let indices = candidates
        .into_iter()
        .filter(|index| {
            scene
                .objects()
                .get(*index)
                .is_some_and(|object| object.is_visible())
        })
        .collect::<Vec<_>>();
    if indices.is_empty() {
        return Err(ExportError::NoVisibleObjects);
    }
    Ok(indices)
}

/// Exports the application-visible scene using the output extension.
pub fn export_scene(scene: &Scene, path: &Path) -> Result<ExportReport, ExportError> {
    let indices = export_indices(scene)?;
    export_scene_objects(scene, &indices, path)
}

/// Exports an explicit ordered set of visible scene objects.
///
/// This is the seam used by print handoff.  It keeps staging independent from
/// the current selection while reusing the same format routing and transform
/// rules as normal application export.
pub fn export_scene_objects(
    scene: &Scene,
    requested_indices: &[usize],
    path: &Path,
) -> Result<ExportReport, ExportError> {
    validate_path(path)?;
    let indices = requested_indices
        .iter()
        .copied()
        .filter(|index| {
            scene
                .objects()
                .get(*index)
                .is_some_and(|object| object.is_visible())
        })
        .collect::<Vec<_>>();
    write_objects(scene, &indices, path)
}

/// Writes every scene object, hidden ones included, as one mesh document.
///
/// This is the desktop Save path for `.stl` and `.3mf` (owner requests
/// OF-23/OF-24): a save must not drop objects the user only hid, because the
/// saved file becomes the document. Export keeps its visible-only rule.
pub fn save_scene_document(scene: &Scene, path: &Path) -> Result<ExportReport, ExportError> {
    validate_path(path)?;
    if scene.objects().is_empty() {
        return Err(ExportError::EmptyScene);
    }
    let indices = (0..scene.objects().len()).collect::<Vec<_>>();
    write_objects(scene, &indices, path)
}

fn write_objects(
    scene: &Scene,
    indices: &[usize],
    path: &Path,
) -> Result<ExportReport, ExportError> {
    if indices.is_empty() {
        return Err(ExportError::NoVisibleObjects);
    }
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .ok_or_else(|| ExportError::UnsupportedExtension(path.display().to_string()))?;
    match extension.as_str() {
        "stl" | "obj" | "ply" => {
            let preserve_vertex_colors = matches!(extension.as_str(), "obj" | "ply");
            let mut meshes = Vec::with_capacity(indices.len());
            let mut vertex_colors = Vec::new();
            for index in indices {
                let object = scene.object(*index)?;
                let mesh = object.transformed_mesh()?;
                if preserve_vertex_colors {
                    let color = color_rgba8(object.color());
                    vertex_colors.resize(vertex_colors.len() + mesh.vertex_count(), color);
                }
                meshes.push(mesh);
            }
            let mesh = Mesh::concatenate(&meshes)?;
            if preserve_vertex_colors {
                let asset = MeshAsset::new(mesh, Some(vertex_colors))?;
                save_mesh_asset(&asset, path)?;
            } else {
                save_mesh(&mesh, path)?;
            }
        }
        "gltf" | "glb" => {
            let items = indices
                .iter()
                .map(|index| {
                    let object = scene.object(*index)?;
                    GltfItem::new(
                        object.mesh().clone(),
                        *object.transform(),
                        object.name().to_owned(),
                        Some(object.color()),
                    )
                })
                .collect::<Result<Vec<_>, CoreError>>()?;
            save_gltf(&items, path)?;
        }
        "3mf" => {
            let items = indices
                .iter()
                .map(|index| {
                    let object = scene.object(*index)?;
                    let mut item = ThreeMfItem::new(
                        object.mesh().clone(),
                        *object.transform(),
                        object.name().to_owned(),
                        object.color(),
                    )?;
                    // Region paint (OF-44) or boolean source colours.
                    item.face_colors = modeler3d_core::object_face_colors(object);
                    // Material texture (OF-46) travels as a 3MF texture.
                    item.texture = modeler3d_core::object_texture(object.metadata());
                    Ok(item)
                })
                .collect::<Result<Vec<_>, CoreError>>()?;
            save_3mf(&items, path, None, None)?;
        }
        "wrl" => {
            let items = indices
                .iter()
                .map(|index| {
                    let object = scene.object(*index)?;
                    WrlItem::new(
                        object.mesh().clone(),
                        *object.transform(),
                        object.name().to_owned(),
                        Some(object.color()),
                    )
                })
                .collect::<Result<Vec<_>, CoreError>>()?;
            save_wrl(&items, path)?;
        }
        _ => return Err(ExportError::UnsupportedExtension(extension)),
    }
    Ok(ExportReport {
        path: path.display().to_string(),
        objects_exported: indices.len(),
        individual_fallback: false,
    })
}

fn color_rgba8(color: [f32; 4]) -> [u8; 4] {
    color.map(|channel| (channel.clamp(0.0, 1.0) * 255.0).round() as u8)
}

fn validate_path(path: &Path) -> Result<(), ExportError> {
    let display = path.to_string_lossy();
    if display.trim().is_empty() || display.len() > 4096 {
        return Err(ExportError::InvalidPath(display.into_owned()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf, time::SystemTime};

    use super::*;
    use modeler3d_core::{
        BooleanOperation, BooleanOptions, CoreError, SceneObject, Transform,
        boolean_color_provenance, make_box,
    };

    fn must<T>(result: Result<T, CoreError>) -> T {
        match result {
            Ok(value) => value,
            Err(error) => panic!("unexpected scene setup error: {error}"),
        }
    }

    fn temporary_directory(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos());
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/rb03-export-tests")
            .join(format!("{label}-{}-{nonce}", std::process::id()));
        assert!(fs::create_dir_all(&path).is_ok());
        path
    }

    fn assert_vec3_close(actual: [f32; 3], expected: [f32; 3]) {
        for axis in 0..3 {
            assert!(
                (actual[axis] - expected[axis]).abs() <= 1.0e-4,
                "axis {axis}: expected {}, got {}",
                expected[axis],
                actual[axis]
            );
        }
    }

    fn assert_color_close(actual: [f32; 4], expected: [f32; 4]) {
        for channel in 0..4 {
            assert!(
                (actual[channel] - expected[channel]).abs() <= 1.0 / 255.0,
                "channel {channel}: expected {}, got {}",
                expected[channel],
                actual[channel]
            );
        }
    }

    fn assert_bounds(mesh: &Mesh, expected: ([f32; 3], [f32; 3])) {
        let Some(actual) = mesh.bounds() else {
            panic!("exported mesh has no bounds");
        };
        assert_vec3_close(actual.0, expected.0);
        assert_vec3_close(actual.1, expected.1);
    }

    #[derive(Clone, Copy)]
    struct ExpectedExportedInstance {
        name: &'static str,
        extents: [f32; 3],
        position: [f32; 3],
        color: [f32; 4],
    }

    const BLUE_BLOCK: ExpectedExportedInstance = ExpectedExportedInstance {
        name: "Blue block",
        extents: [2.0, 4.0, 6.0],
        position: [10.0, 20.0, 30.0],
        color: [0.2, 0.4, 0.6, 0.8],
    };

    const RED_BLOCK: ExpectedExportedInstance = ExpectedExportedInstance {
        name: "Red block",
        extents: [1.0, 3.0, 5.0],
        position: [-4.0, 5.0, 1.0],
        color: [0.9, 0.2, 0.1, 1.0],
    };

    fn scene_with_objects() -> Scene {
        let mut scene = Scene::new();
        let first = must(SceneObject::with_details(
            must(make_box([2.0, 4.0, 6.0])),
            None,
            Some("Blue block".to_owned()),
            Some([0.2, 0.4, 0.6, 0.8]),
            None,
        ));
        let second = must(SceneObject::with_details(
            must(make_box([1.0, 3.0, 5.0])),
            None,
            Some("Red block".to_owned()),
            Some([0.9, 0.2, 0.1, 1.0]),
            None,
        ));
        assert!(scene.add_object(first).is_ok());
        assert!(scene.add_object(second).is_ok());
        assert!(
            scene
                .set_transform(
                    0,
                    must(Transform::identity().translated([10.0, 20.0, 30.0]))
                )
                .is_ok()
        );
        assert!(
            scene
                .set_transform(1, must(Transform::identity().translated([-4.0, 5.0, 1.0])))
                .is_ok()
        );
        scene.clear_selection();
        scene
    }

    #[test]
    fn export_selection_and_visibility_matrix_matches_reference() {
        let directory = temporary_directory("selection");
        let mut scene = scene_with_objects();

        let all_path = directory.join("all.3mf");
        let all_report = export_scene(&scene, &all_path);
        assert!(all_report.is_ok_and(|report| report.objects_exported == 2));
        let all = modeler3d_core::load_3mf(&all_path);
        assert!(all.is_ok_and(|result| {
            result
                .items
                .iter()
                .map(|item| item.name.as_str())
                .collect::<Vec<_>>()
                == ["Blue block", "Red block"]
        }));

        scene.set_selection([1]);
        let one_path = directory.join("one.3mf");
        let one_report = export_scene(&scene, &one_path);
        assert!(one_report.is_ok_and(|report| report.objects_exported == 1));
        let one = modeler3d_core::load_3mf(&one_path);
        assert!(one.is_ok_and(|result| {
            result.items.len() == 1 && result.items[0].name == "Red block"
        }));

        scene.set_selection([0, 1]);
        let multiple_path = directory.join("multiple.3mf");
        let multiple_report = export_scene(&scene, &multiple_path);
        assert!(multiple_report.is_ok_and(|report| report.objects_exported == 2));

        assert!(scene.set_visible(1, false).is_ok());
        let mixed_path = directory.join("mixed.3mf");
        let mixed_report = export_scene(&scene, &mixed_path);
        assert!(mixed_report.is_ok_and(|report| report.objects_exported == 1));
        let mixed = modeler3d_core::load_3mf(&mixed_path);
        assert!(mixed.is_ok_and(|result| {
            result.items.len() == 1 && result.items[0].name == "Blue block"
        }));

        scene.set_selection([1]);
        let hidden_selected_path = directory.join("hidden-selected.3mf");
        assert!(matches!(
            export_scene(&scene, &hidden_selected_path),
            Err(ExportError::NoVisibleObjects)
        ));
        assert!(!hidden_selected_path.exists());

        scene.clear_selection();
        assert!(scene.set_visible(0, false).is_ok());
        let no_visible_path = directory.join("no-visible.3mf");
        assert!(matches!(
            export_scene(&scene, &no_visible_path),
            Err(ExportError::NoVisibleObjects)
        ));
        assert!(!no_visible_path.exists());
        assert!(fs::remove_dir_all(directory).is_ok());
    }

    #[test]
    fn boolean_face_colors_survive_three_mf_export_and_painting() {
        let directory = temporary_directory("boolean-face-colors");
        let target_color = [0.2, 0.4, 0.6, 1.0];
        let tool_color = [0.8, 0.2, 0.1, 1.0];
        let target = must(SceneObject::with_details(
            must(make_box([2.0, 2.0, 2.0])),
            None,
            Some(String::from("Target")),
            Some(target_color),
            None,
        ));
        let tool = must(SceneObject::with_details(
            must(make_box([1.0, 1.0, 1.0])),
            None,
            Some(String::from("Tool")),
            Some(tool_color),
            None,
        ));
        let mut scene = Scene::new();
        assert!(scene.add_object(target).is_ok());
        assert!(scene.add_object(tool).is_ok());
        assert!(
            scene
                .set_transform(1, must(Transform::identity().translated([0.5, 0.0, 0.0])))
                .is_ok()
        );
        let applied = scene.apply_boolean(
            0,
            1,
            BooleanOperation::Difference,
            BooleanOptions::default(),
        );
        assert!(applied.is_ok(), "boolean failed: {applied:?}");

        let provenance = boolean_color_provenance(
            scene.objects()[0].metadata(),
            scene.objects()[0].mesh().face_count(),
        );
        let Some(provenance) = provenance else {
            panic!("boolean face color provenance was not retained");
        };
        assert_eq!(provenance.operation, "subtract");
        assert!(provenance.tool_face_count > 0);
        let expected_face_colors = provenance.face_colors_rgba8();

        let first_path = directory.join("boolean-before-paint.3mf");
        assert!(export_scene(&scene, &first_path).is_ok());
        let first = modeler3d_core::load_3mf(&first_path);
        assert!(first.is_ok(), "3MF could not be reopened: {first:?}");
        let first = first.expect("successful 3MF read");
        assert_eq!(first.items.len(), 1);
        assert_eq!(first.items[0].face_colors, Some(expected_face_colors));

        let painted_color = [0.1, 0.7, 0.3, 1.0];
        assert!(scene.set_color(0, painted_color).is_ok());
        let painted_provenance = boolean_color_provenance(
            scene.objects()[0].metadata(),
            scene.objects()[0].mesh().face_count(),
        );
        let Some(painted_provenance) = painted_provenance else {
            panic!("painting removed valid boolean color provenance");
        };
        assert_eq!(painted_provenance.source_colors[0], painted_color);
        assert_eq!(painted_provenance.source_colors[1], tool_color);
        let second_path = directory.join("boolean-after-paint.3mf");
        assert!(export_scene(&scene, &second_path).is_ok());
        let second = modeler3d_core::load_3mf(&second_path);
        assert!(
            second.is_ok(),
            "painted 3MF could not be reopened: {second:?}"
        );
        let second = second.expect("successful painted 3MF read");
        assert_eq!(
            second.items[0].face_colors,
            Some(painted_provenance.face_colors_rgba8())
        );
        assert!(fs::remove_dir_all(directory).is_ok());
    }

    #[test]
    fn each_reference_export_reopens_with_world_geometry_and_supported_colors() {
        let directory = temporary_directory("formats");
        let scene = scene_with_objects();
        let expected_bounds = ([-4.5, 3.5, -1.5], [11.0, 22.0, 33.0]);
        let colors = [[51, 102, 153, 204], [230, 51, 26, 255]];

        for extension in ["stl", "obj", "ply", "3mf", "gltf", "glb", "wrl"] {
            let path = directory.join(format!("scene.{extension}"));
            let report = export_scene(&scene, &path);
            assert!(
                report
                    .as_ref()
                    .is_ok_and(|report| report.objects_exported == 2),
                "{extension} export failed: {report:?}"
            );
            assert!(path.is_file(), "{extension} output was not written");

            match extension {
                "stl" | "obj" | "ply" => {
                    let asset = modeler3d_core::load_mesh_asset(&path);
                    assert!(
                        asset.is_ok(),
                        "{extension} could not be reopened: {asset:?}"
                    );
                    if let Ok(asset) = asset {
                        assert_bounds(asset.mesh(), expected_bounds);
                        match extension {
                            "stl" => assert_eq!(asset.vertex_colors(), None),
                            "obj" => {
                                let Some(vertex_colors) = asset.vertex_colors() else {
                                    panic!("OBJ output did not retain object colors");
                                };
                                assert_eq!(vertex_colors.len(), 16);
                                assert!(
                                    vertex_colors[..8]
                                        .iter()
                                        .all(|color| *color == [51, 102, 153, 255])
                                );
                                assert!(
                                    vertex_colors[8..]
                                        .iter()
                                        .all(|color| *color == [230, 51, 26, 255])
                                );
                            }
                            "ply" => {
                                let Some(vertex_colors) = asset.vertex_colors() else {
                                    panic!("PLY output did not retain object colors");
                                };
                                assert_eq!(vertex_colors.len(), 16);
                                assert!(vertex_colors[..8].iter().all(|color| *color == colors[0]));
                                assert!(vertex_colors[8..].iter().all(|color| *color == colors[1]));
                            }
                            _ => {}
                        }
                    }
                }
                "3mf" => {
                    let imported = modeler3d_core::load_3mf(&path);
                    assert!(
                        imported.is_ok(),
                        "3MF output could not be reopened: {imported:?}"
                    );
                    if let Ok(imported) = imported {
                        assert_eq!(imported.items.len(), 2);
                        assert_exported_instance(
                            &imported.items[0].name,
                            &imported.items[0].mesh,
                            imported.items[0].transform,
                            imported.items[0].color,
                            BLUE_BLOCK,
                        );
                        assert_exported_instance(
                            &imported.items[1].name,
                            &imported.items[1].mesh,
                            imported.items[1].transform,
                            imported.items[1].color,
                            RED_BLOCK,
                        );
                    }
                }
                "gltf" | "glb" => {
                    let items = modeler3d_core::load_gltf(&path);
                    assert!(
                        items.is_ok(),
                        "{extension} output could not be reopened: {items:?}"
                    );
                    if let Ok(items) = items {
                        assert_eq!(items.len(), 2);
                        let Some(first_color) = items[0].color else {
                            panic!("first {extension} object has no material color");
                        };
                        assert_exported_instance(
                            &items[0].name,
                            &items[0].mesh,
                            items[0].transform,
                            first_color,
                            BLUE_BLOCK,
                        );
                        let Some(second_color) = items[1].color else {
                            panic!("second {extension} object has no material color");
                        };
                        assert_exported_instance(
                            &items[1].name,
                            &items[1].mesh,
                            items[1].transform,
                            second_color,
                            RED_BLOCK,
                        );
                    }
                }
                "wrl" => {
                    let text = fs::read_to_string(&path);
                    assert!(text.is_ok(), "WRL output was not valid UTF-8: {text:?}");
                    if let Ok(text) = text {
                        assert!(text.contains("#VRML V2.0 utf8"));
                        assert!(text.contains("# Object name: \"Blue block\""));
                        assert!(text.contains("# Object name: \"Red block\""));
                        assert_eq!(text.matches("Shape {").count(), 2);
                    }
                }
                _ => unreachable!(),
            }
        }
        assert!(fs::remove_dir_all(directory).is_ok());
    }

    fn assert_exported_instance(
        name: &str,
        mesh: &Mesh,
        transform: Transform,
        color: [f32; 4],
        expected: ExpectedExportedInstance,
    ) {
        assert_eq!(name, expected.name);
        assert_vec3_close(mesh.extents(), expected.extents);
        assert_vec3_close(transform.position(), expected.position);
        assert_color_close(color, expected.color);
        let world_mesh = mesh.transformed(&transform);
        assert!(world_mesh.is_ok());
        if let Ok(world_mesh) = world_mesh {
            let half = expected.extents.map(|value| value / 2.0);
            assert_bounds(
                &world_mesh,
                (
                    [
                        expected.position[0] - half[0],
                        expected.position[1] - half[1],
                        expected.position[2] - half[2],
                    ],
                    [
                        expected.position[0] + half[0],
                        expected.position[1] + half[1],
                        expected.position[2] + half[2],
                    ],
                ),
            );
        }
    }

    #[test]
    fn unsupported_export_extension_is_typed_before_writing() {
        let scene = scene_with_objects();
        let path = std::env::temp_dir().join("modeler3d-r7-export.unsupported");
        let result = export_scene(&scene, &path);
        assert!(matches!(
            result,
            Err(ExportError::UnsupportedExtension(extension)) if extension == "unsupported"
        ));
        assert!(!path.exists());
    }
}
