"""Validate RB-09 TOMLs, evidence references, live captures, manifests, and journals."""

from __future__ import annotations

import hashlib
import json
import re
import struct
import tomllib
from pathlib import Path


ROOT = Path(__file__).resolve().parents[3]
EVIDENCE = ROOT / "evidence" / "rebuild" / "rb-09-shared-ai-and-regression"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


catalog_path = ROOT / "docs" / "component-catalog.toml"
parity_path = ROOT / "evidence" / "rebuild" / "parity-matrix.toml"
catalog = tomllib.loads(catalog_path.read_text(encoding="utf-8"))
parity = tomllib.loads(parity_path.read_text(encoding="utf-8"))

components = catalog["component"]
component_ids = [component["id"] for component in components]
require(len(component_ids) == len(set(component_ids)), "duplicate component IDs")
component_by_id = {component["id"]: component for component in components}
feature = next(
    row for row in parity["feature"] if row["id"] == "builder3d.mcp.catalog_and_parity"
)
require(feature["implementation"] == "pass", "RB-09 implementation status is not pass")
require(feature["automated"] == "pass", "RB-09 automated status is not pass")
require(feature["interactive"] == "pass", "RB-09 live interactive status is not pass")
require(feature["visual"] == "pending", "visual review must remain pending")
require(feature["owner_acceptance"] == "pending", "owner acceptance must remain pending")
require(len(feature["visual_capture_pair"]) == 4, "expected Light/Dark capture pairs")
require(
    "builder3d.mcp.stdio-gateway" in feature["rust_component_ids"],
    "stdio gateway component missing from parity row",
)
for component_id in feature["rust_component_ids"]:
    require(component_id in component_by_id, f"unknown parity component: {component_id}")

for raw_path in feature["evidence"]:
    path = raw_path.split("::", 1)[0]
    require((ROOT / path).is_file(), f"missing parity evidence path: {path}")

for component_id in (
    "builder3d.mcp.desktop-bridge",
    "builder3d.mcp.stdio-gateway",
    "builder3d.mcp.server",
):
    component = component_by_id[component_id]
    for field in ("rust_paths", "tests", "evidence"):
        for raw_path in component.get(field, []):
            path = raw_path.split("::", 1)[0]
            require((ROOT / path).is_file(), f"missing {component_id} {field}: {path}")

required_evidence = (
    "report.md",
    "verification.md",
    "checkpoint.md",
    "architecture-decision.md",
    "requirement-test-map.md",
    "walkthrough.md",
    "paired-comparison-board.md",
    "capture-metadata.json",
    "accepted-journal.ndjson",
    "rejected-journal.ndjson",
    "client-server-log.md",
    "build-manifest.json",
    "fixture-manifest.json",
    "source-preservation.md",
    "snapshot-receipt.md",
    "artifact-identity-recheck.json",
    "artifact-identity-recheck/current-app.png",
    "artifact-identity-recheck/client.jsonl",
    "logs/identity-recheck-bloat-workspace.stderr.log",
    "logs/identity-recheck-restore-release-app.stderr.log",
    "validate_rb09_evidence.py",
)
for name in required_evidence:
    require((EVIDENCE / name).is_file(), f"missing required evidence: {name}")

json_names = (
    "capture-metadata.json",
    "build-manifest.json",
    "fixture-manifest.json",
    "rust_stdio_client_probe-result.json",
    "artifact-identity-recheck.json",
)
parsed_json = {
    name: json.loads((EVIDENCE / name).read_text(encoding="utf-8-sig"))
    for name in json_names
}
capture = parsed_json["capture-metadata.json"]
require(capture["host_preflight"]["lockapp_owned_session"] is False, "LockApp owned capture session")
require(capture["host_preflight"]["power_or_lock_settings_changed"] is False, "power/lock settings changed")
require(capture["visual_review"] == "pending", "visual review must remain pending")
require(capture["owner_acceptance"] == "pending", "owner acceptance must remain pending")
captures = {entry["file"]: entry for entry in capture["native_captures"]}
require(len(captures) == capture["native_capture_count"] == 50, "capture inventory count mismatch")
require(len(capture["paired_capture_sets"]) == 2, "expected exactly two clean scene pairs")
for entry in capture["native_captures"]:
    path = EVIDENCE / entry["file"]
    require(path.is_file(), f"missing capture: {entry['file']}")
    data = path.read_bytes()
    require(digest(path) == entry["sha256"], f"capture SHA-256 mismatch: {entry['file']}")
    require(len(data) == entry["bytes"], f"capture byte count mismatch: {entry['file']}")
    require(data[:8] == b"\x89PNG\r\n\x1a\n", f"not a PNG capture: {entry['file']}")
    require(
        list(struct.unpack(">II", data[16:24])) == entry["png_dimensions_px"],
        f"capture dimensions mismatch: {entry['file']}",
    )
    require(entry["host_desktop_interactive"] is True, f"capture was not interactive: {entry['file']}")

for pair in capture["paired_capture_sets"]:
    python_capture = captures.get(pair["python_capture"])
    rust_capture = captures.get(pair["rust_capture"])
    require(python_capture is not None and rust_capture is not None, "pair references missing capture")
    require(python_capture["accepted_for_live_gate"] and rust_capture["accepted_for_live_gate"],
            "pair includes a capture not accepted for the live gate")
    require(python_capture["theme"] == rust_capture["theme"] == pair["theme"],
            "pair theme mismatch")
    require(python_capture["project_id"] == rust_capture["project_id"] == pair["project_id"],
            "pair project identity mismatch")
for raw_path in feature["visual_capture_pair"]:
    path = raw_path.removeprefix("evidence/rebuild/rb-09-shared-ai-and-regression/")
    require(path in captures and captures[path]["accepted_for_live_gate"],
            f"parity row references a non-accepted paired capture: {raw_path}")

probe = parsed_json["rust_stdio_client_probe-result.json"]
require(probe["tool_count"] == 60, "client probe tool count differs from 60")
require(probe["closed_desktop_call"]["code"] == "desktop_unavailable", "closed desktop code mismatch")
manifest = parsed_json["build-manifest.json"]
for binary in manifest["builds"]:
    if binary.get("sha256"):
        require(re.fullmatch(r"[0-9a-f]{64}", binary["sha256"]) is not None, "invalid binary SHA-256")
# These build-manifest hashes identify the historical paired gate. Current
# target/release outputs are checked against the separate identity recheck below.
builds_by_role = {binary["role"]: binary for binary in manifest["builds"]}
live_app_build = builds_by_role["live Rust app"]
live_gateway_build = builds_by_role["live Rust MCP gateway during paired sequence"]
post_run_gateway_build = builds_by_role["current post-run Rust MCP gateway"]
require(capture["build_identity"]["rust_app_sha256"] == live_app_build["sha256"],
        "historical live app hash differs from capture metadata")
require(capture["build_identity"]["rust_live_gateway_sha256"] == live_gateway_build["sha256"],
        "historical live gateway hash differs from capture metadata")
require(post_run_gateway_build["sha256"] == manifest["post_run_rust_gateway_sha256"],
        "post-run gateway hash differs from build manifest summary")

source_files = sorted((ROOT / "crates").rglob("*.rs"), key=lambda path: path.relative_to(ROOT).as_posix())
source_hash = hashlib.sha256()
for path in source_files:
    source_hash.update(path.relative_to(ROOT).as_posix().encode("utf-8") + b"\0")
    source_hash.update(bytes.fromhex(digest(path)) + b"\0")
require(len(source_files) == manifest["source_tree"]["rust_source_file_count"],
        "Rust source file count mismatch")
require(source_hash.hexdigest() == manifest["source_tree"]["rust_source_manifest_sha256"],
        "Rust source manifest hash mismatch")

identity = parsed_json["artifact-identity-recheck.json"]
require(identity["schema_version"] == 1, "unsupported artifact identity recheck schema")
require(identity["slice"] == "RB-09", "artifact identity recheck slice mismatch")
require(identity["source_tree"]["rust_source_file_count"] == len(source_files),
        "identity recheck Rust source file count mismatch")
require(identity["source_tree"]["rust_source_manifest_sha256"] == source_hash.hexdigest(),
        "identity recheck Rust source manifest mismatch")
require(identity["source_tree"]["rust_application_source_changed"] is False,
        "application source changed during artifact identity recheck")
require(identity["historical_full_gate"]["live_rust_app_sha256"] == live_app_build["sha256"],
        "identity recheck historical app hash differs from build manifest")
require(identity["historical_full_gate"]["live_rust_gateway_sha256"] == live_gateway_build["sha256"],
        "identity recheck historical gateway hash differs from build manifest")
require(identity["historical_full_gate"]["post_run_rust_gateway_sha256"] == post_run_gateway_build["sha256"],
        "identity recheck post-run gateway hash differs from build manifest")
require(identity["historical_full_gate"]["accepted_native_capture_count"] == len(captures) == 50,
        "historical full-gate capture count differs from capture inventory")
require(identity["historical_full_gate"]["full_paired_matrix_repeated"] is False,
        "identity recheck must not claim a paired matrix rerun")
require(identity["previous_wrap_up_failure"]["exit_code"] == 1,
        "previous stale-path validator failure is not recorded")
require(identity["previous_wrap_up_failure"]["error"] == "release binary hash mismatch: live Rust app",
        "previous stale-path validator error differs from the recorded failure")

bloat_log_path = EVIDENCE / identity["mismatch_explanation"]["workspace_bloat_evidence_copy"]
restore_log_path = EVIDENCE / identity["mismatch_explanation"]["app_restore_evidence_copy"]
require(digest(bloat_log_path) == identity["mismatch_explanation"]["workspace_bloat_evidence_sha256"],
        "workspace bloat rebuild log hash mismatch")
require(bloat_log_path.stat().st_size == identity["mismatch_explanation"]["workspace_bloat_evidence_bytes"],
        "workspace bloat rebuild log size mismatch")
require(digest(restore_log_path) == identity["mismatch_explanation"]["app_restore_evidence_sha256"],
        "app restore rebuild log hash mismatch")
require(restore_log_path.stat().st_size == identity["mismatch_explanation"]["app_restore_evidence_bytes"],
        "app restore rebuild log size mismatch")
bloat_log = bloat_log_path.read_text(encoding="utf-8")
restore_log = restore_log_path.read_text(encoding="utf-8")
require("--workspace" in bloat_log and "unused arguments" in bloat_log,
        "workspace bloat CLI warning missing from rebuild log")
require("Compiling builder3d-mcp" in bloat_log and "Compiling builder3d-app" in bloat_log,
        "workspace bloat rebuild log does not show app and gateway rebuilds")
require("Compiling builder3d-app" in restore_log and "Finished" in restore_log,
        "app restore rebuild log does not show a completed app rebuild")

expected_release_artifacts = (
    ("rust_app", "target/release/builder3d-app.exe"),
    ("rust_mcp_gateway", "target/release/r4_mcp.exe"),
)
for artifact_name, expected_relative_path in expected_release_artifacts:
    artifact = identity["release_artifacts"][artifact_name]
    expected_path = ROOT / expected_relative_path
    require(artifact["path"] == expected_relative_path,
            f"unexpected current release artifact path: {artifact_name}")
    require(expected_path.is_file(), f"missing current release binary: {expected_path}")
    require(digest(expected_path) == artifact["sha256"],
            f"current release binary hash mismatch: {artifact_name}")
    require(expected_path.stat().st_size == artifact["bytes"],
            f"current release binary size mismatch: {artifact_name}")
    require(all(value == artifact["sha256"] for value in artifact["hash_observations"].values()),
            f"current release binary identity changed during probe: {artifact_name}")

require(identity["previous_wrap_up_failure"]["app_hash_at_failure"]
        == identity["release_artifacts"]["rust_app"]["sha256"],
        "previous wrap-up failure app hash differs from the current identity recheck")
require(identity["previous_wrap_up_failure"]["gateway_hash_at_failure"]
        == identity["release_artifacts"]["rust_mcp_gateway"]["sha256"],
        "previous wrap-up failure gateway hash differs from the current identity recheck")
app_process = identity["release_artifacts"]["rust_app"]["process"]
require(app_process["pid"] == identity["cleanup"]["app_pid"], "app process ID mismatch")
require(Path(app_process["executable_path"]).resolve()
        == (ROOT / "target/release/builder3d-app.exe").resolve(),
        "app process path differs from the current Release app path")
require(app_process["exited_after_close"] is True, "identity recheck app process did not close")
require(identity["release_artifacts"]["rust_mcp_gateway"]["gateway_process_exited_after_client_close"] is True,
        "identity recheck gateway process did not close")
require(identity["cleanup"]["app_process_exited"] is True,
        "identity recheck app process remains running")
require(identity["cleanup"]["rust_builder_process_count_after_shutdown"] == 0,
        "Rust Builder3D process remained after identity recheck")
require(identity["cleanup"]["runtime_descriptor_removed"] is True,
        "identity recheck runtime descriptor was not removed")

host = identity["host"]
require(host["input_desktop"] == "Default", "identity screenshot was not on the Default desktop")
require(host["session_state"] == 0, "identity screenshot session was not active")
require(host["lockapp_owned_session"] is False, "LockApp owned the identity recheck session")
require(host["foreground_process"] == "builder3d-app", "identity screenshot app was not foreground")
require(host["owner_video_background_only"] is True,
        "owner video was not kept in the background during the identity check")
require(host["power_or_lock_settings_changed"] is False, "power/lock settings changed during recheck")
require(identity["identity_capture"]["host_desktop_interactive"] is True,
        "identity screenshot was not captured on an interactive desktop")
require(identity["identity_capture"]["role"] == "identity_only_not_a_live_gate_pair",
        "identity screenshot is mislabelled as paired live-gate evidence")
require(identity["scope"]["full_paired_workflow_repeated"] is False,
        "identity recheck must not claim a full paired workflow")
require(identity["scope"]["accepted_capture_inventory_changed"] is False,
        "identity recheck changed the accepted capture inventory")
require(identity["scope"]["visual_review_or_owner_approval"] is False,
        "identity recheck must not claim owner visual approval")
require(identity["scope"]["overall_product_acceptance"] is False,
        "identity recheck must not claim overall acceptance")

identity_capture = identity["identity_capture"]
require(identity_capture["png_dimensions_px"] == host["outer_window_px"],
        "identity screenshot dimensions differ from the recorded window")
require(identity_capture["foreground_title"] == host["window_title"],
        "identity screenshot title differs from host preflight")
identity_capture_path = EVIDENCE / identity_capture["file"]
require(identity_capture_path.is_file(), "missing identity-only screenshot")
identity_capture_data = identity_capture_path.read_bytes()
require(digest(identity_capture_path) == identity_capture["sha256"],
        "identity-only screenshot SHA-256 mismatch")
require(len(identity_capture_data) == identity_capture["bytes"],
        "identity-only screenshot byte count mismatch")
require(identity_capture_data[:8] == b"\x89PNG\r\n\x1a\n",
        "identity-only screenshot is not a PNG")
require(list(struct.unpack(">II", identity_capture_data[16:24])) == identity_capture["png_dimensions_px"],
        "identity-only screenshot dimensions mismatch")

mcp_probe = identity["mcp_probe"]
require(mcp_probe["client"] == "official Python MCP SDK" and mcp_probe["sdk_version"] == "2.2.0",
        "identity-only MCP client/version mismatch")
require(mcp_probe["profile"] == "full", "identity-only MCP profile mismatch")
identity_client_log_path = EVIDENCE / mcp_probe["client_log"]
require(identity_client_log_path.is_file(), "missing identity-only MCP client log")
require(digest(identity_client_log_path) == mcp_probe["client_log_sha256"],
        "identity-only MCP client log SHA-256 mismatch")
require(identity_client_log_path.stat().st_size == mcp_probe["client_log_bytes"],
        "identity-only MCP client log byte count mismatch")
identity_client_events = [
    json.loads(line)
    for line in identity_client_log_path.read_text(encoding="utf-8").splitlines()
    if line.strip()
]
require([event.get("event") for event in identity_client_events] == ["ready", "tool_result", "closing"],
        "identity-only client log contains unexpected calls or event order")
ready_event = identity_client_events[0]
require(ready_event["client"] == "official Python MCP SDK",
        "identity-only client is not the official Python MCP SDK")
require(ready_event["tool_count"] == mcp_probe["tool_count"] == 60,
        "identity-only client tool count differs from 60")
require(len(ready_event["tools"]) == 60 and len(set(ready_event["tools"])) == 60,
        "identity-only client tool list is incomplete or duplicated")
status_event = identity_client_events[1]
require(status_event["name"] == "get_status" and status_event["is_error"] is False,
        "identity-only get_status call failed")
status_result = status_event["structured_content"]
for field in ("app_state", "profile", "dirty", "scene_revision", "project_id"):
    require(status_result[field] == mcp_probe["result"][field],
            f"identity-only get_status result mismatch: {field}")
require(status_result["app_state"] == "ready", "identity-only app is not ready")
require(status_result["profile"] == "full", "identity-only MCP profile mismatch")
require(status_result["dirty"] is False, "identity-only probe changed the document")
require(status_result["scene_revision"] == 0, "identity-only probe changed the scene")
require(mcp_probe["called_tool"] == "get_status" and mcp_probe["is_error"] is False,
        "identity-only MCP operation is not a successful read-only status call")
require(mcp_probe["mutations_performed"] == 0, "identity-only MCP probe performed mutations")

fixtures = parsed_json["fixture-manifest.json"]
for fixture in fixtures["files"]:
    path = EVIDENCE / fixture["path"]
    require(path.is_file(), f"missing fixture: {fixture['path']}")
    require(path.stat().st_size == fixture["bytes"], f"fixture byte count mismatch: {fixture['path']}")
    require(digest(path) == fixture["sha256"], f"fixture hash mismatch: {fixture['path']}")

for journal_name in ("accepted-journal.ndjson", "rejected-journal.ndjson"):
    seen: set[str] = set()
    for line_number, line in enumerate(
        (EVIDENCE / journal_name).read_text(encoding="utf-8").splitlines(), start=1
    ):
        if not line.strip():
            continue
        record = json.loads(line)
        evidence_id = record["evidence_id"]
        require(evidence_id not in seen, f"duplicate {journal_name} ID: {evidence_id}")
        seen.add(evidence_id)
    require(seen, f"empty evidence journal: {journal_name}")

print(f"TOML valid: {catalog_path.relative_to(ROOT)}, {parity_path.relative_to(ROOT)}")
print(f"component IDs unique: {len(component_ids)}; RB-09 evidence references exist")
print(f"historical paired live-gate build identities match capture metadata; {len(captures)} accepted PNG hashes/dimensions and 2 clean scene pairs verified")
print(f"fixture hashes verified: {len(fixtures['files'])}; Rust source manifest verified: {len(source_files)} files")
print(f"current Release identity verified: app {identity['release_artifacts']['rust_app']['sha256']}; gateway {identity['release_artifacts']['rust_mcp_gateway']['sha256']}")
print("identity-only official-client probe verified: 60 tools; get_status app_state=ready; 1296x879 interactive screenshot")
print("JSON manifests, rebuild logs, official-client results, and accepted/rejected NDJSON journals valid")
