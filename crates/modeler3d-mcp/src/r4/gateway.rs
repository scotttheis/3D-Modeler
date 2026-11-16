//! Stdio MCP gateway that forwards requests to the authenticated Rust
//! desktop bridge. The gateway never owns an independent product scene.

use std::{
    io::{Read, Write},
    net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream},
    path::PathBuf,
    time::Duration,
};

use modeler3d_platform::{current_runtime_paths, process_is_alive};
use rmcp::{
    ErrorData as McpError, RoleServer, ServerHandler,
    model::{
        CallToolRequestParams, CallToolResponse, CallToolResult, ErrorCode, Implementation,
        ListToolsResult, PaginatedRequestParams, ProtocolVersion, ServerCapabilities, ServerConfig,
        Tool,
    },
    service::RequestContext,
};
use serde::Deserialize;
use serde_json::{Value, json};

use super::{
    catalog::{self, Profile},
    schema,
    server::{AdapterError, R4Config, R4Server},
};

const MAX_RESPONSE_BYTES: usize = 8 * 1024 * 1024;
const MAX_RESPONSE_HEADER_BYTES: usize = 16 * 1024;

/// Fixed-catalog stdio MCP gateway to the live 3D Modeler desktop process.
#[derive(Clone, Debug)]
pub struct R4Gateway {
    profile: Profile,
    descriptor_path: PathBuf,
}

#[derive(Debug, Deserialize)]
struct RuntimeDescriptor {
    schema: u32,
    service: String,
    pid: u32,
    host: String,
    port: u16,
    api_version: String,
    startup_id: String,
    secret: String,
}

#[derive(Debug, Deserialize)]
struct BridgeReply {
    ok: bool,
    result: Option<Value>,
    error: Option<Value>,
}

impl R4Gateway {
    /// Creates a gateway with an explicit profile and runtime descriptor path.
    #[must_use]
    pub fn new(profile: Profile, descriptor_path: PathBuf) -> Self {
        Self {
            profile,
            descriptor_path,
        }
    }

    /// Creates a gateway using the product profile and per-user runtime path.
    pub fn from_env() -> Result<Self, AdapterError> {
        let config = R4Config::from_env()?;
        Ok(Self::new(
            config.profile,
            current_runtime_paths().mcp_descriptor_path,
        ))
    }

    fn tools(&self) -> Vec<Tool> {
        R4Server::tools_for_profile(self.profile)
    }

    fn tool(&self, name: &str) -> Option<Tool> {
        catalog::find_tool(name)
            .filter(|spec| spec.visible_in(self.profile))
            .map(|spec| R4Server::tool_for_profile(self.profile, spec))
    }

    fn call_desktop(&self, name: &str, arguments: &Value) -> Result<BridgeReply, AdapterError> {
        let descriptor = self.read_descriptor()?;
        if !process_is_alive(descriptor.pid) {
            return Err(unavailable_error());
        }
        let status = http_request(&descriptor, "GET", "/status", &[])?;
        if status.status != 200
            || status.body.pointer("/ok").and_then(Value::as_bool) != Some(true)
            || status.body.pointer("/status/state").and_then(Value::as_str) != Some("ready")
            || status.body.pointer("/status/host").and_then(Value::as_str)
                != Some(descriptor.host.as_str())
            || status.body.pointer("/status/pid").and_then(Value::as_u64)
                != Some(u64::from(descriptor.pid))
            || status
                .body
                .pointer("/status/startup_id")
                .and_then(Value::as_str)
                != Some(descriptor.startup_id.as_str())
            || status.body.pointer("/status/port").and_then(Value::as_u64)
                != Some(u64::from(descriptor.port))
            || status
                .body
                .pointer("/status/api_version")
                .and_then(Value::as_str)
                != Some(descriptor.api_version.as_str())
        {
            return Err(unavailable_error());
        }
        let body = serde_json::to_vec(&json!({"name":name,"arguments":arguments}))
            .map_err(|error| AdapterError::Internal(error.to_string()))?;
        let response = http_request(&descriptor, "POST", "/mcp", &body)?;
        if response.status != 200 {
            return Err(unavailable_error());
        }
        serde_json::from_value(response.body).map_err(|_| unavailable_error())
    }

    fn read_descriptor(&self) -> Result<RuntimeDescriptor, AdapterError> {
        let bytes = std::fs::read(&self.descriptor_path).map_err(|_| unavailable_error())?;
        let descriptor: RuntimeDescriptor =
            serde_json::from_slice(&bytes).map_err(|_| unavailable_error())?;
        if descriptor.schema != 1
            || descriptor.service != "Modeler3D"
            || descriptor.host != Ipv4Addr::LOCALHOST.to_string()
            || descriptor.port == 0
            || descriptor.api_version != "1"
            || descriptor.startup_id.is_empty()
            || !valid_runtime_token(&descriptor.startup_id)
            || !valid_runtime_token(&descriptor.secret)
            || descriptor.pid == 0
        {
            return Err(unavailable_error());
        }
        Ok(descriptor)
    }
}

impl ServerHandler for R4Gateway {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_protocol_version(ProtocolVersion::V_2026_07_28)
            .with_server_info(Implementation::new(
                "modeler3d-rust-mcp",
                env!("CARGO_PKG_VERSION"),
            ))
            .with_instructions("3D Modeler exposes its fixed MCP catalog through the authenticated loopback bridge owned by the live Rust desktop. Open 3D Modeler before calling tools; file tools require configured approved roots.")
    }

    fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListToolsResult, McpError>>
    + rmcp::service::MaybeSendFuture
    + '_ {
        std::future::ready(Ok(ListToolsResult::with_all_items(self.tools())))
    }

    fn get_tool(&self, name: &str) -> Option<Tool> {
        self.tool(name)
    }

    fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<CallToolResponse, McpError>>
    + rmcp::service::MaybeSendFuture
    + '_ {
        let name = request.name.to_string();
        let arguments = Value::Object(request.arguments.unwrap_or_default());
        let gateway = self.clone();
        async move {
            let Some(spec) = catalog::find_tool(&name) else {
                return Err(McpError::new(
                    ErrorCode::METHOD_NOT_FOUND,
                    format!("unknown MCP tool: {name}"),
                    None,
                ));
            };
            if !spec.visible_in(gateway.profile) {
                return Err(McpError::new(
                    ErrorCode::METHOD_NOT_FOUND,
                    format!(
                        "tool {name} is not available in profile {}",
                        gateway.profile
                    ),
                    None,
                ));
            }
            if let Err(message) = schema::validate_arguments(&name, &arguments) {
                return Ok(CallToolResult::structured_error(
                    AdapterError::Validation(message).to_value(),
                )
                .into());
            }
            let argument_bytes = match serde_json::to_vec(&arguments) {
                Ok(bytes) => bytes.len(),
                Err(error) => {
                    return Ok(CallToolResult::structured_error(
                        AdapterError::Internal(error.to_string()).to_value(),
                    )
                    .into());
                }
            };
            if argument_bytes > super::state::MAX_ARGUMENT_BYTES {
                return Ok(CallToolResult::structured_error(
                    AdapterError::Validation(format!(
                        "argument payload exceeds {} bytes",
                        super::state::MAX_ARGUMENT_BYTES
                    ))
                    .to_value(),
                )
                .into());
            }
            let output_name = name.clone();
            let result =
                tokio::task::spawn_blocking(move || gateway.call_desktop(&name, &arguments)).await;
            match result {
                Ok(Ok(reply)) if reply.ok => match reply.result {
                    Some(value) => match schema::validate_result(&output_name, &value) {
                        Ok(()) => Ok(CallToolResult::structured(value).into()),
                        Err(message) => Ok(CallToolResult::structured_error(
                            AdapterError::Internal(format!(
                                "desktop tool {output_name} returned a value outside its declared output schema: {message}"
                            ))
                            .to_value(),
                        )
                        .into()),
                    },
                    None => {
                        Ok(CallToolResult::structured_error(unavailable_error().to_value()).into())
                    }
                },
                Ok(Ok(reply)) => Ok(CallToolResult::structured_error(reply.error.unwrap_or_else(
                    || {
                        json!({
                            "code":"internal_error",
                            "message":"desktop bridge returned an empty error",
                            "retryable":false,
                            "details":{}
                        })
                    },
                ))
                .into()),
                Ok(Err(error)) => Ok(CallToolResult::structured_error(error.to_value()).into()),
                Err(_) => {
                    Ok(CallToolResult::structured_error(unavailable_error().to_value()).into())
                }
            }
        }
    }
}

struct HttpResponse {
    status: u16,
    body: Value,
}

fn http_request(
    descriptor: &RuntimeDescriptor,
    method: &str,
    path: &str,
    body: &[u8],
) -> Result<HttpResponse, AdapterError> {
    let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), descriptor.port);
    let mut stream = TcpStream::connect_timeout(&address, Duration::from_secs(2))
        .map_err(|_| unavailable_error())?;
    stream
        .set_read_timeout(Some(Duration::from_secs(32)))
        .map_err(|_| unavailable_error())?;
    stream
        .set_write_timeout(Some(Duration::from_secs(2)))
        .map_err(|_| unavailable_error())?;
    let header = format!(
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nAuthorization: Bearer {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        descriptor.port,
        descriptor.secret,
        body.len()
    );
    stream
        .write_all(header.as_bytes())
        .and_then(|()| stream.write_all(body))
        .map_err(|_| unavailable_error())?;
    let mut bytes = Vec::new();
    stream
        .take((MAX_RESPONSE_BYTES + MAX_RESPONSE_HEADER_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| unavailable_error())?;
    if bytes.len() > MAX_RESPONSE_BYTES + MAX_RESPONSE_HEADER_BYTES {
        return Err(unavailable_error());
    }
    parse_http_response(&bytes)
}

fn parse_http_response(bytes: &[u8]) -> Result<HttpResponse, AdapterError> {
    let Some(header_end) = bytes.windows(4).position(|part| part == b"\r\n\r\n") else {
        return Err(unavailable_error());
    };
    if header_end > MAX_RESPONSE_HEADER_BYTES {
        return Err(unavailable_error());
    }
    let header = std::str::from_utf8(&bytes[..header_end]).map_err(|_| unavailable_error())?;
    let mut lines = header.split("\r\n");
    let mut status_parts = lines.next().unwrap_or_default().split_whitespace();
    if status_parts.next() != Some("HTTP/1.1") {
        return Err(unavailable_error());
    }
    let status = status_parts
        .next()
        .and_then(|value| value.parse::<u16>().ok())
        .ok_or_else(unavailable_error)?;
    let mut content_length = None;
    for line in lines {
        if let Some((name, value)) = line.split_once(':')
            && name.eq_ignore_ascii_case("content-length")
        {
            content_length = value.trim().parse::<usize>().ok();
        }
    }
    let body = &bytes[header_end + 4..];
    let length = content_length.ok_or_else(unavailable_error)?;
    if length > MAX_RESPONSE_BYTES || body.len() != length {
        return Err(unavailable_error());
    }
    let body = serde_json::from_slice(body).map_err(|_| unavailable_error())?;
    Ok(HttpResponse { status, body })
}

fn unavailable_error() -> AdapterError {
    AdapterError::Unavailable(
        "3D Modeler desktop is unavailable or its runtime descriptor is stale; open the Rust desktop and retry".to_owned(),
    )
}

fn valid_runtime_token(value: &str) -> bool {
    value.len() == 32 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gateway_publishes_the_fixed_catalog_without_owning_a_scene() {
        let gateway = R4Gateway::new(
            Profile::Full,
            std::env::temp_dir().join(format!(
                "modeler3d-r4-gateway-{}-missing.json",
                std::process::id()
            )),
        );
        let tools = gateway.tools();
        assert_eq!(tools.len(), 74);
        assert_eq!(
            tools.first().map(|tool| tool.name.as_ref()),
            Some("get_status")
        );
        assert_eq!(
            tools.last().map(|tool| tool.name.as_ref()),
            Some("exit_app")
        );
        assert!(matches!(
            gateway.call_desktop("get_status", &json!({})),
            Err(AdapterError::Unavailable(_))
        ));
    }
}
