//! Production R4 stdio entry point.

use modeler3d_mcp::R4Gateway;
use rmcp::{ServiceExt, transport::stdio};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = R4Gateway::from_env()?;
    server.serve(stdio()).await?.waiting().await?;
    Ok(())
}
