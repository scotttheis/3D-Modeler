use std::collections::BTreeSet;
use std::fs::{self, OpenOptions};
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::scene::validate_scene_objects;
use crate::{CoreError, Mesh, ObjectId, Scene, SceneChild, SceneObject, Transform};

/// Current Builder3D project archive marker.
pub const PROJECT_FORMAT: &str = "Builder3D";
/// Current project schema.
pub const PROJECT_SCHEMA_VERSION: u64 = 1;
/// Legacy unversioned schema.
pub const LEGACY_PROJECT_SCHEMA_VERSION: u64 = 0;
/// Maximum number of ZIP members.
pub const MAX_PROJECT_OBJECTS: usize = 10_000;
/// Maximum archive members, including scene.json.
pub const MAX_PROJECT_ARCHIVE_ENTRIES: usize = MAX_PROJECT_OBJECTS + 1;
/// Maximum compressed archive bytes.
pub const MAX_PROJECT_ARCHIVE_BYTES: u64 = 512 * 1024 * 1024;
/// Maximum aggregate uncompressed member bytes.
pub const MAX_PROJECT_UNCOMPRESSED_BYTES: u64 = 1024 * 1024 * 1024;
/// Maximum uncompressed bytes for one member.
pub const MAX_PROJECT_MEMBER_BYTES: u64 = 256 * 1024 * 1024;
/// Maximum uncompressed scene.json bytes.
pub const MAX_PROJECT_SCENE_JSON_BYTES: u64 = 8 * 1024 * 1024;
/// Maximum aggregate metadata bytes.
pub const MAX_PROJECT_METADATA_BYTES: usize = 8 * 1024 * 1024;
/// Maximum metadata bytes on one object.
pub const MAX_PROJECT_OBJECT_METADATA_BYTES: usize = 1024 * 1024;
/// Compression-ratio limit for large members.
pub const MAX_PROJECT_COMPRESSION_RATIO: f64 = 1000.0;
/// Minimum member size at which the ratio check is applied.
pub const MAX_PROJECT_COMPRESSION_CHECK_BYTES: u64 = 1024 * 1024;
/// Maximum vertices in one project mesh.
pub const MAX_PROJECT_MESH_VERTICES: usize = 5_000_000;
/// Maximum faces in one project mesh.
pub const MAX_PROJECT_MESH_FACES: usize = 5_000_000;
/// Maximum vertices across a project.
pub const MAX_PROJECT_TOTAL_MESH_VERTICES: usize = 10_000_000;
/// Maximum faces across a project.
pub const MAX_PROJECT_TOTAL_MESH_FACES: usize = 10_000_000;
/// Maximum nested group depth.
pub const MAX_PROJECT_GROUP_DEPTH: usize = 32;

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug)]
struct ValidatedObject {
    object_id: ObjectId,
    name: String,
    transform: Transform,
    color: [f32; 4],
    visible: bool,
    metadata: Value,
    mesh_file: String,
    children: Vec<ValidatedObject>,
}

#[derive(Debug)]
struct ValidatedProject {
    project_id: ObjectId,
    revision: u64,
    objects: Vec<ValidatedObject>,
}

#[derive(Clone, Debug, Serialize)]
struct StoredProject {
    format: String,
    schema_version: u64,
    project_id: String,
    scene_revision: u64,
    objects: Vec<StoredObject>,
}

#[derive(Clone, Debug, Serialize)]
struct StoredObject {
    object_id: String,
    name: String,
    transform: [[f64; 4]; 4],
    color: [f64; 4],
    visible: bool,
    metadata: Value,
    mesh_file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    children: Option<Vec<StoredObject>>,
}

/// Saves a scene atomically to a .3dp path.
pub fn save_project(scene: &Scene, path: impl AsRef<Path>) -> Result<(), CoreError> {
    let target = path.as_ref();
    save_project_with_commit(scene, target, replace_project_file)
}

fn save_project_with_commit<F>(scene: &Scene, target: &Path, commit: F) -> Result<(), CoreError>
where
    F: FnOnce(&Path, &Path) -> std::io::Result<()>,
{
    let bytes = save_project_bytes(scene)?;
    let parent = target.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    let temporary = temporary_path(target, parent)?;
    let write_result = (|| -> Result<(), CoreError> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        drop(file);
        commit(&temporary, target).map_err(CoreError::from)
    })();
    if write_result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    write_result
}

#[cfg(not(windows))]
fn replace_project_file(temporary: &Path, target: &Path) -> std::io::Result<()> {
    fs::rename(temporary, target)
}

#[cfg(windows)]
#[allow(unsafe_code)]
fn replace_project_file(temporary: &Path, target: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;

    use windows::Win32::Storage::FileSystem::{
        MOVEFILE_WRITE_THROUGH, MoveFileExW, REPLACE_FILE_FLAGS, ReplaceFileW,
    };
    use windows::core::PCWSTR;

    let temporary_wide = temporary
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let target_wide = target
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();

    let result = if target.exists() {
        // SAFETY: both pointers refer to owned, NUL-terminated UTF-16 buffers
        // that remain alive for the duration of the synchronous API call.
        unsafe {
            ReplaceFileW(
                PCWSTR(target_wide.as_ptr()),
                PCWSTR(temporary_wide.as_ptr()),
                PCWSTR::null(),
                REPLACE_FILE_FLAGS(0),
                None,
                None,
            )
        }
    } else {
        // SAFETY: both pointers refer to owned, NUL-terminated UTF-16 buffers
        // that remain alive for the duration of the synchronous API call.
        unsafe {
            MoveFileExW(
                PCWSTR(temporary_wide.as_ptr()),
                PCWSTR(target_wide.as_ptr()),
                MOVEFILE_WRITE_THROUGH,
            )
        }
    };

    result.map_err(|error| std::io::Error::other(error.to_string()))
}

/// Serializes a scene to the .3dp byte stream.
///
/// An untouched archive loaded by load_project is returned byte-for-byte,
/// preserving Python-produced ZIP metadata and STL members. A changed scene
/// is emitted with the reference member order and binary-STL contract.
pub fn save_project_bytes(scene: &Scene) -> Result<Vec<u8>, CoreError> {
    if let Some(source) = scene.source_archive() {
        return Ok(source.to_vec());
    }
    validate_scene_for_save(scene)?;
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let mut stored_objects = Vec::with_capacity(scene.objects().len());
    for (index, object) in scene.objects().iter().enumerate() {
        stored_objects.push(write_object(
            &mut writer,
            options,
            object,
            format!("meshes/mesh_{index}"),
        )?);
    }
    let project = StoredProject {
        format: PROJECT_FORMAT.to_owned(),
        schema_version: PROJECT_SCHEMA_VERSION,
        project_id: scene.project_id().to_owned(),
        scene_revision: scene.revision(),
        objects: stored_objects,
    };
    let scene_json = serde_json::to_vec_pretty(&project).map_err(CoreError::json)?;
    writer
        .start_file("scene.json", options)
        .map_err(CoreError::archive)?;
    writer.write_all(&scene_json)?;
    let cursor = writer.finish().map_err(CoreError::archive)?;
    Ok(cursor.into_inner())
}

/// Loads and validates a .3dp file without mutating another scene.
pub fn load_project(path: impl AsRef<Path>) -> Result<Scene, CoreError> {
    let target = path.as_ref();
    let bytes = fs::read(target).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            CoreError::ProjectFormat(format!("project file was not found: {}", target.display()))
        } else {
            CoreError::ProjectFormat(format!(
                "project file could not be read: {}",
                target.display()
            ))
        }
    })?;
    load_project_bytes(&bytes)
}

/// Loads and validates a .3dp byte stream.
pub fn load_project_bytes(bytes: &[u8]) -> Result<Scene, CoreError> {
    let archive_size = u64::try_from(bytes.len())
        .map_err(|_| CoreError::ProjectFormat("project archive is too large".to_owned()))?;
    if archive_size > MAX_PROJECT_ARCHIVE_BYTES {
        return Err(CoreError::ProjectFormat(format!(
            "project archive exceeds the maximum size of {MAX_PROJECT_ARCHIVE_BYTES} bytes"
        )));
    }
    let mut archive = ZipArchive::new(Cursor::new(bytes.to_vec())).map_err(|_| {
        CoreError::ProjectFormat("project file is not a readable ZIP archive".to_owned())
    })?;
    let names = validate_archive_limits(&mut archive)?;
    let scene_json = read_archive_entry(&mut archive, "scene.json")?;
    let root: Value = serde_json::from_slice(&scene_json)
        .map_err(|_| CoreError::ProjectFormat("scene.json is not valid UTF-8 JSON".to_owned()))?;
    let validated = validate_project_data(&root, &names)?;
    let mut mesh_budget = MeshBudget::default();
    let mut scene = Scene::new();
    let mut objects = Vec::with_capacity(validated.objects.len());
    for object in validated.objects {
        objects.push(load_object(&mut archive, object, &mut mesh_budget)?);
    }
    validate_scene_objects(&objects)?;
    scene.set_project_fields(validated.project_id, validated.revision);
    *scene.objects_mut() = objects;
    scene.set_loaded_archive(bytes.to_vec());
    Ok(scene)
}

fn write_object<W: Write + std::io::Seek>(
    writer: &mut ZipWriter<W>,
    options: SimpleFileOptions,
    object: &SceneObject,
    prefix: String,
) -> Result<StoredObject, CoreError> {
    let mesh_file = format!("{prefix}.stl");
    write_mesh(writer, options, &mesh_file, object.mesh())?;
    let mut children = Vec::with_capacity(object.children().len());
    for (index, child) in object.children().iter().enumerate() {
        children.push(write_child(
            writer,
            options,
            child,
            format!("{prefix}_child_{index}"),
        )?);
    }
    Ok(StoredObject {
        object_id: object.object_id().to_owned(),
        name: object.name().to_owned(),
        transform: object.transform().f64_rows(),
        color: object.color().map(f64::from),
        visible: object.is_visible(),
        metadata: object.metadata().clone(),
        mesh_file,
        children: (!children.is_empty()).then_some(children),
    })
}

fn write_child<W: Write + std::io::Seek>(
    writer: &mut ZipWriter<W>,
    options: SimpleFileOptions,
    child: &SceneChild,
    prefix: String,
) -> Result<StoredObject, CoreError> {
    let mesh_file = format!("{prefix}.stl");
    write_mesh(writer, options, &mesh_file, child.mesh())?;
    let mut children = Vec::with_capacity(child.children().len());
    for (index, nested) in child.children().iter().enumerate() {
        children.push(write_child(
            writer,
            options,
            nested,
            format!("{prefix}_child_{index}"),
        )?);
    }
    Ok(StoredObject {
        object_id: child.object_id().to_owned(),
        name: child.name().to_owned(),
        transform: child.transform().f64_rows(),
        color: child.color().map(f64::from),
        visible: child.is_visible(),
        metadata: child.metadata().clone(),
        mesh_file,
        children: (!children.is_empty()).then_some(children),
    })
}

fn write_mesh<W: Write + std::io::Seek>(
    writer: &mut ZipWriter<W>,
    options: SimpleFileOptions,
    name: &str,
    mesh: &Mesh,
) -> Result<(), CoreError> {
    let bytes = mesh.to_binary_stl()?;
    writer
        .start_file(name, options)
        .map_err(CoreError::archive)?;
    writer.write_all(&bytes)?;
    Ok(())
}

fn load_object<R: Read + std::io::Seek>(
    archive: &mut ZipArchive<R>,
    object: ValidatedObject,
    budget: &mut MeshBudget,
) -> Result<SceneObject, CoreError> {
    let mesh = read_mesh(archive, &object.mesh_file, &object.name, budget)?;
    let children = object
        .children
        .into_iter()
        .map(|child| load_child(archive, child, budget))
        .collect::<Result<Vec<_>, _>>()?;
    SceneObject::from_parts(
        mesh,
        object.object_id,
        object.name,
        object.transform,
        object.color,
        object.visible,
        object.metadata,
        children,
    )
}

fn load_child<R: Read + std::io::Seek>(
    archive: &mut ZipArchive<R>,
    child: ValidatedObject,
    budget: &mut MeshBudget,
) -> Result<SceneChild, CoreError> {
    let mesh = read_mesh(archive, &child.mesh_file, &child.name, budget)?;
    let children = child
        .children
        .into_iter()
        .map(|nested| load_child(archive, nested, budget))
        .collect::<Result<Vec<_>, _>>()?;
    SceneChild::from_parts(
        mesh,
        child.object_id,
        child.name,
        child.transform,
        child.color,
        child.visible,
        child.metadata,
        children,
    )
}

fn read_mesh<R: Read + std::io::Seek>(
    archive: &mut ZipArchive<R>,
    mesh_file: &str,
    label: &str,
    budget: &mut MeshBudget,
) -> Result<Mesh, CoreError> {
    let bytes = read_archive_entry(archive, mesh_file).map_err(|error| match error {
        CoreError::ProjectFormat(message) if message.contains("missing ZIP entry") => {
            CoreError::ProjectFormat(format!("{label} file is missing: {mesh_file}"))
        }
        other => other,
    })?;
    if bytes.is_empty() {
        return Err(CoreError::ProjectFormat(format!(
            "{label} file is empty: {mesh_file}"
        )));
    }
    let mesh = Mesh::from_binary_stl(&bytes).map_err(|error| {
        CoreError::ProjectFormat(format!(
            "{label} file could not be read: {mesh_file}: {error}"
        ))
    })?;
    if mesh.vertex_count() == 0 || mesh.face_count() == 0 {
        return Err(CoreError::ProjectFormat(format!(
            "{label} file contains an empty mesh: {mesh_file}"
        )));
    }
    if mesh.vertex_count() > MAX_PROJECT_MESH_VERTICES {
        return Err(CoreError::ProjectFormat(format!(
            "{label} contains more than {MAX_PROJECT_MESH_VERTICES} vertices: {mesh_file}"
        )));
    }
    if mesh.face_count() > MAX_PROJECT_MESH_FACES {
        return Err(CoreError::ProjectFormat(format!(
            "{label} contains more than {MAX_PROJECT_MESH_FACES} faces: {mesh_file}"
        )));
    }
    budget.vertices = budget.vertices.saturating_add(mesh.vertex_count());
    budget.faces = budget.faces.saturating_add(mesh.face_count());
    if budget.vertices > MAX_PROJECT_TOTAL_MESH_VERTICES {
        return Err(CoreError::ProjectFormat(format!(
            "project meshes exceed the maximum total of {MAX_PROJECT_TOTAL_MESH_VERTICES} vertices"
        )));
    }
    if budget.faces > MAX_PROJECT_TOTAL_MESH_FACES {
        return Err(CoreError::ProjectFormat(format!(
            "project meshes exceed the maximum total of {MAX_PROJECT_TOTAL_MESH_FACES} faces"
        )));
    }
    Ok(mesh)
}

fn validate_archive_limits<R: Read + std::io::Seek>(
    archive: &mut ZipArchive<R>,
) -> Result<BTreeSet<String>, CoreError> {
    if archive.len() > MAX_PROJECT_ARCHIVE_ENTRIES {
        return Err(CoreError::ProjectFormat(format!(
            "project archive contains more than {MAX_PROJECT_ARCHIVE_ENTRIES} entries"
        )));
    }
    let mut names = BTreeSet::new();
    let mut total_uncompressed = 0_u64;
    for index in 0..archive.len() {
        let file = archive.by_index(index).map_err(CoreError::archive)?;
        let name = file.name().map_err(CoreError::archive)?.into_owned();
        if !names.insert(name.clone()) {
            return Err(CoreError::ProjectFormat(
                "the ZIP archive contains duplicate entries".to_owned(),
            ));
        }
        let size = file.size();
        let compressed_size = file.compressed_size();
        if size > MAX_PROJECT_MEMBER_BYTES {
            return Err(CoreError::ProjectFormat(format!(
                "ZIP entry exceeds the maximum size of {MAX_PROJECT_MEMBER_BYTES} bytes: {name}"
            )));
        }
        total_uncompressed = total_uncompressed.saturating_add(size);
        if total_uncompressed > MAX_PROJECT_UNCOMPRESSED_BYTES {
            return Err(CoreError::ProjectFormat(format!(
                "project archive exceeds the maximum uncompressed size of {MAX_PROJECT_UNCOMPRESSED_BYTES} bytes"
            )));
        }
        if size >= MAX_PROJECT_COMPRESSION_CHECK_BYTES
            && (size as f64) / (compressed_size.max(1) as f64) > MAX_PROJECT_COMPRESSION_RATIO
        {
            return Err(CoreError::ProjectFormat(format!(
                "ZIP entry has a suspicious compression ratio: {name}"
            )));
        }
    }
    if !names.contains("scene.json") {
        return Err(CoreError::ProjectFormat(
            "the ZIP archive is missing scene.json".to_owned(),
        ));
    }
    let scene_size = archive
        .by_name("scene.json")
        .map_err(CoreError::archive)?
        .size();
    if scene_size > MAX_PROJECT_SCENE_JSON_BYTES {
        return Err(CoreError::ProjectFormat(format!(
            "scene.json exceeds the maximum size of {MAX_PROJECT_SCENE_JSON_BYTES} bytes"
        )));
    }
    Ok(names)
}

fn read_archive_entry<R: Read + std::io::Seek>(
    archive: &mut ZipArchive<R>,
    name: &str,
) -> Result<Vec<u8>, CoreError> {
    let mut file = archive
        .by_name(name)
        .map_err(|_| CoreError::ProjectFormat(format!("missing ZIP entry: {name}")))?;
    if file.size() > MAX_PROJECT_MEMBER_BYTES {
        return Err(CoreError::ProjectFormat(format!(
            "ZIP entry exceeds the maximum size of {MAX_PROJECT_MEMBER_BYTES} bytes: {name}"
        )));
    }
    let capacity = usize::try_from(file.size())
        .map_err(|_| CoreError::ProjectFormat(format!("ZIP entry is too large to load: {name}")))?;
    let mut bytes = Vec::with_capacity(capacity);
    file.read_to_end(&mut bytes)?;
    Ok(bytes)
}

fn validate_project_data(
    root: &Value,
    zip_names: &BTreeSet<String>,
) -> Result<ValidatedProject, CoreError> {
    let object = root.as_object().ok_or_else(|| {
        CoreError::ProjectFormat("scene.json must contain a JSON object".to_owned())
    })?;
    let schema_version = match object.get("schema_version") {
        None => LEGACY_PROJECT_SCHEMA_VERSION,
        Some(value) => value.as_u64().ok_or_else(|| {
            CoreError::ProjectFormat("schema_version must be an integer".to_owned())
        })?,
    };
    if schema_version > PROJECT_SCHEMA_VERSION {
        return Err(CoreError::UnsupportedProjectVersion(schema_version));
    }
    let format = object.get("format").and_then(Value::as_str);
    if schema_version == PROJECT_SCHEMA_VERSION && format != Some(PROJECT_FORMAT) {
        return Err(CoreError::ProjectFormat(
            "scene.json has no valid Builder3D format identifier".to_owned(),
        ));
    }
    if let Some(format) = object.get("format")
        && format.as_str() != Some(PROJECT_FORMAT)
    {
        return Err(CoreError::ProjectFormat(format!(
            "unsupported project format {format:?}"
        )));
    }
    let project_id = match object.get("project_id") {
        Some(Value::String(value)) if !value.trim().is_empty() => value.clone(),
        None if schema_version == LEGACY_PROJECT_SCHEMA_VERSION => {
            Uuid::new_v4().simple().to_string()
        }
        _ => {
            return Err(CoreError::ProjectFormat(
                "project_id must be a non-empty string".to_owned(),
            ));
        }
    };
    let revision = match object.get("scene_revision") {
        Some(value) => value.as_u64().ok_or_else(|| {
            CoreError::ProjectFormat("scene_revision must be a non-negative integer".to_owned())
        })?,
        None if schema_version == LEGACY_PROJECT_SCHEMA_VERSION => 0,
        None => {
            return Err(CoreError::ProjectFormat(
                "scene_revision is required for schema version 1".to_owned(),
            ));
        }
    };
    let objects = object
        .get("objects")
        .and_then(Value::as_array)
        .ok_or_else(|| CoreError::ProjectFormat("objects must be a list".to_owned()))?;
    let mut object_ids = BTreeSet::new();
    let mut mesh_names = BTreeSet::new();
    let mut object_count = 0_usize;
    let mut metadata_bytes = 0_usize;
    let mut validated_objects = Vec::with_capacity(objects.len());
    for (index, value) in objects.iter().enumerate() {
        validated_objects.push(validate_object(
            value,
            &format!("objects[{index}]"),
            schema_version,
            zip_names,
            &mut mesh_names,
            &mut object_ids,
            &mut object_count,
            0,
            &mut metadata_bytes,
        )?);
    }
    let unexpected = zip_names
        .iter()
        .filter(|name| *name != "scene.json" && !mesh_names.contains(*name))
        .cloned()
        .collect::<Vec<_>>();
    if !unexpected.is_empty() {
        return Err(CoreError::ProjectFormat(format!(
            "the ZIP archive contains unexpected entries: {}",
            unexpected
                .iter()
                .take(5)
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        )));
    }
    Ok(ValidatedProject {
        project_id,
        revision,
        objects: validated_objects,
    })
}

#[allow(clippy::too_many_arguments)]
fn validate_object(
    value: &Value,
    label: &str,
    schema_version: u64,
    zip_names: &BTreeSet<String>,
    mesh_names: &mut BTreeSet<String>,
    object_ids: &mut BTreeSet<String>,
    object_count: &mut usize,
    depth: usize,
    metadata_bytes: &mut usize,
) -> Result<ValidatedObject, CoreError> {
    let object = value
        .as_object()
        .ok_or_else(|| CoreError::ProjectFormat(format!("{label} must be an object")))?;
    if depth > MAX_PROJECT_GROUP_DEPTH {
        return Err(CoreError::ProjectFormat(format!(
            "{label} exceeds the maximum group nesting depth of {MAX_PROJECT_GROUP_DEPTH}"
        )));
    }
    if *object_count >= MAX_PROJECT_OBJECTS {
        return Err(CoreError::ProjectFormat(format!(
            "project contains more than {MAX_PROJECT_OBJECTS} objects"
        )));
    }
    *object_count += 1;
    if schema_version == PROJECT_SCHEMA_VERSION {
        let required = [
            "object_id",
            "name",
            "transform",
            "color",
            "visible",
            "metadata",
            "mesh_file",
        ];
        let missing = required
            .iter()
            .filter(|field| !object.contains_key(**field))
            .copied()
            .collect::<Vec<_>>();
        if !missing.is_empty() {
            return Err(CoreError::ProjectFormat(format!(
                "{label} is missing required fields: {}",
                missing.join(", ")
            )));
        }
    }
    let object_id = match object.get("object_id") {
        Some(Value::String(value)) if !value.trim().is_empty() => value.clone(),
        None if schema_version == LEGACY_PROJECT_SCHEMA_VERSION => {
            Uuid::new_v4().simple().to_string()
        }
        _ => {
            return Err(CoreError::ProjectFormat(format!(
                "{label}.object_id must be a non-empty string"
            )));
        }
    };
    if !object_ids.insert(object_id.clone()) {
        return Err(CoreError::ProjectFormat(format!(
            "duplicate object_id {object_id:?}"
        )));
    }
    let name = match object.get("name") {
        Some(Value::String(value)) if !value.trim().is_empty() => value.clone(),
        None if schema_version == LEGACY_PROJECT_SCHEMA_VERSION => label.to_owned(),
        _ => {
            return Err(CoreError::ProjectFormat(format!(
                "{label}.name must be a non-empty string"
            )));
        }
    };
    let transform = parse_transform(object.get("transform"), label, schema_version)?;
    let color = parse_color(object.get("color"), label, schema_version)?;
    let visible = match object.get("visible") {
        Some(Value::Bool(value)) => *value,
        None => true,
        _ => {
            return Err(CoreError::ProjectFormat(format!(
                "{label}.visible must be true or false"
            )));
        }
    };
    let metadata = match object.get("metadata") {
        Some(value) if value.is_object() => value.clone(),
        None => Value::Object(serde_json::Map::new()),
        _ => {
            return Err(CoreError::ProjectFormat(format!(
                "{label}.metadata must be an object"
            )));
        }
    };
    let serialized_metadata = serde_json::to_vec(&metadata).map_err(CoreError::json)?;
    if serialized_metadata.len() > MAX_PROJECT_OBJECT_METADATA_BYTES {
        return Err(CoreError::ProjectFormat(format!(
            "{label}.metadata exceeds the maximum size of {MAX_PROJECT_OBJECT_METADATA_BYTES} bytes"
        )));
    }
    *metadata_bytes = metadata_bytes.saturating_add(serialized_metadata.len());
    if *metadata_bytes > MAX_PROJECT_METADATA_BYTES {
        return Err(CoreError::ProjectFormat(format!(
            "project metadata exceeds the maximum size of {MAX_PROJECT_METADATA_BYTES} bytes"
        )));
    }
    let mesh_file = validate_mesh_name(object.get("mesh_file"), label, zip_names, mesh_names)?;
    let children = match object.get("children") {
        None | Some(Value::Null) => Vec::new(),
        Some(Value::Array(children)) => children
            .iter()
            .enumerate()
            .map(|(index, child)| {
                validate_object(
                    child,
                    &format!("{label}.children[{index}]"),
                    schema_version,
                    zip_names,
                    mesh_names,
                    object_ids,
                    object_count,
                    depth + 1,
                    metadata_bytes,
                )
            })
            .collect::<Result<Vec<_>, _>>()?,
        _ => {
            return Err(CoreError::ProjectFormat(format!(
                "{label}.children must be a list"
            )));
        }
    };
    Ok(ValidatedObject {
        object_id,
        name,
        transform,
        color,
        visible,
        metadata,
        mesh_file,
        children,
    })
}

fn parse_transform(
    value: Option<&Value>,
    label: &str,
    schema_version: u64,
) -> Result<Transform, CoreError> {
    let Some(value) = value else {
        if schema_version == LEGACY_PROJECT_SCHEMA_VERSION {
            return Ok(Transform::identity());
        }
        return Err(CoreError::ProjectFormat(format!(
            "{label}.transform must be a 4x4 numeric matrix"
        )));
    };
    let rows = value.as_array().ok_or_else(|| {
        CoreError::ProjectFormat(format!("{label}.transform must be a 4x4 numeric matrix"))
    })?;
    if rows.len() != 4 {
        return Err(CoreError::ProjectFormat(format!(
            "{label}.transform must be a 4x4 numeric matrix"
        )));
    }
    let mut parsed = [[0.0_f64; 4]; 4];
    for (row_index, row) in rows.iter().enumerate() {
        let values = row.as_array().ok_or_else(|| {
            CoreError::ProjectFormat(format!("{label}.transform must be a 4x4 numeric matrix"))
        })?;
        if values.len() != 4 {
            return Err(CoreError::ProjectFormat(format!(
                "{label}.transform must be a 4x4 numeric matrix"
            )));
        }
        for (column_index, number) in values.iter().enumerate() {
            parsed[row_index][column_index] = number.as_f64().ok_or_else(|| {
                CoreError::ProjectFormat(format!("{label}.transform must be a 4x4 numeric matrix"))
            })?;
        }
    }
    Transform::from_f64_rows(parsed)
        .map_err(|error| CoreError::ProjectFormat(format!("{label}.transform {error}")))
}

fn parse_color(
    value: Option<&Value>,
    label: &str,
    schema_version: u64,
) -> Result<[f32; 4], CoreError> {
    let default = crate::DEFAULT_MESH_COLOR;
    let Some(value) = value else {
        if schema_version == LEGACY_PROJECT_SCHEMA_VERSION {
            return Ok(default);
        }
        return Err(CoreError::ProjectFormat(format!(
            "{label}.color must contain four numeric values"
        )));
    };
    if value.is_null() && schema_version == LEGACY_PROJECT_SCHEMA_VERSION {
        return Ok(default);
    }
    let values = value.as_array().ok_or_else(|| {
        CoreError::ProjectFormat(format!(
            "{label}.color must contain three or four numeric values"
        ))
    })?;
    let valid_length = if schema_version == LEGACY_PROJECT_SCHEMA_VERSION {
        values.len() == 3 || values.len() == 4
    } else {
        values.len() == 4
    };
    if !valid_length {
        return Err(CoreError::ProjectFormat(format!(
            "{label}.color must contain three or four numeric values"
        )));
    }
    let mut result = [1.0_f32; 4];
    for (index, value) in values.iter().enumerate() {
        let number = value.as_f64().ok_or_else(|| {
            CoreError::ProjectFormat(format!(
                "{label}.color must contain three or four numeric values"
            ))
        })?;
        if !number.is_finite()
            || !(0.0..=1.0).contains(&number)
            || number < f64::from(f32::MIN)
            || number > f64::from(f32::MAX)
        {
            return Err(CoreError::ProjectFormat(format!(
                "{label}.color values must be between 0 and 1"
            )));
        }
        result[index] = number as f32;
    }
    Ok(result)
}

fn validate_mesh_name(
    value: Option<&Value>,
    label: &str,
    zip_names: &BTreeSet<String>,
    mesh_names: &mut BTreeSet<String>,
) -> Result<String, CoreError> {
    let value = value.and_then(Value::as_str).ok_or_else(|| {
        CoreError::ProjectFormat(format!("{label}.mesh_file must be a relative ZIP path"))
    })?;
    let normalized = value.replace('\\', "/");
    let parts = normalized.split('/').collect::<Vec<_>>();
    let drive_path = normalized.as_bytes().get(1).is_some_and(|colon| {
        normalized
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphabetic)
            && *colon == b':'
    });
    if normalized.trim().is_empty()
        || normalized.starts_with('/')
        || drive_path
        || parts
            .iter()
            .any(|part| part.is_empty() || *part == "." || *part == "..")
        || parts
            .last()
            .is_some_and(|part| part.eq_ignore_ascii_case("scene.json"))
    {
        return Err(CoreError::ProjectFormat(format!(
            "{label}.mesh_file is not a safe relative ZIP path"
        )));
    }
    if !zip_names.contains(&normalized) {
        return Err(CoreError::ProjectFormat(format!(
            "{label}.mesh_file is missing from the project archive: {normalized}"
        )));
    }
    if !mesh_names.insert(normalized.clone()) {
        return Err(CoreError::ProjectFormat(format!(
            "{label}.mesh_file is referenced more than once: {normalized}"
        )));
    }
    Ok(normalized)
}

fn validate_scene_for_save(scene: &Scene) -> Result<(), CoreError> {
    validate_scene_objects(scene.objects())?;
    if scene.objects().len() > MAX_PROJECT_OBJECTS {
        return Err(CoreError::ProjectFormat(format!(
            "project contains more than {MAX_PROJECT_OBJECTS} objects"
        )));
    }
    let mut budget = MeshBudget::default();
    let mut metadata_bytes = 0_usize;
    let mut object_count = 0_usize;
    for object in scene.objects() {
        validate_save_object(
            object,
            &mut budget,
            &mut metadata_bytes,
            &mut object_count,
            0,
        )?;
    }
    Ok(())
}

fn validate_save_object(
    object: &SceneObject,
    budget: &mut MeshBudget,
    metadata_bytes: &mut usize,
    object_count: &mut usize,
    depth: usize,
) -> Result<(), CoreError> {
    account_object(object_count, depth)?;
    if depth > MAX_PROJECT_GROUP_DEPTH {
        return Err(CoreError::ProjectFormat(format!(
            "object exceeds the maximum group nesting depth of {MAX_PROJECT_GROUP_DEPTH}"
        )));
    }
    object.mesh().validate_non_empty()?;
    account_mesh(object.mesh(), budget)?;
    account_metadata(object.metadata(), metadata_bytes)?;
    for child in object.children() {
        account_object(object_count, depth + 1)?;
        child.mesh().validate_non_empty()?;
        account_mesh(child.mesh(), budget)?;
        account_metadata(child.metadata(), metadata_bytes)?;
        validate_save_child(child, budget, metadata_bytes, object_count, depth + 1)?;
    }
    Ok(())
}

fn validate_save_child(
    child: &SceneChild,
    budget: &mut MeshBudget,
    metadata_bytes: &mut usize,
    object_count: &mut usize,
    depth: usize,
) -> Result<(), CoreError> {
    if depth > MAX_PROJECT_GROUP_DEPTH {
        return Err(CoreError::ProjectFormat(format!(
            "object exceeds the maximum group nesting depth of {MAX_PROJECT_GROUP_DEPTH}"
        )));
    }
    for nested in child.children() {
        account_object(object_count, depth + 1)?;
        nested.mesh().validate_non_empty()?;
        account_mesh(nested.mesh(), budget)?;
        account_metadata(nested.metadata(), metadata_bytes)?;
        validate_save_child(nested, budget, metadata_bytes, object_count, depth + 1)?;
    }
    Ok(())
}

fn account_object(object_count: &mut usize, depth: usize) -> Result<(), CoreError> {
    if *object_count >= MAX_PROJECT_OBJECTS {
        return Err(CoreError::ProjectFormat(format!(
            "project contains more than {MAX_PROJECT_OBJECTS} objects"
        )));
    }
    *object_count += 1;
    if depth > MAX_PROJECT_GROUP_DEPTH {
        return Err(CoreError::ProjectFormat(format!(
            "object exceeds the maximum group nesting depth of {MAX_PROJECT_GROUP_DEPTH}"
        )));
    }
    Ok(())
}

fn account_mesh(mesh: &Mesh, budget: &mut MeshBudget) -> Result<(), CoreError> {
    if mesh.vertex_count() > MAX_PROJECT_MESH_VERTICES {
        return Err(CoreError::ProjectFormat(format!(
            "mesh contains more than {MAX_PROJECT_MESH_VERTICES} vertices"
        )));
    }
    if mesh.face_count() > MAX_PROJECT_MESH_FACES {
        return Err(CoreError::ProjectFormat(format!(
            "mesh contains more than {MAX_PROJECT_MESH_FACES} faces"
        )));
    }
    budget.vertices = budget.vertices.saturating_add(mesh.vertex_count());
    budget.faces = budget.faces.saturating_add(mesh.face_count());
    if budget.vertices > MAX_PROJECT_TOTAL_MESH_VERTICES
        || budget.faces > MAX_PROJECT_TOTAL_MESH_FACES
    {
        return Err(CoreError::ProjectFormat(
            "project meshes exceed their maximum total complexity".to_owned(),
        ));
    }
    Ok(())
}

fn account_metadata(metadata: &Value, total: &mut usize) -> Result<(), CoreError> {
    let bytes = serde_json::to_vec(metadata).map_err(CoreError::json)?;
    if bytes.len() > MAX_PROJECT_OBJECT_METADATA_BYTES {
        return Err(CoreError::ProjectFormat(format!(
            "object metadata exceeds the maximum size of {MAX_PROJECT_OBJECT_METADATA_BYTES} bytes"
        )));
    }
    *total = total.saturating_add(bytes.len());
    if *total > MAX_PROJECT_METADATA_BYTES {
        return Err(CoreError::ProjectFormat(format!(
            "project metadata exceeds the maximum size of {MAX_PROJECT_METADATA_BYTES} bytes"
        )));
    }
    Ok(())
}

#[derive(Default)]
struct MeshBudget {
    vertices: usize,
    faces: usize,
}

fn temporary_path(target: &Path, parent: &Path) -> Result<PathBuf, CoreError> {
    let name = target
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| CoreError::ProjectFormat("project path has no file name".to_owned()))?;
    let process = std::process::id();
    let seed = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    for offset in 0..32_u64 {
        let candidate = parent.join(format!(
            ".{name}.{process}.{}.tmp",
            seed.saturating_add(offset)
        ));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    Err(CoreError::ProjectFormat(
        "could not allocate a temporary project path".to_owned(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_atomic_replace_keeps_the_previous_valid_project() {
        let directory = std::env::temp_dir().join(format!(
            "builder3d-project-{}-{}",
            std::process::id(),
            Uuid::new_v4()
        ));
        let target = directory.join("scene.3dp");
        assert!(save_project(&Scene::new(), &target).is_ok());
        let previous_bytes = fs::read(&target).ok();
        assert!(previous_bytes.is_some());

        let result = save_project_with_commit(&Scene::new(), &target, |_temporary, _target| {
            Err(std::io::Error::other("simulated atomic replace failure"))
        });

        assert!(matches!(result, Err(CoreError::Io(_))));
        assert_eq!(fs::read(&target).ok(), previous_bytes);
        assert!(load_project(&target).is_ok());
        assert!(fs::remove_dir_all(directory).is_ok());
    }

    #[test]
    fn successful_save_replaces_an_existing_project_file() {
        let directory = std::env::temp_dir().join(format!(
            "builder3d-project-replace-{}-{}",
            std::process::id(),
            Uuid::new_v4()
        ));
        let target = directory.join("scene.3dp");
        let first = Scene::new();
        assert!(save_project(&first, &target).is_ok());
        let first_bytes = fs::read(&target).ok();
        assert!(first_bytes.is_some());

        let second = Scene::new();
        assert!(save_project(&second, &target).is_ok());
        let second_bytes = fs::read(&target).ok();
        assert!(second_bytes.is_some());
        assert_ne!(first_bytes, second_bytes);
        assert!(load_project(&target).is_ok());
        assert!(fs::remove_dir_all(directory).is_ok());
    }
}
