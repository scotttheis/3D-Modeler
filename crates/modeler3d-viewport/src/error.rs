use std::path::PathBuf;

use modeler3d_platform::LedgerError;

/// Errors crossing the viewport's geometry, resource, and graphics seams.
#[derive(Debug, thiserror::Error)]
pub enum ViewportError {
    /// A caller supplied a non-finite, singular, or otherwise invalid value.
    #[error("invalid viewport input: {0}")]
    InvalidInput(String),
    /// A bounded viewport collection would exceed its configured limit.
    #[error("viewport capacity exceeded: {0}")]
    CapacityExceeded(String),
    /// The portable core rejected an authoritative scene operation.
    #[error("core scene error: {0}")]
    Core(#[from] modeler3d_core::CoreError),
    /// The central resource ledger rejected a reservation.
    #[error("resource ledger error: {0}")]
    Ledger(#[from] LedgerError),
    /// No usable adapter matched the viewport policy.
    #[error("no usable graphics adapter is available")]
    NoAdapter,
    /// The graphics device could not be created.
    #[error("graphics device creation failed: {0}")]
    Device(String),
    /// A graphics readback or command boundary failed.
    #[error("graphics operation failed: {0}")]
    Graphics(String),
    /// A bounded readback wait expired.
    #[error("graphics readback timed out")]
    ReadbackTimeout,
    /// A capture or evidence file could not be written.
    #[error("viewport I/O failed for {path}: {source}", path = path.display())]
    Io {
        /// The affected path.
        path: PathBuf,
        /// The operating-system error.
        #[source]
        source: std::io::Error,
    },
    /// The requested renderer path is not available on this device.
    #[error("viewport feature is unavailable: {0}")]
    Unavailable(String),
}

impl ViewportError {
    pub(crate) fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }
}
