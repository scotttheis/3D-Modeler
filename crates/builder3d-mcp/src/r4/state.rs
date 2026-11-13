//! Mutable, adapter-only MCP state.  Core scene data remains owned by
//! `builder3d_core`; this module contains launch policy, view preferences,
//! replay records, and bounded operation bookkeeping.

use std::{
    collections::VecDeque,
    fs,
    path::PathBuf,
    sync::{Arc, atomic::AtomicBool},
    time::{Duration, Instant},
};

use builder3d_core::{Scene, SceneObject};
use serde_json::{Map, Value, json};
use uuid::Uuid;

/// Hard limit on retained replay responses per server process.
pub const MAX_REPLAY_RECORDS: usize = 256;
/// Replay retention window from the Python gateway contract.
pub const REPLAY_TTL: Duration = Duration::from_secs(600);
/// Hard limit on retained operation records.
pub const MAX_OPERATION_RECORDS: usize = 128;
/// Retention window for terminal operation records.
pub const OPERATION_TTL: Duration = Duration::from_secs(3600);
/// Maximum serialized MCP argument payload.
pub const MAX_ARGUMENT_BYTES: usize = 4 * 1024 * 1024;
/// Maximum serialized structured result payload returned by this adapter.
pub const MAX_RESULT_BYTES: usize = 4 * 1024 * 1024;
/// Maximum number of top-level objects retained by the portable clipboard.
pub const MAX_CLIPBOARD_OBJECTS: usize = 64;
/// Maximum estimated mesh bytes retained by the portable clipboard.
pub const MAX_CLIPBOARD_BYTES: usize = 128 * 1024 * 1024;
/// Maximum number of temporary print-staging directories retained by one
/// server process. Older owned staging directories are removed as new ones
/// are created.
pub const MAX_STAGING_DIRECTORIES: usize = 8;

/// Adapter-owned camera/view state.  It never crosses into core geometry.
#[derive(Clone, Debug)]
pub struct ViewState {
    /// Horizontal camera angle in degrees.
    pub yaw_deg: f32,
    /// Vertical camera angle in degrees.
    pub pitch_deg: f32,
    /// Camera distance in millimetres.
    pub distance_mm: f32,
    /// Camera target in millimetres.
    pub center_mm: [f32; 3],
    /// Whether the desktop grid should be shown.
    pub show_grid: bool,
    /// Whether the desktop viewport uses wireframe rendering.
    pub wireframe: bool,
    /// Whether the desktop viewport uses x-ray rendering.
    pub xray: bool,
}

impl Default for ViewState {
    fn default() -> Self {
        Self {
            yaw_deg: 45.0,
            pitch_deg: 25.0,
            distance_mm: 250.0,
            center_mm: [0.0; 3],
            show_grid: true,
            wireframe: false,
            xray: false,
        }
    }
}

impl ViewState {
    /// Converts the view state to the stable Python-compatible object shape.
    #[must_use]
    pub fn to_value(&self) -> Value {
        json!({
            "yaw_deg": self.yaw_deg,
            "pitch_deg": self.pitch_deg,
            "distance_mm": self.distance_mm,
            "center_mm": self.center_mm,
            "show_grid": self.show_grid,
            "wireframe": self.wireframe,
            "xray": self.xray,
        })
    }
}

/// Stable state names for the product operation manager.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OperationPhase {
    Queued,
    Running,
    Committing,
    Cancelling,
    Completed,
    Failed,
    Cancelled,
}

impl OperationPhase {
    /// Returns the wire spelling used by operation_status.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Committing => "committing",
            Self::Cancelling => "cancelling",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    /// Returns whether no further state transition is expected.
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}

/// A retained operation record.  The cancellation flag is deliberately
/// separate from the scene lock so cancellation remains cooperative.
#[derive(Clone, Debug)]
pub struct OperationRecord {
    /// Stable operation identifier.
    pub id: String,
    /// Catalog action that owns the operation.
    pub action: String,
    /// Current lifecycle state.
    pub phase: OperationPhase,
    /// Progress in the inclusive 0..=100 range.
    pub progress: u8,
    /// Human-readable progress message.
    pub message: String,
    /// Structured result after completion.
    pub result: Option<Value>,
    /// Stable structured error after failure.
    pub error: Option<Value>,
    /// Cooperative cancellation signal.
    pub cancel: Arc<AtomicBool>,
    /// Last state update time.
    pub updated: Instant,
}

impl OperationRecord {
    /// Creates a queued record with a fresh UUID.
    #[must_use]
    pub fn new(action: &str) -> Self {
        let now = Instant::now();
        Self {
            id: Uuid::new_v4().simple().to_string(),
            action: action.to_owned(),
            phase: OperationPhase::Queued,
            progress: 0,
            message: "queued".to_owned(),
            result: None,
            error: None,
            cancel: Arc::new(AtomicBool::new(false)),
            updated: now,
        }
    }

    /// Returns the bounded status object exposed to MCP clients.
    #[must_use]
    pub fn status_value(&self) -> Value {
        json!({
            "operation_id": self.id,
            "action": self.action,
            "state": self.phase.as_str(),
            "progress": self.progress,
            "message": self.message,
            "result": self.result,
            "error": self.error,
        })
    }

    /// Records a lifecycle transition.
    pub fn set_phase(&mut self, phase: OperationPhase, progress: u8, message: &str) {
        self.phase = phase;
        self.progress = progress.min(100);
        self.message = message.chars().take(1024).collect();
        self.updated = Instant::now();
    }
}

/// One replayed idempotency response.
#[derive(Clone, Debug)]
pub struct ReplayEntry {
    /// Client-provided idempotency key.
    pub key: String,
    /// Canonical request fingerprint.
    pub fingerprint: String,
    /// Prior structured response.
    pub response: Value,
    /// Expiration instant.
    pub expires_at: Instant,
}

/// Bounded replay cache with deterministic canonicalization.
#[derive(Clone, Debug, Default)]
pub struct ReplayCache {
    entries: VecDeque<ReplayEntry>,
}

impl ReplayCache {
    /// Removes expired entries and entries beyond the fixed capacity.
    pub fn prune(&mut self) {
        let now = Instant::now();
        self.entries.retain(|entry| entry.expires_at > now);
        while self.entries.len() > MAX_REPLAY_RECORDS {
            let _ = self.entries.pop_front();
        }
    }

    /// Looks up a key and returns the prior response when the fingerprint is
    /// identical.  A mismatch is reported separately by the caller.
    #[must_use]
    pub fn find(&mut self, key: &str, fingerprint: &str) -> Option<Result<Value, ()>> {
        self.prune();
        self.entries
            .iter()
            .find(|entry| entry.key == key)
            .map(|entry| {
                if entry.fingerprint == fingerprint {
                    Ok(entry.response.clone())
                } else {
                    Err(())
                }
            })
    }

    /// Inserts or replaces a replay response under the fixed retention policy.
    pub fn insert(&mut self, key: String, fingerprint: String, response: Value) {
        self.prune();
        self.entries.retain(|entry| entry.key != key);
        self.entries.push_back(ReplayEntry {
            key,
            fingerprint,
            response,
            expires_at: Instant::now() + REPLAY_TTL,
        });
        self.prune();
    }
}

/// All mutable state that belongs to the MCP adapter rather than the core.
pub struct R4State {
    /// Authoritative portable scene.
    pub scene: Scene,
    /// Last approved project path, when one has been opened/saved.
    pub project_path: Option<PathBuf>,
    /// Whether the adapter has unsaved model/project changes.
    pub dirty: bool,
    /// Adapter-owned view preferences.
    pub view: ViewState,
    /// Adapter-owned snapping preference.
    pub snapping: bool,
    /// Bounded scene-local clipboard owned by the MCP adapter.
    pub clipboard: Option<Vec<SceneObject>>,
    /// Owned temporary print handoff directories, retained until the bounded
    /// queue evicts them so a launched slicer can read its files.
    pub staging_directories: VecDeque<PathBuf>,
    /// Bounded replay cache.
    pub replay: ReplayCache,
    /// Bounded operation records.
    pub operations: VecDeque<OperationRecord>,
    /// The slicer saved in the desktop settings, used by `launch_slicer` when
    /// no `slicer_key` is given. Set by the desktop adapter; never by a tool.
    pub saved_slicer: Option<SavedSlicer>,
}

/// The desktop's saved slicer: executable path and display label.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SavedSlicer {
    /// Saved executable path.
    pub path: PathBuf,
    /// Saved display label, when the owner has one.
    pub label: Option<String>,
}

impl Default for R4State {
    fn default() -> Self {
        Self {
            scene: Scene::new(),
            project_path: None,
            dirty: false,
            view: ViewState::default(),
            snapping: true,
            clipboard: None,
            staging_directories: VecDeque::new(),
            replay: ReplayCache::default(),
            operations: VecDeque::new(),
            saved_slicer: None,
        }
    }
}

impl R4State {
    /// Prunes expired operation records while preserving active operations.
    pub fn prune_operations(&mut self) {
        let now = Instant::now();
        self.operations.retain(|record| {
            !record.phase.is_terminal() || now.duration_since(record.updated) <= OPERATION_TTL
        });
        while self.operations.len() > MAX_OPERATION_RECORDS {
            if self
                .operations
                .front()
                .is_some_and(|record| !record.phase.is_terminal())
            {
                break;
            }
            let _ = self.operations.pop_front();
        }
    }

    /// Finds an operation by its exact bounded identifier.
    pub fn operation_mut(&mut self, id: &str) -> Option<&mut OperationRecord> {
        self.operations.iter_mut().find(|record| record.id == id)
    }

    /// Finds an operation without exposing the internal queue.
    #[must_use]
    pub fn operation(&self, id: &str) -> Option<&OperationRecord> {
        self.operations.iter().find(|record| record.id == id)
    }

    /// Returns whether a non-terminal operation is currently active.
    #[must_use]
    pub fn has_active_operation(&self) -> bool {
        self.operations
            .iter()
            .any(|record| !record.phase.is_terminal())
    }

    /// Retains one owned staging directory and removes the oldest owned path
    /// when the bounded handoff queue is full.
    pub fn retain_staging_directory(&mut self, path: PathBuf) {
        self.staging_directories.push_back(path);
        while self.staging_directories.len() > MAX_STAGING_DIRECTORIES {
            if let Some(oldest) = self.staging_directories.pop_front() {
                let _ = fs::remove_dir_all(oldest);
            }
        }
    }
}

/// Canonicalizes arbitrary JSON recursively so replay fingerprints do not
/// depend on object-key insertion order.
#[must_use]
pub fn canonical_json(value: &Value) -> Value {
    match value {
        Value::Object(object) => {
            let mut sorted = Map::new();
            let mut keys = object.keys().cloned().collect::<Vec<_>>();
            keys.sort_unstable();
            for key in keys {
                if let Some(value) = object.get(&key) {
                    sorted.insert(key, canonical_json(value));
                }
            }
            Value::Object(sorted)
        }
        Value::Array(values) => Value::Array(values.iter().map(canonical_json).collect()),
        other => other.clone(),
    }
}

/// Returns a stable serialized request fingerprint without a cryptographic
/// dependency; the idempotency key remains scoped to one server process.
pub fn fingerprint(action: &str, args: &Value) -> Result<String, serde_json::Error> {
    serde_json::to_string(&json!({
        "action": action,
        "args": canonical_json(args),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_fingerprint_ignores_object_key_order() {
        let first = json!({"b": 2, "a": {"d": 4, "c": 3}});
        let second = json!({"a": {"c": 3, "d": 4}, "b": 2});
        assert_eq!(
            fingerprint("translate", &first).expect("fingerprint"),
            fingerprint("translate", &second).expect("fingerprint")
        );
    }

    #[test]
    fn operation_registry_has_bounded_lifecycle() {
        let mut state = R4State::default();
        let mut record = OperationRecord::new("smooth");
        assert_eq!(record.phase, OperationPhase::Queued);
        record.set_phase(OperationPhase::Running, 30, "working");
        assert_eq!(record.progress, 30);
        state.operations.push_back(record);
        assert!(state.has_active_operation());
    }
}
