//! Optional authenticated loopback bridge from the stdio MCP gateway to the
//! eframe owner thread. Short requests borrow the exact live `Scene`; long
//! requests receive an isolated snapshot and return through the checked core
//! history boundary.

use std::{
    collections::VecDeque,
    fs::{self, OpenOptions},
    io::{Read, Write},
    net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, SyncSender, TryRecvError, TrySendError},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use eframe::egui;
use modeler3d_mcp::{AdapterError, R4Config, R4Document, R4Server, SavedSlicer, ViewState};
use modeler3d_platform::{current_runtime_paths, process_is_alive};
use modeler3d_viewport::CameraState;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::{
    commands::{McpDocumentVersion, McpOperationCommit},
    shell::Modeler3dApp,
};

const BRIDGE_API_VERSION: &str = "1";
const MAX_HTTP_HEADER_BYTES: usize = 16 * 1024;
const MAX_HTTP_BODY_BYTES: usize = 4 * 1024 * 1024 + 64 * 1024;
const MAX_PENDING_BRIDGE_REQUESTS: usize = 8;
const MAX_STALE_OPERATION_STATUSES: usize = 128;
const OPERATION_POLL_INTERVAL: Duration = Duration::from_millis(50);
const REQUEST_RESPONSE_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Clone, Debug, Deserialize, Serialize)]
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

#[derive(Debug)]
struct BridgeRequest {
    name: String,
    arguments: Value,
    response: SyncSender<BridgeReply>,
    cancelled: Arc<AtomicBool>,
}

#[derive(Clone, Debug, Serialize)]
struct BridgeReply {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<Value>,
}

impl BridgeReply {
    fn success(result: Value) -> Self {
        Self {
            ok: true,
            result: Some(result),
            error: None,
        }
    }

    fn failure(error: AdapterError) -> Self {
        Self {
            ok: false,
            result: None,
            error: Some(error.to_value()),
        }
    }
}

struct McpDesktopBridge {
    requests: Receiver<BridgeRequest>,
    stopping: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
    descriptor_path: PathBuf,
    port: u16,
    process_id: u32,
    startup_id: String,
}

impl McpDesktopBridge {
    fn start() -> Result<Self, String> {
        Self::start_at(current_runtime_paths().mcp_descriptor_path)
    }

    fn start_at(descriptor_path: PathBuf) -> Result<Self, String> {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .map_err(|error| format!("loopback listener could not start: {error}"))?;
        listener
            .set_nonblocking(true)
            .map_err(|error| format!("loopback listener setup failed: {error}"))?;
        let port = listener
            .local_addr()
            .map_err(|error| format!("loopback listener address unavailable: {error}"))?
            .port();
        let process_id = std::process::id();
        let startup_id = Uuid::new_v4().simple().to_string();
        let descriptor = RuntimeDescriptor {
            schema: 1,
            service: "Modeler3D".to_owned(),
            pid: process_id,
            host: Ipv4Addr::LOCALHOST.to_string(),
            port,
            api_version: BRIDGE_API_VERSION.to_owned(),
            startup_id: startup_id.clone(),
            secret: Uuid::new_v4().simple().to_string(),
        };
        write_runtime_descriptor(&descriptor_path, &descriptor)?;

        let (request_sender, requests) = mpsc::sync_channel(MAX_PENDING_BRIDGE_REQUESTS);
        let stopping = Arc::new(AtomicBool::new(false));
        let worker_stopping = Arc::clone(&stopping);
        let worker_descriptor = descriptor.clone();
        let worker = match thread::Builder::new()
            .name("modeler3d-mcp-loopback".to_owned())
            .spawn(move || {
                serve_loopback(listener, request_sender, worker_descriptor, worker_stopping)
            }) {
            Ok(worker) => worker,
            Err(error) => {
                remove_owned_descriptor(&descriptor_path, process_id, &startup_id);
                return Err(format!("loopback worker could not start: {error}"));
            }
        };

        Ok(Self {
            requests,
            stopping,
            worker: Some(worker),
            descriptor_path,
            port,
            process_id,
            startup_id,
        })
    }

    fn try_receive(&self) -> Result<Option<BridgeRequest>, String> {
        match self.requests.try_recv() {
            Ok(request) => Ok(Some(request)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err("loopback request queue has stopped".to_owned()),
        }
    }
}

impl Drop for McpDesktopBridge {
    fn drop(&mut self) {
        self.stopping.store(true, Ordering::Release);
        let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), self.port);
        let _ = TcpStream::connect_timeout(&address, Duration::from_millis(100));
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        remove_owned_descriptor(&self.descriptor_path, self.process_id, &self.startup_id);
    }
}

struct PendingOperation {
    operation_id: String,
    action: String,
    document_version: McpDocumentVersion,
}

/// Adapter state owned by the desktop UI thread. The R4 server's empty scene
/// is only a detached placeholder between requests; the desktop document is
/// attached for one synchronous call or copied into an isolated long job.
pub(crate) struct McpDesktopIntegration {
    config: R4Config,
    server: R4Server,
    bridge: McpDesktopBridge,
    active_operation: Option<PendingOperation>,
    stale_statuses: VecDeque<(String, Value)>,
    last_poll: Instant,
}

impl McpDesktopIntegration {
    pub(crate) fn start() -> Result<Self, String> {
        let config = R4Config::from_env().map_err(|error| error.to_string())?;
        let bridge = McpDesktopBridge::start()?;
        Ok(Self {
            server: R4Server::new(config.clone()),
            config,
            bridge,
            active_operation: None,
            stale_statuses: VecDeque::new(),
            last_poll: Instant::now(),
        })
    }

    pub(crate) fn poll(&mut self, app: &mut Modeler3dApp, context: &egui::Context) {
        self.poll_operation(app, context);
        for _ in 0..8 {
            let request = match self.bridge.try_receive() {
                Ok(Some(request)) => request,
                Ok(None) => break,
                Err(error) => {
                    app.commands.set_status(format!("MCP unavailable: {error}"));
                    break;
                }
            };
            if request.cancelled.load(Ordering::Acquire) {
                continue;
            }
            if request.name == "capture_window" {
                // Answered from the screenshot callback once a frame is drawn.
                self.start_window_capture(context, request);
                continue;
            }
            let result = self.call_tool(app, context, &request.name, request.arguments);
            let reply = match result {
                Ok(result) => BridgeReply::success(result),
                Err(error) => BridgeReply::failure(error),
            };
            let _ = request.response.send(reply);
            context.request_repaint();
        }
        if self.active_operation.is_some() {
            context.request_repaint_after(OPERATION_POLL_INTERVAL);
        }
    }

    fn call_tool(
        &mut self,
        app: &mut Modeler3dApp,
        context: &egui::Context,
        name: &str,
        arguments: Value,
    ) -> Result<Value, AdapterError> {
        if name == "operation_status" {
            self.finish_terminal_operation(app, context);
            let result = self.server.call_value(name, arguments.clone())?;
            if let Some(id) = arguments.get("operation_id").and_then(Value::as_str)
                && let Some(status) = self.stale_status(id)
            {
                return Ok(status);
            }
            return Ok(result);
        }
        if name == "cancel_operation" {
            return self.server.call_value(name, arguments);
        }
        if name == "get_status" && self.active_operation.is_some() {
            return self.live_status(app, arguments);
        }

        if self.server.is_shell_tool(name) {
            // Settings, ribbon, screens and recent-file lists do not touch the
            // document, so they work during a long operation; opening a file
            // or exiting waits for it.
            if self.active_operation.is_some() && matches!(name, "open_recent" | "exit_app") {
                return Err(AdapterError::Busy);
            }
            let version = app.commands.mcp_document_version();
            let result = self
                .server
                .call_shell_value(name, arguments, |args| app.call_shell_tool(name, args));
            if result.is_ok() && name == "open_recent" {
                app.synchronize_mcp_presentation(&version.project_id, version.revision, false);
            }
            return result;
        }

        if self.active_operation.is_some() {
            if let Some(replay) = self.server.replay_value(name, &arguments)? {
                return Ok(replay);
            }
            if self.server.is_long_running_tool(name) == Some(true) {
                return self.server.call_value(name, arguments);
            }
            if self.server.is_read_only_tool(name) == Some(true) {
                return self.call_isolated_read(app, name, arguments);
            }
            return Err(AdapterError::Busy);
        }

        if matches!(name, "new_project" | "open_project") {
            if let Some(replay) = self.server.replay_value(name, &arguments)? {
                return Ok(replay);
            }
            if app.commands.is_dirty() {
                return Err(AdapterError::Conflict(
                    "the current project has unsaved changes; save it before creating or opening a project"
                        .to_owned(),
                ));
            }
        }

        match self.server.is_long_running_tool(name) {
            Some(true) => self.start_operation(app, name, arguments),
            Some(false) if name == "frame_view" => {
                let target = arguments.get("target").cloned();
                let before = app.camera.state();
                let result = self.call_synchronous(app, name, arguments)?;
                Ok(frame_on_desktop(app, target.as_ref(), before, result))
            }
            Some(false) => self.call_synchronous(app, name, arguments),
            None => self.server.call_value(name, arguments),
        }
    }

    /// `capture_window` (OF-58): checks the call and the approved output path
    /// now, then asks for a screenshot of the next frame; the callback writes
    /// the PNG and answers the waiting MCP request itself.
    fn start_window_capture(&self, context: &egui::Context, request: BridgeRequest) {
        let prepared = self
            .server
            .prepare_shell_call("capture_window", &request.arguments)
            .and_then(|args| {
                let raw = args.get("path").and_then(Value::as_str).unwrap_or_default();
                let overwrite = args
                    .get("overwrite")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                if !Path::new(raw)
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("png"))
                {
                    return Err(AdapterError::Validation(
                        "capture_window writes a .png file".to_owned(),
                    ));
                }
                self.server.approved_write_path(raw, overwrite)
            });
        let path = match prepared {
            Ok(path) => path,
            Err(error) => {
                let _ = request.response.send(BridgeReply::failure(error));
                return;
            }
        };
        let server = self.server.clone();
        let response = request.response;
        context.send_viewport_cmd(egui::ViewportCommand::Screenshot(
            egui::ScreenshotCallback::new(move |image| {
                let reply = crate::review_capture::save_png(&path, &image)
                    .map_err(|error| {
                        AdapterError::Internal(format!("could not write the PNG: {error}"))
                    })
                    .and_then(|()| {
                        let bytes = fs::metadata(&path)
                            .map_err(|error| AdapterError::Internal(error.to_string()))?
                            .len();
                        server.finish_shell_result(
                            "capture_window",
                            json!({
                                "path": path.display().to_string(),
                                "width_px": image.size[0],
                                "height_px": image.size[1],
                                "bytes": bytes,
                            }),
                        )
                    });
                let _ = response.send(match reply {
                    Ok(result) => BridgeReply::success(result),
                    Err(error) => BridgeReply::failure(error),
                });
            }),
        ));
        context.request_repaint();
    }

    fn live_status(&self, app: &Modeler3dApp, arguments: Value) -> Result<Value, AdapterError> {
        let mut status = self.server.call_value("get_status", arguments)?;
        if let Some(object) = status.as_object_mut() {
            object.insert("app_state".to_owned(), json!("busy"));
            object.insert("busy".to_owned(), json!(true));
            object.insert(
                "project_id".to_owned(),
                json!(app.commands.scene().project_id()),
            );
            object.insert(
                "scene_revision".to_owned(),
                json!(app.commands.scene().revision()),
            );
            object.insert("dirty".to_owned(), json!(app.commands.is_dirty()));
            if let Some(operation) = &self.active_operation {
                let active_operation = self
                    .server
                    .call_value(
                        "operation_status",
                        json!({"operation_id": operation.operation_id}),
                    )
                    .unwrap_or_else(|_| {
                        json!({
                            "operation_id": operation.operation_id,
                            "action": operation.action,
                            "state": "running",
                        })
                    });
                object.insert("active_operation".to_owned(), active_operation);
            }
        }
        Ok(status)
    }

    fn call_synchronous(
        &mut self,
        app: &mut Modeler3dApp,
        name: &str,
        arguments: Value,
    ) -> Result<Value, AdapterError> {
        let document_version = app.commands.mcp_document_version();
        let view = view_from_app(app);
        let document = app.commands.take_mcp_document(view);
        self.call_with_server(
            &self.server,
            app,
            name,
            arguments,
            document,
            &document_version,
        )
    }

    fn call_isolated_read(
        &self,
        app: &mut Modeler3dApp,
        name: &str,
        arguments: Value,
    ) -> Result<Value, AdapterError> {
        let server = R4Server::new(self.config.clone());
        let document_version = app.commands.mcp_document_version();
        let view = view_from_app(app);
        let document = app.commands.take_mcp_document(view);
        self.call_with_server(&server, app, name, arguments, document, &document_version)
    }

    fn call_with_server(
        &self,
        server: &R4Server,
        app: &mut Modeler3dApp,
        name: &str,
        arguments: Value,
        document: R4Document,
        expected: &McpDocumentVersion,
    ) -> Result<Value, AdapterError> {
        let mut document = Some(document);
        if let Err(error) = server.replace_document(&mut document) {
            if let Some(document) = document {
                let _ = install_document(app, document, expected);
            }
            return Err(error);
        }
        let result = server.call_value(name, arguments);
        let document = server.take_document()?;
        install_document(app, document, expected)?;
        if result.is_ok() {
            app.synchronize_mcp_presentation(&expected.project_id, expected.revision, false);
        }
        result
    }

    fn start_operation(
        &mut self,
        app: &mut Modeler3dApp,
        name: &str,
        arguments: Value,
    ) -> Result<Value, AdapterError> {
        // A completed idempotent retry may return the original operation ID.
        // Its scene result was already committed or rejected, so do not attach
        // a fresh desktop snapshot and mistake that terminal result for work.
        if let Some(replay) = self.server.replay_value(name, &arguments)? {
            return Ok(replay);
        }
        let document_version = app.commands.mcp_document_version();
        let view = view_from_app(app);
        let document = app
            .commands
            .snapshot_mcp_document(view)
            .map_err(|error| AdapterError::Validation(error.to_string()))?;
        let mut document = Some(document);
        self.server.replace_document(&mut document)?;
        // The Slicer button's saved slicer is also the default for
        // `launch_slicer` when the call names none (F12 parity).
        let preferences = app.commands.preferences();
        self.server
            .set_saved_slicer(preferences.preferred_slicer_path().map(|path| SavedSlicer {
                path: path.to_path_buf(),
                label: preferences.preferred_slicer_label().map(str::to_owned),
            }))?;
        let result = match self.server.call_value(name, arguments) {
            Ok(result) => result,
            Err(error) => {
                let _ = self.server.take_document();
                return Err(error);
            }
        };
        let Some(operation_id) = result
            .get("operation_id")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
        else {
            let _ = self.server.take_document();
            return Err(AdapterError::Internal(
                "long operation response did not include an operation ID".to_owned(),
            ));
        };
        // Even a very short job can finish before this method returns. Keep its
        // isolated document attached and let the normal terminal poll commit it.
        self.active_operation = Some(PendingOperation {
            operation_id,
            action: name.to_owned(),
            document_version,
        });
        Ok(result)
    }

    fn poll_operation(&mut self, app: &mut Modeler3dApp, context: &egui::Context) {
        if self.last_poll.elapsed() < OPERATION_POLL_INTERVAL {
            return;
        }
        self.last_poll = Instant::now();
        self.finish_terminal_operation(app, context);
    }

    fn finish_terminal_operation(&mut self, app: &mut Modeler3dApp, context: &egui::Context) {
        let Some(operation_id) = self
            .active_operation
            .as_ref()
            .map(|operation| operation.operation_id.clone())
        else {
            return;
        };
        let status = self
            .server
            .call_value("operation_status", json!({"operation_id": operation_id}));
        let Ok(status) = status else {
            return;
        };
        let state = status
            .get("state")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if !matches!(state, "completed" | "failed" | "cancelled") {
            return;
        }
        let Some(active) = self.active_operation.take() else {
            return;
        };
        let Ok(document) = self.server.take_document() else {
            return;
        };
        let commit_result = if state == "completed" {
            let commit = match active.action.as_str() {
                "open_project" => McpOperationCommit::ProjectReplace,
                "save_project" | "save_project_as" => McpOperationCommit::ProjectSave,
                _ => McpOperationCommit::SceneUpdate,
            };
            let result = app.commands.commit_mcp_operation_document(
                &active.document_version,
                commit,
                document,
            );
            if result.is_ok() {
                app.synchronize_mcp_presentation(
                    &active.document_version.project_id,
                    active.document_version.revision,
                    commit == McpOperationCommit::ProjectReplace,
                );
            }
            result
        } else {
            Ok(())
        };
        if let Err(error) = commit_result {
            self.retain_stale_status(&active, error.to_string());
            app.commands
                .set_status("MCP operation result discarded because the active document changed");
        }
        context.request_repaint();
    }

    fn retain_stale_status(&mut self, operation: &PendingOperation, message: String) {
        let value = json!({
            "operation_id": operation.operation_id,
            "action": operation.action,
            "state": "failed",
            "progress": 100,
            "message": "active document changed while the operation was running; result was discarded",
            "result": null,
            "error": {
                "code": "revision_conflict",
                "message": message,
                "retryable": false,
                "details": {}
            }
        });
        self.stale_statuses
            .retain(|(id, _)| id != &operation.operation_id);
        self.stale_statuses
            .push_back((operation.operation_id.clone(), value));
        while self.stale_statuses.len() > MAX_STALE_OPERATION_STATUSES {
            let _ = self.stale_statuses.pop_front();
        }
    }

    fn stale_status(&self, operation_id: &str) -> Option<Value> {
        self.stale_statuses
            .iter()
            .find(|(id, _)| id == operation_id)
            .map(|(_, status)| status.clone())
    }
}

/// MCP `frame_view` in the live app frames with the desktop camera, as the
/// Python reference does, so the AI gets the same camera as a double-click or
/// the Fit buttons (OF-63). The adapter has already checked the arguments;
/// this replaces only its camera with the desktop one. `selected` with an
/// empty selection frames everything, and hidden objects are skipped.
fn frame_on_desktop(
    app: &mut Modeler3dApp,
    target: Option<&Value>,
    before: CameraState,
    mut result: Value,
) -> Value {
    let scene = app.commands.scene();
    let selected = scene.selection().iter().copied().collect::<Vec<_>>();
    let indices = if target.and_then(Value::as_str) == Some("selected") && !selected.is_empty() {
        selected
    } else {
        (0..scene.objects().len()).collect()
    };
    if !app.frame_objects(&indices) {
        // Nothing visible to frame: the camera stays exactly where it was.
        let _ = app.camera.set_state(before);
    }
    if let Some(object) = result.as_object_mut() {
        object.insert("view".to_owned(), view_from_app(app).to_value());
    }
    result
}

fn view_from_app(app: &Modeler3dApp) -> ViewState {
    let camera = app.camera.state();
    let display = app.commands.display_state();
    ViewState {
        yaw_deg: camera.yaw_degrees,
        pitch_deg: camera.pitch_degrees,
        distance_mm: camera.distance,
        center_mm: camera.center,
        show_grid: display.grid_visible,
        wireframe: display.wireframe,
        xray: display.xray,
    }
}

fn install_document(
    app: &mut Modeler3dApp,
    document: R4Document,
    expected: &McpDocumentVersion,
) -> Result<(), AdapterError> {
    let camera = CameraState {
        yaw_degrees: document.view.yaw_deg,
        pitch_degrees: document.view.pitch_deg,
        distance: document.view.distance_mm,
        center: document.view.center_mm,
    };
    app.camera
        .set_state(camera)
        .map_err(|error| AdapterError::Internal(error.to_string()))?;
    app.commands
        .install_mcp_document(document, expected)
        .map_err(|error| AdapterError::Internal(error.to_string()))
}

#[derive(Debug)]
struct HttpRequest {
    method: String,
    path: String,
    host: String,
    authorization: Option<String>,
    content_type: Option<String>,
    origin: Option<String>,
    content_length: usize,
    body: Vec<u8>,
}

fn serve_loopback(
    listener: TcpListener,
    requests: SyncSender<BridgeRequest>,
    descriptor: RuntimeDescriptor,
    stopping: Arc<AtomicBool>,
) {
    while !stopping.load(Ordering::Acquire) {
        match listener.accept() {
            Ok((mut stream, address)) => {
                if !address.ip().is_loopback() {
                    write_http_response(&mut stream, 403, json!({"error":"forbidden"}));
                    continue;
                }
                handle_http_connection(&mut stream, &requests, &descriptor, &stopping);
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(10));
            }
            Err(_) => thread::sleep(Duration::from_millis(25)),
        }
    }
}

fn handle_http_connection(
    stream: &mut TcpStream,
    requests: &SyncSender<BridgeRequest>,
    descriptor: &RuntimeDescriptor,
    stopping: &AtomicBool,
) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
    let request = match read_http_request(stream) {
        Ok(request) => request,
        Err(status) => {
            write_http_response(stream, status, json!({"error":"invalid request"}));
            return;
        }
    };
    let expected_authorization = format!("Bearer {}", descriptor.secret);
    if !request.authorization.as_deref().is_some_and(|provided| {
        constant_time_eq(provided.as_bytes(), expected_authorization.as_bytes())
    }) {
        write_http_response(stream, 401, json!({"error":"unauthorized"}));
        return;
    }
    if request.host != format!("127.0.0.1:{}", descriptor.port) {
        write_http_response(stream, 400, json!({"error":"invalid host"}));
        return;
    }
    if request.origin.is_some() {
        write_http_response(
            stream,
            403,
            json!({"error":"browser origins are not supported"}),
        );
        return;
    }
    if request.method == "GET" && request.path == "/status" {
        write_http_response(
            stream,
            200,
            json!({
                "ok": true,
                "status": {
                    "state": "ready",
                    "pid": descriptor.pid,
                    "host": descriptor.host,
                    "startup_id": descriptor.startup_id,
                    "port": descriptor.port,
                    "api_version": descriptor.api_version,
                }
            }),
        );
        return;
    }
    if request.method != "POST" || request.path != "/mcp" {
        write_http_response(stream, 404, json!({"error":"not found"}));
        return;
    }
    if request.content_type.as_deref() != Some("application/json") {
        write_http_response(stream, 415, json!({"error":"application/json is required"}));
        return;
    }
    if request.content_length == 0 || request.body.len() != request.content_length {
        write_http_response(stream, 400, json!({"error":"invalid body length"}));
        return;
    }
    let payload: Value = match serde_json::from_slice(&request.body) {
        Ok(payload) => payload,
        Err(_) => {
            write_http_response(stream, 400, json!({"error":"invalid JSON"}));
            return;
        }
    };
    let Some(name) = payload.get("name").and_then(Value::as_str) else {
        write_http_response(stream, 400, json!({"error":"tool name is required"}));
        return;
    };
    let Some(arguments) = payload.get("arguments").filter(|value| value.is_object()) else {
        write_http_response(
            stream,
            400,
            json!({"error":"tool arguments must be an object"}),
        );
        return;
    };
    let (response_sender, response_receiver) = mpsc::sync_channel(1);
    let cancelled = Arc::new(AtomicBool::new(false));
    let bridge_request = BridgeRequest {
        name: name.to_owned(),
        arguments: arguments.clone(),
        response: response_sender,
        cancelled: Arc::clone(&cancelled),
    };
    match requests.try_send(bridge_request) {
        Ok(()) => {}
        Err(TrySendError::Full(_)) => {
            write_http_response(
                stream,
                503,
                json!({"error":"desktop request queue is full"}),
            );
            return;
        }
        Err(TrySendError::Disconnected(_)) => {
            write_http_response(stream, 503, json!({"error":"desktop is shutting down"}));
            return;
        }
    }
    let started = Instant::now();
    loop {
        if stopping.load(Ordering::Acquire) {
            cancelled.store(true, Ordering::Release);
            write_http_response(stream, 503, json!({"error":"desktop is shutting down"}));
            return;
        }
        if started.elapsed() >= REQUEST_RESPONSE_TIMEOUT {
            cancelled.store(true, Ordering::Release);
            write_http_response(stream, 503, json!({"error":"desktop response timed out"}));
            return;
        }
        match response_receiver.recv_timeout(Duration::from_millis(100)) {
            Ok(reply) => {
                write_http_response(stream, 200, serde_json::to_value(reply).unwrap_or_else(|_| {
                    json!({"ok":false,"error":{"code":"internal_error","message":"response serialization failed"}})
                }));
                return;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                write_http_response(
                    stream,
                    503,
                    json!({"error":"desktop request was abandoned"}),
                );
                return;
            }
        }
    }
}

fn read_http_request(stream: &mut TcpStream) -> Result<HttpRequest, u16> {
    let mut header = Vec::with_capacity(1024);
    let mut byte = [0_u8; 1];
    while header.len() < MAX_HTTP_HEADER_BYTES {
        match stream.read(&mut byte) {
            Ok(0) => return Err(400),
            Ok(_) => {
                header.push(byte[0]);
                if header.ends_with(b"\r\n\r\n") {
                    break;
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) if error.kind() == std::io::ErrorKind::TimedOut => return Err(408),
            Err(_) => return Err(400),
        }
    }
    if !header.ends_with(b"\r\n\r\n") {
        return Err(431);
    }
    let text = std::str::from_utf8(&header).map_err(|_| 400_u16)?;
    let mut lines = text.split("\r\n");
    let mut request_parts = lines.next().unwrap_or_default().split_whitespace();
    let method = request_parts.next().ok_or(400_u16)?.to_owned();
    let path = request_parts.next().ok_or(400_u16)?.to_owned();
    let version = request_parts.next().ok_or(400_u16)?;
    if version != "HTTP/1.1" || request_parts.next().is_some() {
        return Err(400);
    }
    let mut host = None;
    let mut authorization = None;
    let mut content_type = None;
    let mut origin = None;
    let mut content_length = None;
    for line in lines {
        if line.is_empty() {
            break;
        }
        let Some((name, value)) = line.split_once(':') else {
            return Err(400);
        };
        if name.eq_ignore_ascii_case("host") {
            if host.is_some() {
                return Err(400);
            }
            host = Some(value.trim().to_owned());
        } else if name.eq_ignore_ascii_case("authorization") {
            if authorization.is_some() {
                return Err(400);
            }
            authorization = Some(value.trim().to_owned());
        } else if name.eq_ignore_ascii_case("content-type") {
            if content_type.is_some() {
                return Err(400);
            }
            content_type = Some(value.trim().to_owned());
        } else if name.eq_ignore_ascii_case("origin") {
            if origin.is_some() {
                return Err(400);
            }
            origin = Some(value.trim().to_owned());
        } else if name.eq_ignore_ascii_case("content-length") {
            if content_length.is_some() {
                return Err(400);
            }
            let length = value.trim().parse::<usize>().map_err(|_| 400_u16)?;
            if length > MAX_HTTP_BODY_BYTES {
                return Err(413);
            }
            content_length = Some(length);
        } else if name.eq_ignore_ascii_case("transfer-encoding") {
            return Err(400);
        }
    }
    let host = host.ok_or(400_u16)?;
    let content_length = content_length.unwrap_or(0);
    let mut body = vec![0; content_length];
    if content_length > 0 {
        stream.read_exact(&mut body).map_err(|_| 400_u16)?;
    }
    Ok(HttpRequest {
        method,
        path,
        host,
        authorization,
        content_type,
        origin,
        content_length,
        body,
    })
}

fn write_http_response(stream: &mut TcpStream, status: u16, body: Value) {
    let body = serde_json::to_vec(&body).unwrap_or_else(|_| b"{}".to_vec());
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        408 => "Request Timeout",
        413 => "Payload Too Large",
        415 => "Unsupported Media Type",
        431 => "Request Header Fields Too Large",
        503 => "Service Unavailable",
        _ => "Internal Server Error",
    };
    let header = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(header.as_bytes());
    let _ = stream.write_all(&body);
}

fn write_runtime_descriptor(path: &Path, descriptor: &RuntimeDescriptor) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("runtime directory could not be created: {error}"))?;
    }
    if path.exists() {
        let existing = fs::read(path)
            .map_err(|error| format!("existing runtime descriptor could not be read: {error}"))?;
        let Ok(existing) = serde_json::from_slice::<RuntimeDescriptor>(&existing) else {
            return Err("an unrecognized runtime descriptor already exists".to_owned());
        };
        if existing.service != "Modeler3D" || existing.host != Ipv4Addr::LOCALHOST.to_string() {
            return Err("the runtime descriptor is not owned by 3D Modeler".to_owned());
        }
        if process_is_alive(existing.pid) {
            return Err("another 3D Modeler process already owns the MCP bridge".to_owned());
        }
        fs::remove_file(path).map_err(|error| {
            format!("stale 3D Modeler runtime descriptor could not be removed: {error}")
        })?;
    }
    let parent = path
        .parent()
        .ok_or_else(|| "runtime descriptor path has no parent directory".to_owned())?;
    let temporary_path = parent.join(format!(
        "{}.{}.tmp",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("mcp-runtime.json"),
        descriptor.startup_id
    ));
    let result = (|| -> Result<(), String> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary_path)
            .map_err(|error| format!("runtime descriptor staging failed: {error}"))?;
        serde_json::to_writer(&mut file, descriptor)
            .map_err(|error| format!("runtime descriptor encoding failed: {error}"))?;
        file.sync_all()
            .map_err(|error| format!("runtime descriptor flush failed: {error}"))?;
        fs::rename(&temporary_path, path)
            .map_err(|error| format!("runtime descriptor publish failed: {error}"))?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary_path);
    }
    result
}

fn remove_owned_descriptor(path: &Path, process_id: u32, startup_id: &str) {
    let Ok(bytes) = fs::read(path) else {
        return;
    };
    let Ok(descriptor) = serde_json::from_slice::<RuntimeDescriptor>(&bytes) else {
        return;
    };
    if descriptor.pid == process_id && descriptor.startup_id == startup_id {
        let _ = fs::remove_file(path);
    }
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    let mut difference = left.len() ^ right.len();
    let max_length = left.len().max(right.len());
    for index in 0..max_length {
        difference |= usize::from(
            left.get(index).copied().unwrap_or_default()
                ^ right.get(index).copied().unwrap_or_default(),
        );
    }
    difference == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::CommandId;
    use modeler3d_core::PrimitiveKind;
    use std::net::TcpStream;

    fn temporary_descriptor() -> (PathBuf, PathBuf) {
        let root = std::env::temp_dir().join(format!(
            "modeler3d-rb09-bridge-{}-{}",
            std::process::id(),
            Uuid::new_v4().simple()
        ));
        let path = root.join("mcp-runtime.json");
        (root, path)
    }

    fn request(
        port: u16,
        authorization: Option<&str>,
        body: Option<&[u8]>,
        content_type: Option<&str>,
        origin: Option<&str>,
    ) -> Vec<u8> {
        let mut stream = TcpStream::connect((Ipv4Addr::LOCALHOST, port)).expect("connect");
        let body = body.unwrap_or_default();
        let authorization = authorization
            .map(|value| format!("Authorization: {value}\r\n"))
            .unwrap_or_default();
        let content_type = content_type
            .map(|value| format!("Content-Type: {value}\r\n"))
            .unwrap_or_default();
        let origin = origin
            .map(|value| format!("Origin: {value}\r\n"))
            .unwrap_or_default();
        let path = if body.is_empty() { "/status" } else { "/mcp" };
        let method = if body.is_empty() { "GET" } else { "POST" };
        let header = format!(
            "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n{authorization}{content_type}{origin}Content-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        stream.write_all(header.as_bytes()).expect("write header");
        stream.write_all(body).expect("write body");
        let mut response = Vec::new();
        stream.read_to_end(&mut response).expect("read response");
        response
    }

    #[test]
    fn loopback_bridge_authenticates_queues_and_removes_only_its_descriptor() {
        let (root, descriptor_path) = temporary_descriptor();
        let bridge = McpDesktopBridge::start_at(descriptor_path.clone()).expect("start bridge");
        let descriptor: RuntimeDescriptor =
            serde_json::from_slice(&fs::read(&descriptor_path).expect("read descriptor"))
                .expect("decode descriptor");

        let rejected = request(bridge.port, None, None, None, None);
        assert!(rejected.starts_with(b"HTTP/1.1 401"));
        let status = request(
            bridge.port,
            Some(&format!("Bearer {}", descriptor.secret)),
            None,
            None,
            None,
        );
        let status_text = String::from_utf8(status).expect("status response UTF-8");
        assert!(status_text.starts_with("HTTP/1.1 200"));
        assert!(status_text.contains("\"api_version\":\"1\""));
        assert!(status_text.contains(&descriptor.startup_id));

        let payload = serde_json::to_vec(&json!({
            "name":"scene_summary",
            "arguments":{}
        }))
        .expect("encode request");
        let missing_content_type = request(
            bridge.port,
            Some(&format!("Bearer {}", descriptor.secret)),
            Some(&payload),
            None,
            None,
        );
        assert!(missing_content_type.starts_with(b"HTTP/1.1 415"));
        let browser_origin = request(
            bridge.port,
            Some(&format!("Bearer {}", descriptor.secret)),
            Some(&payload),
            Some("application/json"),
            Some("https://example.invalid"),
        );
        assert!(browser_origin.starts_with(b"HTTP/1.1 403"));
        let client = thread::spawn({
            let secret = descriptor.secret.clone();
            let port = bridge.port;
            move || {
                request(
                    port,
                    Some(&format!("Bearer {secret}")),
                    Some(&payload),
                    Some("application/json"),
                    None,
                )
            }
        });
        let queued = (0..100)
            .find_map(|_| match bridge.try_receive() {
                Ok(Some(request)) => Some(request),
                Ok(None) => {
                    thread::sleep(Duration::from_millis(10));
                    None
                }
                Err(error) => panic!("bridge queue failed: {error}"),
            })
            .expect("queued tool request");
        assert_eq!(queued.name, "scene_summary");
        assert_eq!(queued.arguments, json!({}));
        queued
            .response
            .send(BridgeReply::success(json!({"scene_revision":0})))
            .expect("reply to client");
        let accepted = client.join().expect("client response");
        let response_text = String::from_utf8(accepted).expect("HTTP response UTF-8");
        assert!(response_text.starts_with("HTTP/1.1 200"));
        assert!(response_text.contains("\"scene_revision\":0"));

        drop(bridge);
        assert!(!descriptor_path.exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn shell_tools_route_to_the_desktop_and_capture_checks_its_path_first() {
        let (root, descriptor_path) = temporary_descriptor();
        fs::create_dir_all(&root).expect("root");
        let config = R4Config::for_profile(
            modeler3d_mcp::Profile::Full,
            modeler3d_mcp::ApprovedRoots::from_paths(vec![root.clone()]).expect("roots"),
        );
        let bridge = McpDesktopBridge::start_at(descriptor_path.clone()).expect("start bridge");
        let mut integration = McpDesktopIntegration {
            server: R4Server::new(config.clone()),
            config,
            bridge,
            active_operation: None,
            stale_statuses: VecDeque::new(),
            last_poll: Instant::now(),
        };
        let mut app = Modeler3dApp::default();
        let context = egui::Context::default();

        // A read-only shell tool goes to the shell, not the document adapter.
        let settings = integration
            .call_tool(&mut app, &context, "app_settings", json!({}))
            .expect("app settings");
        assert_eq!(settings["about"]["product"], json!("3D Modeler"));
        assert_eq!(settings["screen"], json!("start"));

        // capture_window answers bad paths at once, before any screenshot.
        for (arguments, code) in [
            (
                json!({"path": root.join("shot.txt").display().to_string()}),
                "invalid_input",
            ),
            (json!({"path": "C:/outside/shot.png"}), "path_policy"),
            (json!({}), "invalid_input"),
        ] {
            let (sender, receiver) = mpsc::sync_channel(1);
            integration.start_window_capture(
                &context,
                BridgeRequest {
                    name: "capture_window".to_owned(),
                    arguments,
                    response: sender,
                    cancelled: Arc::new(AtomicBool::new(false)),
                },
            );
            let reply = receiver.try_recv().expect("immediate reply");
            assert!(!reply.ok);
            assert_eq!(
                reply.error.as_ref().and_then(|error| error.get("code")),
                Some(&json!(code))
            );
        }

        drop(integration);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn project_replacement_tools_respect_the_live_dirty_guard() {
        let (root, descriptor_path) = temporary_descriptor();
        let config = R4Config::for_profile(
            modeler3d_mcp::Profile::Full,
            modeler3d_mcp::ApprovedRoots::from_paths(Vec::new()).expect("deny-all roots"),
        );
        let bridge = McpDesktopBridge::start_at(descriptor_path.clone()).expect("start bridge");
        let mut integration = McpDesktopIntegration {
            server: R4Server::new(config.clone()),
            config,
            bridge,
            active_operation: None,
            stale_statuses: VecDeque::new(),
            last_poll: Instant::now(),
        };
        let mut app = Modeler3dApp::default();
        app.commands
            .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
            .expect("add dirty project object");
        let context = egui::Context::default();

        for (name, arguments) in [
            ("new_project", json!({})),
            ("open_project", json!({"path":"C:/outside/project.3dp"})),
        ] {
            assert!(matches!(
                integration.call_tool(&mut app, &context, name, arguments),
                Err(AdapterError::Conflict(message)) if message.contains("unsaved changes")
            ));
            assert_eq!(app.commands.scene().objects().len(), 1);
            assert!(app.commands.is_dirty());
        }

        drop(integration);
        assert!(!descriptor_path.exists());
        let _ = fs::remove_dir_all(root);
    }

    fn framing_integration() -> (McpDesktopIntegration, PathBuf) {
        let (root, descriptor_path) = temporary_descriptor();
        let config = R4Config::for_profile(
            modeler3d_mcp::Profile::Full,
            modeler3d_mcp::ApprovedRoots::from_paths(Vec::new()).expect("deny-all roots"),
        );
        let bridge = McpDesktopBridge::start_at(descriptor_path).expect("start bridge");
        let integration = McpDesktopIntegration {
            server: R4Server::new(config.clone()),
            config,
            bridge,
            active_operation: None,
            stale_statuses: VecDeque::new(),
            last_poll: Instant::now(),
        };
        (integration, root)
    }

    /// Two apps holding the same scene: a small cube at the origin and a
    /// large one far along +X, both built through MCP.
    fn framing_scene(
        integration: &mut McpDesktopIntegration,
        context: &egui::Context,
    ) -> Modeler3dApp {
        let mut app = Modeler3dApp::default();
        app.camera
            .set_viewport_size(1280, 720)
            .expect("viewport size");
        for (name, arguments) in [
            (
                "add_primitive",
                json!({"primitive":"cube","dimensions_mm":[10.0,10.0,10.0]}),
            ),
            (
                "add_primitive",
                json!({"primitive":"cube","dimensions_mm":[40.0,30.0,20.0]}),
            ),
            ("translate", json!({"index":1,"offset_mm":[300.0,50.0,0.0]})),
        ] {
            integration
                .call_tool(&mut app, context, name, arguments)
                .expect("build scene");
        }
        app
    }

    #[test]
    fn items_and_viewport_double_click_frame_like_mcp_frame_view() {
        let (mut integration, root) = framing_integration();
        let context = egui::Context::default();

        // Desktop: the Items double-click (two plain clicks, then centring)
        // ends with the row as the only selection and the object framed.
        let mut desktop = framing_scene(&mut integration, &context);
        let history = desktop.commands.scene().history_state_id();
        let dirty = desktop.commands.is_dirty();
        let before = desktop.camera.state();
        desktop.commands.select_index(0);
        assert!(desktop.centre_on_object(1));
        let framed = desktop.camera.state();
        assert_ne!(framed, before);
        assert_eq!(framed.yaw_degrees, before.yaw_degrees);
        assert_eq!(framed.pitch_degrees, before.pitch_degrees);
        assert!((framed.center[0] - 300.0).abs() < 1.0e-3);
        assert_eq!(
            desktop
                .commands
                .scene()
                .selection()
                .iter()
                .copied()
                .collect::<Vec<_>>(),
            [1]
        );
        assert_eq!(desktop.commands.scene().history_state_id(), history);
        assert_eq!(desktop.commands.is_dirty(), dirty);

        // MCP: select + frame_view(selected) gives the very same camera.
        let mut ai = framing_scene(&mut integration, &context);
        integration
            .call_tool(&mut ai, &context, "select", json!({"indices":[1]}))
            .expect("select");
        let result = integration
            .call_tool(
                &mut ai,
                &context,
                "frame_view",
                json!({"target":"selected"}),
            )
            .expect("frame view");
        assert_eq!(ai.camera.state(), framed);
        assert_eq!(result["view"]["distance_mm"], json!(framed.distance));

        // frame_view(all) matches the desktop framing of every object.
        integration
            .call_tool(&mut ai, &context, "frame_view", json!({"target":"all"}))
            .expect("frame all");
        assert!(desktop.frame_objects(&[0, 1]));
        assert_eq!(ai.camera.state(), desktop.camera.state());

        // A hidden object is selected but the camera stays where it is, on
        // both sides.
        integration
            .call_tool(
                &mut ai,
                &context,
                "set_visibility",
                json!({"index":0,"visible":false}),
            )
            .expect("hide");
        let still = ai.camera.state();
        assert!(!ai.centre_on_object(0));
        assert_eq!(ai.camera.state(), still);
        assert_eq!(
            ai.commands
                .scene()
                .selection()
                .iter()
                .copied()
                .collect::<Vec<_>>(),
            [0]
        );
        integration
            .call_tool(
                &mut ai,
                &context,
                "frame_view",
                json!({"target":"selected"}),
            )
            .expect("frame hidden selection");
        assert_eq!(ai.camera.state(), still);

        drop(integration);
        let _ = fs::remove_dir_all(root);
    }
}
