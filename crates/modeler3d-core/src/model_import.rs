//! Shared, atomic model import used by desktop and automation.
use crate::{
    AddObjectCommand, Command as CoreCommand, CoreError, GltfItem, GroupedCommand, Mesh,
    MeshFormat, Scene, SceneObject, ThreeMfItem, Transform, TransformObjectCommand, load_3mf,
    load_gltf, load_mesh_asset, load_project,
};
use serde_json::Value;
use std::path::Path;

/// Imports a model into the scene, centres it on the floor and selects the new objects.
/// Unitless STL/OBJ/PLY/WRL use `factor_mm`; other formats retain their declared units.
/// Parsing completes before one undoable commit; errors preserve scene and history.
pub fn import_model(
    scene: &mut Scene,
    path: &Path,
    factor_mm: f32,
) -> Result<Vec<String>, CoreError> {
    prepare_model_import(path, factor_mm)?.commit(scene)
}

/// Fully parsed model data ready for one scene commit.
/// Keeps file reads and validation outside an adapter's scene lock.
pub struct PreparedModelImport {
    imported: Vec<ImportedObject>,
}

/// Reads and validates a model without mutating a live scene.
pub fn prepare_model_import(path: &Path, factor_mm: f32) -> Result<PreparedModelImport, CoreError> {
    if !factor_mm.is_finite() || factor_mm <= 0. {
        return Err(CoreError::MeshFile("invalid import scale".to_owned()));
    }
    let mut imported = load_import_objects(path, factor_mm)?;
    if imported.is_empty() {
        return Err(CoreError::MeshFile(
            "import contains no mesh objects".to_owned(),
        ));
    }
    center_import_objects(&mut imported)?;
    Ok(PreparedModelImport { imported })
}

impl PreparedModelImport {
    /// Adds all prepared objects as one undo entry and returns their fresh IDs.
    pub fn commit(self, scene: &mut Scene) -> Result<Vec<String>, CoreError> {
        let imported = self.imported;
        let start = scene.objects().len();
        let count = imported.len();
        let mut commands: Vec<Box<dyn CoreCommand>> = Vec::with_capacity(count * 2);
        let mut ids = Vec::with_capacity(count);
        for (offset, item) in imported.into_iter().enumerate() {
            let index = start + offset;
            ids.push(item.object.object_id().to_owned());
            commands.push(Box::new(AddObjectCommand::at_index(item.object, index)));
            if item.transform != Transform::identity() {
                commands.push(Box::new(TransformObjectCommand::new(index, item.transform)));
            }
        }
        scene.execute(GroupedCommand::with_after_selection(
            commands,
            (start..start + count).collect(),
        ))?;
        Ok(ids)
    }
}

struct ImportedObject {
    object: SceneObject,
    transform: Transform,
}

fn load_import_objects(path: &Path, factor: f32) -> Result<Vec<ImportedObject>, CoreError> {
    if path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("3dp"))
    {
        return load_project(path)?
            .objects()
            .iter()
            .map(|item| {
                let object = item.duplicate_for_import()?;
                Ok(ImportedObject {
                    transform: *object.transform(),
                    object,
                })
            })
            .collect();
    }
    match MeshFormat::from_path(path)? {
        MeshFormat::Stl | MeshFormat::Obj | MeshFormat::Ply => {
            let asset = load_mesh_asset(path)?;
            let mesh = if factor == 1.0 {
                asset.mesh().clone()
            } else {
                let scale = Transform::identity().scaled([factor; 3])?;
                asset.mesh().transformed(&scale)?
            };
            Ok(vec![imported_object(
                mesh,
                Transform::identity(),
                import_file_name(path),
                asset.display_color(),
                None,
            )?])
        }
        MeshFormat::Gltf | MeshFormat::Glb => {
            load_gltf(path)?.into_iter().map(import_gltf).collect()
        }
        MeshFormat::ThreeMf => load_3mf(path)?
            .items
            .into_iter()
            .map(import_three_mf)
            .collect(),
        MeshFormat::Wrl => crate::load_wrl(path)?
            .into_iter()
            .map(|item| {
                let scale = Transform::identity().scaled([factor; 3])?;
                imported_object(
                    item.mesh,
                    scale.multiply(&item.transform)?,
                    item.name,
                    item.color,
                    None,
                )
            })
            .collect(),
    }
}

fn center_import_objects(items: &mut [ImportedObject]) -> Result<(), CoreError> {
    let mut bounds: Option<([f32; 3], [f32; 3])> = None;
    for item in items.iter() {
        let world_mesh = item.object.mesh().transformed(&item.transform)?;
        let Some((minimum, maximum)) = world_mesh.bounds() else {
            return Err(CoreError::InvalidMesh(
                "imported mesh has no vertex bounds".to_owned(),
            ));
        };
        bounds = Some(match bounds {
            None => (minimum, maximum),
            Some((current_minimum, current_maximum)) => {
                let mut next_minimum = current_minimum;
                let mut next_maximum = current_maximum;
                for axis in 0..3 {
                    next_minimum[axis] = next_minimum[axis].min(minimum[axis]);
                    next_maximum[axis] = next_maximum[axis].max(maximum[axis]);
                }
                (next_minimum, next_maximum)
            }
        });
    }

    let Some((minimum, maximum)) = bounds else {
        return Ok(());
    };
    let placement = Transform::identity().translated([
        -(minimum[0] + maximum[0]) * 0.5,
        -minimum[1],
        -(minimum[2] + maximum[2]) * 0.5,
    ])?;
    for item in items {
        item.transform = placement.multiply(&item.transform)?;
    }
    Ok(())
}

fn import_gltf(item: GltfItem) -> Result<ImportedObject, CoreError> {
    imported_object(item.mesh, item.transform, item.name, item.color, None)
}

fn import_three_mf(item: ThreeMfItem) -> Result<ImportedObject, CoreError> {
    let metadata = if item.metadata.is_empty() {
        None
    } else {
        Some(
            serde_json::to_value(item.metadata)
                .map_err(|error| CoreError::Json(error.to_string()))?,
        )
    };
    imported_object(
        item.mesh,
        item.transform,
        item.name,
        Some(item.color),
        metadata,
    )
}

fn imported_object(
    mesh: Mesh,
    transform: Transform,
    name: String,
    color: Option<[f32; 4]>,
    metadata: Option<Value>,
) -> Result<ImportedObject, CoreError> {
    Ok(ImportedObject {
        object: SceneObject::with_details(mesh, None, Some(name), color, metadata)?,
        transform,
    })
}

fn import_file_name(path: &Path) -> String {
    path.file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.trim().is_empty())
        .map_or_else(|| String::from("Imported"), ToOwned::to_owned)
}
