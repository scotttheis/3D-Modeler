//! R4 MCP adapter modules.

mod catalog;
mod gateway;
mod schema;
mod security;
mod server;
mod state;

pub use catalog::{CATALOG_VERSION, Profile, TOOL_CATALOG, ToolSpec};
pub use gateway::R4Gateway;
pub use security::ApprovedRoots;
pub use server::{AdapterError, R4Config, R4Document, R4Server};
pub use state::{SavedSlicer, ViewState};
