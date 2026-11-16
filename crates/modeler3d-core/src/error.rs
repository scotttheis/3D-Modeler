use thiserror::Error;

/// Errors returned by the portable 3D Modeler model and project services.
#[derive(Debug, Error)]
pub enum CoreError {
    /// A mesh contains non-finite coordinates or an invalid face index.
    #[error("invalid mesh: {0}")]
    InvalidMesh(String),
    /// A transform is not a finite homogeneous 4x4 matrix.
    #[error("invalid transform: {0}")]
    InvalidTransform(String),
    /// A color is not a finite RGBA value in the inclusive 0..=1 range.
    #[error("invalid color: {0}")]
    InvalidColor(String),
    /// A texture image could not be imported, stored or found.
    #[error("texture problem: {0}")]
    InvalidTexture(String),
    /// A scene object or project field violates its domain contract.
    #[error("invalid object: {0}")]
    InvalidObject(String),
    /// A primitive name or parameter set is not supported.
    #[error("invalid primitive: {0}")]
    InvalidPrimitive(String),
    /// A command could not be applied without changing the scene.
    #[error("command '{command}' failed: {reason}")]
    CommandFailed { command: String, reason: String },
    /// An object index was not present in the scene.
    #[error("object index {0} is out of range")]
    ObjectIndex(usize),
    /// An object ID was not present or was duplicated.
    #[error("object ID error: {0}")]
    ObjectId(String),
    /// A `.3dp` archive is structurally invalid or exceeds a safety limit.
    #[error("project file is invalid: {0}")]
    ProjectFormat(String),
    #[error("mesh file is invalid: {0}")]
    MeshFile(String),
    /// A 3MF package, model part, relationship, or production extension is invalid.
    #[error("3MF file is invalid: {0}")]
    ThreeMf(String),
    /// A glTF JSON document, GLB container, resource, or scene is invalid.
    #[error("glTF file is invalid: {0}")]
    Gltf(String),
    /// A VRML 2.0/WRL document or export request is invalid.
    #[error("WRL file is invalid: {0}")]
    Wrl(String),
    /// A mesh operation input or result violates its geometry contract.
    #[error("mesh operation failed: {0}")]
    MeshOperation(String),
    /// A mesh operation exceeded a caller-supplied resource limit before the
    /// authoritative scene could be changed.
    #[error("mesh operation '{operation}' exceeded its resource limit: {reason}")]
    OperationResourceLimit { operation: String, reason: String },
    /// A long-running mesh operation was cancelled before it committed.
    #[error("mesh operation '{0}' was cancelled")]
    OperationCancelled(String),
    #[error("unsupported mesh format: {0}")]
    UnsupportedMeshFormat(String),
    /// The archive uses a schema newer than this core understands.
    #[error("project schema version {0} is not supported")]
    UnsupportedProjectVersion(u64),
    /// The filesystem could not read or write the project.
    #[error("project I/O failed: {0}")]
    Io(#[from] std::io::Error),
    /// The ZIP container could not be decoded or written.
    #[error("project archive failed: {0}")]
    Archive(String),
    /// A JSON value could not be encoded for a project boundary.
    #[error("project JSON failed: {0}")]
    Json(String),
}

impl CoreError {
    pub(crate) fn archive(error: impl std::fmt::Display) -> Self {
        Self::Archive(error.to_string())
    }

    pub(crate) fn json(error: impl std::fmt::Display) -> Self {
        Self::Json(error.to_string())
    }

    pub(crate) fn mesh_file(error: impl std::fmt::Display) -> Self {
        Self::MeshFile(error.to_string())
    }

    pub(crate) fn three_mf(error: impl std::fmt::Display) -> Self {
        Self::ThreeMf(error.to_string())
    }

    pub(crate) fn gltf(error: impl std::fmt::Display) -> Self {
        Self::Gltf(error.to_string())
    }

    pub(crate) fn wrl(error: impl std::fmt::Display) -> Self {
        Self::Wrl(error.to_string())
    }
}
