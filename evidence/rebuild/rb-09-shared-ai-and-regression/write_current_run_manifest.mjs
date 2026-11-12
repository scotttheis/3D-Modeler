import { createHash } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";

const runRoot = resolve(process.argv[2] ?? "");
if (!process.argv[2]) {
  throw new Error("Usage: node write_current_run_manifest.mjs <current-run-directory>");
}

const readBytes = (relativePath) => readFileSync(join(runRoot, relativePath));
const readText = (relativePath) =>
  readBytes(relativePath).toString("utf8").replace(/^\uFEFF/u, "");
const readJson = (relativePath) => JSON.parse(readText(relativePath));
const readJsonLines = (relativePath) =>
  readText(relativePath)
    .split(/\r?\n/u)
    .filter(Boolean)
    .map((line) => JSON.parse(line));
const fileInfo = (relativePath) => {
  const bytes = readBytes(relativePath);
  return {
    path: relativePath.replaceAll("\\", "/"),
    bytes: bytes.length,
    sha256: createHash("sha256").update(bytes).digest("hex"),
  };
};
const imageInfo = (relativePath, purpose, disposition, reason) => {
  const bytes = readBytes(relativePath);
  if (bytes.toString("ascii", 1, 4) !== "PNG") {
    throw new Error(`${relativePath} is not a PNG capture`);
  }
  return {
    ...fileInfo(relativePath),
    width: bytes.readUInt32BE(16),
    height: bytes.readUInt32BE(20),
    purpose,
    disposition,
    reason,
  };
};

const profilePairs = [
  ["observe", "python-observe.json", "rust-observe-live.json"],
  ["model", "python-model.json", "rust-model-live.json"],
  ["files", "python-files.json", "rust-files-live.json"],
  ["full", "python-full-live.json", "rust-full-live.json"],
];
const profileResults = profilePairs.map(([profile, pythonFile, rustFile]) => {
  const pythonTools = readJson(`catalogs/${pythonFile}`).tools;
  const rustTools = readJson(`catalogs/${rustFile}`).tools;
  return {
    profile,
    python_count: pythonTools.length,
    rust_count: rustTools.length,
    ordered_names_equal:
      JSON.stringify(pythonTools.map((tool) => tool.name)) ===
      JSON.stringify(rustTools.map((tool) => tool.name)),
  };
});

const reopenEvents = readJsonLines("logs/rust-current-live-reopen-client.jsonl");
const finalStatus = reopenEvents
  .filter(
    (event) =>
      event.event === "tool_result" &&
      event.name === "get_status" &&
      event.structured_content?.project_id ===
        "e5d81c5226e64f58b838d55d9395afa9",
  )
  .at(-1)?.structured_content;
const reopenedScene = reopenEvents
  .filter(
    (event) =>
      event.event === "tool_result" &&
      event.name === "scene_summary" &&
      event.structured_content?.project_id ===
        "e5d81c5226e64f58b838d55d9395afa9",
  )
  .at(-1)?.structured_content;
if (!finalStatus || !reopenedScene) {
  throw new Error("The current Release GUI reopen result is missing from its SDK log");
}

const optionalEvents = readJsonLines(
  "logs/rust-current-optional-failure-client.jsonl",
);
const optionalResults = optionalEvents
  .filter((event) => event.event === "tool_result")
  .map((event) => ({
    tool: event.name,
    is_error: event.is_error,
    code: event.structured_content?.code ?? null,
    retryable: event.structured_content?.retryable ?? null,
  }));
const startup = readJson("logs/rust-current-optional-startup.json");
const savedProject = fileInfo("fixtures/approved/rust-current-live-saved.3dp");

const manifest = {
  run_id: "currpair-20261106-072716",
  recorded_utc: "2026-11-06",
  scope:
    "Current Rust Release desktop/MCP workflow and optional-bridge failure check. The Python Builder3D product was not launched; the official Python MCP SDK was used only as a client for the Rust stdio gateway.",
  rust_release: {
    app_path: "target/release/builder3d-app.exe",
    app_sha256:
      "0bc4f1a0fd0327e51ecd8acdb75a01120b3c53325b11738a7b3fb6ce198cc75a",
    gateway_path: "target/release/r4_mcp.exe",
    gateway_sha256:
      "aff0af548a3f8055e2ebe1d218135a774cc9b88f210a94f19627a9f9b22e1c1e",
  },
  client: {
    sdk: "official Python MCP SDK 2.2.0",
    role: "stdio client to the Rust r4_mcp.exe gateway only",
    python_product_launched: false,
  },
  profiles: profileResults,
  saved_project: {
    ...savedProject,
    project_id: finalStatus.project_id,
    scene_revision: finalStatus.scene_revision,
    dirty: finalStatus.dirty,
    objects: reopenedScene.objects.map((object) => ({
      name: object.name,
      object_id: object.object_id,
      vertices: object.vertices,
      triangles: object.triangles,
      position_mm: object.position_mm,
      size_mm: object.size_mm,
    })),
  },
  lifecycle: {
    paired_release_app_pid: 28536,
    paired_release_app_exited_normally: true,
    paired_release_gateway_exited_after_client_close: true,
    gui_reopened_saved_path:
      "fixtures/approved/rust-current-live-saved.3dp",
    gui_reopen_title: "Builder3D - rust-current-live-saved.3dp",
    gui_reopen_status: finalStatus,
    normal_shutdown_descriptor_removed: true,
    normal_shutdown_listener_removed: true,
    optional_failure_app_pid: 5400,
    optional_failure_app_exited_normally: true,
    optional_failure_runtime_parent_is_existing_file: true,
    optional_failure_descriptor_created: false,
    optional_failure_app_owned_listener_count: 0,
    optional_failure_sdk_tool_count: 60,
    optional_failure_tool_results: optionalResults,
    optional_failure_client_closed_normally: true,
    optional_failure_gateway_exited: true,
    optional_failure_runtime_descriptor_absent_after_shutdown: true,
    optional_failure_listener_absent_after_shutdown: true,
    blocker_fixture_unchanged:
      startup.blocker_sha256 ===
      "C19715ED3F027242D1D6208C6B85C894C3BA06A57CF81F61B5A32060A3F5BAC4",
  },
  evidence_files: [
    fileInfo("logs/rust-current-live.jsonl"),
    fileInfo("logs/rust-current-live-reopen-client.jsonl"),
    fileInfo("logs/rust-current-two-client-race.json"),
    fileInfo("logs/rust-current-optional-startup.json"),
    fileInfo("logs/rust-current-optional-failure-client.jsonl"),
    fileInfo("schema-comparison.json"),
  ],
  captures: [
    imageInfo(
      "captures/rust-current-live-gui-open-dialog-retry.png",
      "Open Project dialog on the current Release app",
      "accepted",
      "Shows the current Release GUI open workflow before path entry.",
    ),
    imageInfo(
      "captures/rust-current-live-gui-open-path-retry2.png",
      "saved project path entered into the Rust Open Project dialog",
      "intermediate",
      "The path is visible in the field; the final reopen is established by the later clean GUI and MCP results.",
    ),
    imageInfo(
      "captures/rust-current-live-gui-reopened-after-restart-final.png",
      "saved project after Rust Release GUI reopen",
      "accepted",
      "Foreground PID 28536, title matches the saved .3dp, and the viewport and Items panel show the saved Cube and Sphere.",
    ),
    imageInfo(
      "captures/rust-current-optional-startup-unavailable.png",
      "optional MCP startup failure with desktop still at Welcome",
      "accepted",
      "Rust Release status bar reports MCP unavailable; the app remained responsive and had no descriptor or listener.",
    ),
    imageInfo(
      "captures/rust-current-optional-gui-new-scene-retry.png",
      "New Scene through the Rust GUI while MCP startup is unavailable",
      "accepted",
      "Workspace opened and remained interactive with an empty scene.",
    ),
    imageInfo(
      "captures/rust-current-optional-gui-added-primitive.png",
      "Cube added through the Rust GUI while MCP startup is unavailable",
      "accepted",
      "The viewport and Items panel show the new Cube; the document became dirty.",
    ),
    imageInfo(
      "captures/rust-current-optional-gui-undo.png",
      "GUI undo after optional-MCP GUI edit",
      "accepted",
      "Undo removed the Cube and returned the new scene to a clean empty state.",
    ),
    imageInfo(
      "captures/rust-current-live-gui-reopened-after-restart-retry2.png",
      "first reopen screenshot attempt",
      "rejected",
      "Another foreground window covered Builder3D during capture; do not use it as Builder3D visual evidence.",
    ),
    imageInfo(
      "captures/rust-current-optional-gui-new-scene.png",
      "first New Scene click attempt under optional MCP failure",
      "rejected",
      "The screen-coordinate offset was wrong and the click missed the New Scene card; the corrected retry is accepted.",
    ),
  ],
  schema_comparison_path: "schema-comparison.json",
  acceptance_boundary: {
    tool_names_and_order: "verified for observe/model/files/full profiles",
    schema_parity: "open; differences are documented, not owner-approved",
    visual_review: "pending",
    owner_acceptance: "pending",
    interface_and_floor_gaps: "pending",
    rb08_native_matrix_and_completion_snapshot: "pending",
    overall_product_acceptance: "pending",
    rb10_started: false,
  },
};

const outputPath = join(runRoot, "current-run-manifest.json");
writeFileSync(outputPath, `${JSON.stringify(manifest, null, 2)}\n`, "utf8");
console.log(
  JSON.stringify(
    {
      output: outputPath,
      profile_results: manifest.profiles,
      saved_project: {
        path: manifest.saved_project.path,
        bytes: manifest.saved_project.bytes,
        sha256: manifest.saved_project.sha256,
        project_id: manifest.saved_project.project_id,
        scene_revision: manifest.saved_project.scene_revision,
        object_count: manifest.saved_project.objects.length,
      },
      capture_count: manifest.captures.length,
      rejected_capture_count: manifest.captures.filter(
        (capture) => capture.disposition === "rejected",
      ).length,
    },
    null,
    2,
  ),
);
