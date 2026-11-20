//! R4 MCP adapter modules.

mod catalog;
mod gateway;
mod schema;
mod security;
mod server;
mod state;

pub use catalog::{CATALOG_VERSION, Profile, SHELL_TOOLS, TOOL_CATALOG, ToolSpec};
pub use gateway::R4Gateway;
/// Checks a tool result against its declared output schema, the same check
/// the stdio gateway applies to every desktop answer on every platform.
pub use schema::validate_result as validate_tool_result;
pub use security::ApprovedRoots;
pub use server::{AdapterError, R4Config, R4Document, R4Server};
pub use state::{SavedSlicer, ViewState};

/// Temporary folder for tests. On macOS `$TMPDIR` is under the `/var` symbolic
/// link, which the approved-roots check refuses, so resolve it first.
#[cfg(test)]
pub(crate) fn test_temp_dir() -> std::path::PathBuf {
    let dir = std::env::temp_dir();
    if cfg!(windows) {
        dir
    } else {
        dir.canonicalize().unwrap_or(dir)
    }
}
