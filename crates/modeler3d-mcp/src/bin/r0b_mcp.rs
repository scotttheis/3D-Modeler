use modeler3d_mcp::R0bServer;
use modeler3d_platform::discover_capability_profile;
use rmcp::{ServiceExt, transport::stdio};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let profile = discover_capability_profile().await?;
    R0bServer::new(profile)
        .serve(stdio())
        .await?
        .waiting()
        .await?;
    Ok(())
}
