//! Bounded application operation lifecycle shared by desktop workers.
//!
//! The registry owns status and cooperative cancellation only.  Worker
//! threads and UI adapters decide how mesh work runs and where a successful
//! result is committed.

use std::{
    collections::VecDeque,
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

/// Maximum number of terminal records retained by the application registry.
pub const MAX_OPERATION_RECORDS: usize = 128;
/// Retention window for terminal application operation records.
pub const OPERATION_RETENTION: Duration = Duration::from_secs(3_600);

static NEXT_OPERATION_ID: AtomicU64 = AtomicU64::new(1);

/// Stable operation phases shared by desktop and MCP adapters.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum OperationPhase {
    /// Work has been accepted but has not begun.
    Queued,
    /// A worker is producing a result.
    Running,
    /// The UI-thread commit boundary is active.
    Committing,
    /// Cooperative cancellation has been requested.
    Cancelling,
    /// The operation completed and its result is available.
    Completed,
    /// The operation failed before producing a result.
    Failed,
    /// The operation stopped after cancellation.
    Cancelled,
}

impl OperationPhase {
    /// Returns the stable wire spelling.
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

    /// Returns whether a worker may still update the operation.
    #[must_use]
    pub const fn is_active(self) -> bool {
        matches!(
            self,
            Self::Queued | Self::Running | Self::Committing | Self::Cancelling
        )
    }

    /// Returns whether the operation has reached a terminal state.
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}

/// A structured failure retained in an operation status record.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OperationFailure {
    /// Stable machine-readable error code.
    pub code: String,
    /// Human-readable diagnostic.
    pub message: String,
    /// Whether retrying the same operation may succeed.
    pub retryable: bool,
    /// Optional structured details for an adapter.
    pub details: Option<Value>,
}

impl OperationFailure {
    /// Creates a non-retryable failure without extra details.
    #[must_use]
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            retryable: false,
            details: None,
        }
    }

    /// Marks this failure as retryable.
    #[must_use]
    pub const fn retryable(mut self, retryable: bool) -> Self {
        self.retryable = retryable;
        self
    }
}

/// A snapshot of one operation safe to hand to a UI or automation adapter.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OperationStatus {
    /// Stable operation identifier.
    pub operation_id: String,
    /// User-facing operation title.
    pub title: String,
    /// Adapter category, such as `desktop` or `mcp`.
    pub kind: String,
    /// Optional caller request identifier.
    pub request_id: Option<String>,
    /// Current lifecycle phase.
    pub phase: OperationPhase,
    /// Inclusive 0..=100 progress, when known.
    pub progress: Option<u8>,
    /// Latest bounded progress message.
    pub message: String,
    /// Structured result after completion.
    pub result: Option<Value>,
    /// Structured failure after failure or cancellation.
    pub error: Option<OperationFailure>,
    /// Creation timestamp in Unix milliseconds.
    pub created_at: String,
    /// Start timestamp, when work began.
    pub started_at: Option<String>,
    /// Terminal timestamp, when the operation finished.
    pub finished_at: Option<String>,
}

/// A handle passed to a worker for cooperative cancellation.
#[derive(Clone, Debug)]
pub struct OperationHandle {
    /// Stable operation identifier.
    pub operation_id: String,
    cancel: Arc<AtomicBool>,
}

impl OperationHandle {
    /// Returns whether cancellation has been requested.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancel.load(Ordering::Acquire)
    }

    pub(crate) fn request_cancel(&self) {
        self.cancel.store(true, Ordering::Release);
    }
}

/// Errors from registry control operations.
#[derive(Debug, Error, Eq, PartialEq)]
pub enum OperationRegistryError {
    /// Another operation is still active.
    #[error("Builder3D is busy with another operation")]
    Busy,
    /// The requested operation ID is not retained.
    #[error("no operation exists with operation_id {0}")]
    NotFound(String),
    /// A caller attempted to update a terminal state to another phase.
    #[error("invalid active operation state: {0}")]
    InvalidPhase(String),
    /// The process could not access the registry mutex.
    #[error("operation registry lock is poisoned")]
    LockPoisoned,
}

struct OperationRecord {
    operation_id: String,
    title: String,
    kind: String,
    request_id: Option<String>,
    cancel: Arc<AtomicBool>,
    phase: OperationPhase,
    progress: Option<u8>,
    message: String,
    result: Option<Value>,
    error: Option<OperationFailure>,
    created_at: String,
    started_at: Option<String>,
    finished_at: Option<String>,
    finished_instant: Option<Instant>,
}

impl OperationRecord {
    fn status(&self) -> OperationStatus {
        OperationStatus {
            operation_id: self.operation_id.clone(),
            title: self.title.clone(),
            kind: self.kind.clone(),
            request_id: self.request_id.clone(),
            phase: self.phase,
            progress: self.progress,
            message: self.message.clone(),
            result: self.result.clone(),
            error: self.error.clone(),
            created_at: self.created_at.clone(),
            started_at: self.started_at.clone(),
            finished_at: self.finished_at.clone(),
        }
    }
}

/// Thread-safe, single-active-operation registry with bounded history.
pub struct OperationRegistry {
    records: Mutex<VecDeque<OperationRecord>>,
    max_records: usize,
    retention: Duration,
}

impl Default for OperationRegistry {
    fn default() -> Self {
        Self::new(MAX_OPERATION_RECORDS, OPERATION_RETENTION)
    }
}

impl OperationRegistry {
    /// Creates a registry with a bounded history and terminal retention.
    #[must_use]
    pub fn new(max_records: usize, retention: Duration) -> Self {
        Self {
            records: Mutex::new(VecDeque::new()),
            max_records: max_records.max(8),
            retention,
        }
    }

    /// Starts one queued operation or reports that another is active.
    pub fn start(
        &self,
        title: impl Into<String>,
        kind: impl Into<String>,
        request_id: Option<String>,
    ) -> Result<OperationHandle, OperationRegistryError> {
        let mut records = self.lock_records()?;
        Self::trim_history(&mut records, self.max_records, self.retention);
        if records.iter().any(|record| record.phase.is_active()) {
            return Err(OperationRegistryError::Busy);
        }
        let operation_id = format!(
            "desktop-op-{}",
            NEXT_OPERATION_ID.fetch_add(1, Ordering::Relaxed)
        );
        let cancel = Arc::new(AtomicBool::new(false));
        records.push_back(OperationRecord {
            operation_id: operation_id.clone(),
            title: title.into(),
            kind: kind.into(),
            request_id,
            cancel: Arc::clone(&cancel),
            phase: OperationPhase::Queued,
            progress: None,
            message: String::new(),
            result: None,
            error: None,
            created_at: timestamp(),
            started_at: None,
            finished_at: None,
            finished_instant: None,
        });
        Self::trim_history(&mut records, self.max_records, self.retention);
        Ok(OperationHandle {
            operation_id,
            cancel,
        })
    }

    /// Returns a retained status snapshot.
    pub fn status(&self, operation_id: &str) -> Result<OperationStatus, OperationRegistryError> {
        let mut records = self.lock_records()?;
        Self::trim_history(&mut records, self.max_records, self.retention);
        Self::find(&records, operation_id).map(OperationRecord::status)
    }

    /// Returns the newest active operation, if one exists.
    pub fn active_status(&self) -> Result<Option<OperationStatus>, OperationRegistryError> {
        let mut records = self.lock_records()?;
        Self::trim_history(&mut records, self.max_records, self.retention);
        Ok(records
            .iter()
            .rev()
            .find(|record| record.phase.is_active())
            .map(OperationRecord::status))
    }

    /// Returns whether an active operation currently exists.
    pub fn has_active(&self) -> Result<bool, OperationRegistryError> {
        Ok(self.active_status()?.is_some())
    }

    /// Returns whether a worker has received a cancellation request.
    pub fn cancel_requested(&self, operation_id: &str) -> Result<bool, OperationRegistryError> {
        let mut records = self.lock_records()?;
        Self::trim_history(&mut records, self.max_records, self.retention);
        Ok(Self::find(&records, operation_id)?
            .cancel
            .load(Ordering::Acquire))
    }

    /// Updates an active operation without changing its terminal state.
    pub fn update(
        &self,
        operation_id: &str,
        phase: Option<OperationPhase>,
        progress: Option<u8>,
        message: Option<&str>,
    ) -> Result<OperationStatus, OperationRegistryError> {
        let mut records = self.lock_records()?;
        let record = Self::find_mut(&mut records, operation_id)?;
        if record.phase.is_terminal() {
            return Ok(record.status());
        }
        if record.cancel.load(Ordering::Acquire) {
            return Ok(record.status());
        }
        if let Some(next) = phase {
            if !next.is_active() {
                return Err(OperationRegistryError::InvalidPhase(
                    next.as_str().to_owned(),
                ));
            }
            record.phase = next;
            if next == OperationPhase::Running && record.started_at.is_none() {
                record.started_at = Some(timestamp());
            }
        }
        if let Some(value) = progress {
            record.progress = Some(value.min(100));
        }
        if let Some(value) = message {
            record.message = value.chars().take(1_024).collect();
        }
        Ok(record.status())
    }

    /// Requests cooperative cancellation and transitions to `Cancelling`.
    pub fn cancel(&self, operation_id: &str) -> Result<OperationStatus, OperationRegistryError> {
        let mut records = self.lock_records()?;
        let record = Self::find_mut(&mut records, operation_id)?;
        if record.phase.is_terminal() {
            return Ok(record.status());
        }
        if record.phase == OperationPhase::Committing {
            return Ok(record.status());
        }
        record.cancel.store(true, Ordering::Release);
        record.phase = OperationPhase::Cancelling;
        if record.message.is_empty() {
            record.message = String::from("Cancellation requested");
        }
        Ok(record.status())
    }

    /// Completes an operation unless cancellation won the commit race.
    pub fn complete(
        &self,
        operation_id: &str,
        result: Option<Value>,
    ) -> Result<OperationStatus, OperationRegistryError> {
        let mut records = self.lock_records()?;
        let record = Self::find_mut(&mut records, operation_id)?;
        if record.phase.is_terminal() {
            return Ok(record.status());
        }
        if record.cancel.load(Ordering::Acquire) {
            Self::finish_cancelled(record, "Operation cancelled");
        } else {
            record.phase = OperationPhase::Completed;
            record.progress = Some(100);
            record.result = result;
            record.error = None;
            record.finished_at = Some(timestamp());
            record.finished_instant = Some(Instant::now());
        }
        Ok(record.status())
    }

    /// Fails an operation, converting a concurrent cancellation to cancelled.
    pub fn fail(
        &self,
        operation_id: &str,
        error: OperationFailure,
    ) -> Result<OperationStatus, OperationRegistryError> {
        let mut records = self.lock_records()?;
        let record = Self::find_mut(&mut records, operation_id)?;
        if record.phase.is_terminal() {
            return Ok(record.status());
        }
        if record.cancel.load(Ordering::Acquire) {
            Self::finish_cancelled(record, "Operation cancelled");
        } else {
            record.phase = OperationPhase::Failed;
            record.error = Some(error);
            record.finished_at = Some(timestamp());
            record.finished_instant = Some(Instant::now());
        }
        Ok(record.status())
    }

    /// Records an explicit terminal cancellation.
    pub fn cancelled(
        &self,
        operation_id: &str,
        message: impl Into<String>,
    ) -> Result<OperationStatus, OperationRegistryError> {
        let mut records = self.lock_records()?;
        let record = Self::find_mut(&mut records, operation_id)?;
        record.cancel.store(true, Ordering::Release);
        if record.phase.is_terminal() {
            return Ok(record.status());
        }
        let message = message.into();
        Self::finish_cancelled(record, &message);
        Ok(record.status())
    }

    fn lock_records(
        &self,
    ) -> Result<MutexGuard<'_, VecDeque<OperationRecord>>, OperationRegistryError> {
        self.records
            .lock()
            .map_err(|_| OperationRegistryError::LockPoisoned)
    }

    fn find<'a>(
        records: &'a VecDeque<OperationRecord>,
        operation_id: &str,
    ) -> Result<&'a OperationRecord, OperationRegistryError> {
        records
            .iter()
            .find(|record| record.operation_id == operation_id)
            .ok_or_else(|| OperationRegistryError::NotFound(operation_id.to_owned()))
    }

    fn find_mut<'a>(
        records: &'a mut VecDeque<OperationRecord>,
        operation_id: &str,
    ) -> Result<&'a mut OperationRecord, OperationRegistryError> {
        records
            .iter_mut()
            .find(|record| record.operation_id == operation_id)
            .ok_or_else(|| OperationRegistryError::NotFound(operation_id.to_owned()))
    }

    fn trim_history(
        records: &mut VecDeque<OperationRecord>,
        max_records: usize,
        retention: Duration,
    ) {
        let now = Instant::now();
        records.retain(|record| {
            !record.phase.is_terminal()
                || record
                    .finished_instant
                    .is_none_or(|finished| now.duration_since(finished) < retention)
        });
        while records.len() > max_records {
            if records
                .front()
                .is_some_and(|record| record.phase.is_active())
            {
                break;
            }
            let _ = records.pop_front();
        }
    }

    fn finish_cancelled(record: &mut OperationRecord, message: &str) {
        record.phase = OperationPhase::Cancelled;
        record.message = message.chars().take(1_024).collect();
        record.error = Some(OperationFailure::new("cancelled", record.message.clone()));
        record.finished_at = Some(timestamp());
        record.finished_instant = Some(Instant::now());
    }
}

fn timestamp() -> String {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    elapsed.as_millis().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status(result: Result<OperationStatus, OperationRegistryError>) -> OperationStatus {
        match result {
            Ok(value) => value,
            Err(error) => panic!("unexpected operation error: {error}"),
        }
    }

    #[test]
    fn lifecycle_reports_progress_and_terminal_result() {
        let registry = OperationRegistry::default();
        let handle = registry
            .start("Mesh operation", "desktop", Some(String::from("request-1")))
            .unwrap_or_else(|error| panic!("start failed: {error}"));
        let queued = status(registry.status(&handle.operation_id));
        assert_eq!(queued.phase, OperationPhase::Queued);
        assert_eq!(queued.progress, None);
        let running = status(registry.update(
            &handle.operation_id,
            Some(OperationPhase::Running),
            Some(20),
            Some("working"),
        ));
        assert_eq!(running.phase, OperationPhase::Running);
        assert_eq!(running.progress, Some(20));
        assert!(running.started_at.is_some());
        let completed = status(registry.complete(
            &handle.operation_id,
            Some(serde_json::json!({"changed": true})),
        ));
        assert_eq!(completed.phase, OperationPhase::Completed);
        assert_eq!(completed.progress, Some(100));
        assert!(completed.result.is_some());
        assert!(!registry.has_active().unwrap_or(true));
    }

    #[test]
    fn busy_and_cancellation_are_cooperative_and_idempotent() {
        let registry = OperationRegistry::default();
        let first = registry
            .start("First", "desktop", None)
            .unwrap_or_else(|error| panic!("start failed: {error}"));
        assert!(matches!(
            registry.start("Second", "desktop", None),
            Err(OperationRegistryError::Busy)
        ));
        let cancelling = status(registry.cancel(&first.operation_id));
        assert_eq!(cancelling.phase, OperationPhase::Cancelling);
        assert!(first.is_cancelled());
        let late_progress = status(registry.update(
            &first.operation_id,
            Some(OperationPhase::Running),
            Some(80),
            Some("late worker update"),
        ));
        assert_eq!(late_progress.phase, OperationPhase::Cancelling);
        assert_ne!(late_progress.progress, Some(80));
        assert!(
            registry
                .cancel_requested(&first.operation_id)
                .unwrap_or(false)
        );
        let cancelled = status(registry.complete(&first.operation_id, None));
        assert_eq!(cancelled.phase, OperationPhase::Cancelled);
        let terminal = status(registry.update(
            &first.operation_id,
            Some(OperationPhase::Running),
            Some(99),
            Some("ignored"),
        ));
        assert_eq!(terminal.phase, OperationPhase::Cancelled);
        assert_eq!(terminal.progress, None);
        assert!(!registry.has_active().unwrap_or(true));
    }

    #[test]
    fn cancellation_is_closed_after_the_commit_phase_begins() {
        let registry = OperationRegistry::default();
        let handle = registry
            .start("Commit", "desktop", None)
            .unwrap_or_else(|error| panic!("start failed: {error}"));
        assert!(
            registry
                .update(
                    &handle.operation_id,
                    Some(OperationPhase::Running),
                    None,
                    None
                )
                .is_ok()
        );
        let committing = status(registry.update(
            &handle.operation_id,
            Some(OperationPhase::Committing),
            Some(95),
            Some("Applying result"),
        ));
        assert_eq!(committing.phase, OperationPhase::Committing);
        let after_cancel = status(registry.cancel(&handle.operation_id));
        assert_eq!(after_cancel.phase, OperationPhase::Committing);
        assert!(!handle.is_cancelled());
        let completed = status(registry.complete(&handle.operation_id, None));
        assert_eq!(completed.phase, OperationPhase::Completed);
    }

    #[test]
    fn failure_and_history_are_bounded() {
        let registry = OperationRegistry::new(8, Duration::from_secs(3_600));
        for index in 0..12 {
            let handle = registry
                .start(format!("Operation {index}"), "desktop", None)
                .unwrap_or_else(|error| panic!("start failed: {error}"));
            let failed = status(registry.fail(
                &handle.operation_id,
                OperationFailure::new("mesh_failed", "bounded failure").retryable(true),
            ));
            assert_eq!(failed.phase, OperationPhase::Failed);
            assert!(failed.error.is_some());
        }
        assert!(matches!(
            registry.status("desktop-op-1"),
            Err(OperationRegistryError::NotFound(_))
        ));
    }
}
