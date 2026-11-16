"""Validate RB-09 TOMLs, evidence references, live captures, manifests, and journals."""

from __future__ import annotations

import hashlib
import json
import re
import struct
import sys
import tomllib
from pathlib import Path


ROOT = Path(__file__).resolve().parents[3]
EVIDENCE = ROOT / "evidence" / "rebuild" / "rb-09-shared-ai-and-regression"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def established_tool_names() -> list[str]:
    source_path = ROOT / "crates" / "modeler3d-mcp" / "src" / "r4" / "catalog.rs"
    source = source_path.read_text(encoding="utf-8")
    catalog = re.search(
        r"pub const TOOL_CATALOG: \[ToolSpec;\s*60\]\s*=\s*\[(.*?)\n\];",
        source,
        re.DOTALL,
    )
    require(catalog is not None, "Rust MCP TOOL_CATALOG declaration is missing")
    names = re.findall(
        r'^[ \t]*name:[ \t]*"([^"]+)",[ \t]*$',
        catalog.group(1),
        re.MULTILINE,
    )
    require(len(names) == 60, "Rust MCP TOOL_CATALOG does not contain 60 ordered names")
    require(len(set(names)) == 60, "Rust MCP TOOL_CATALOG contains duplicate names")
    return names


def ordered_tool_names_match(actual: list[str], expected: list[str]) -> bool:
    return actual == expected


def check_tool_name_regression(expected: list[str]) -> None:
    require(
        ordered_tool_names_match(expected, expected),
        "tool-name regression check rejected the established catalog",
    )
    substituted = expected.copy()
    substituted[1] = f"{substituted[1]}_renamed_for_regression"
    require(
        not ordered_tool_names_match(substituted, expected),
        "tool-name regression check accepted a renamed tool",
    )


if __name__ == "__main__" and sys.argv[1:] == ["--tool-name-self-test"]:
    tool_names = established_tool_names()
    check_tool_name_regression(tool_names)
    print(
        "tool-name regression passed: ordered list accepted and renamed tool "
        f"rejected ({len(tool_names)} names)"
    )
    raise SystemExit(0)


catalog_path = ROOT / "docs" / "component-catalog.toml"
parity_path = ROOT / "evidence" / "rebuild" / "parity-matrix.toml"
catalog = tomllib.loads(catalog_path.read_text(encoding="utf-8"))
parity = tomllib.loads(parity_path.read_text(encoding="utf-8"))

components = catalog["component"]
component_ids = [component["id"] for component in components]
require(len(component_ids) == len(set(component_ids)), "duplicate component IDs")
component_by_id = {component["id"]: component for component in components}
feature = next(
    row for row in parity["feature"] if row["id"] == "modeler3d.mcp.catalog_and_parity"
)
current_run_directory = EVIDENCE / "currpair-20261106-072716"
current_run_manifest_path = current_run_directory / "current-run-manifest.json"
historical_schema_comparison_path = current_run_directory / "schema-comparison.json"
schema_parity_directory = EVIDENCE / "currpair-20261107-093804-schema-parity"
schema_comparison_path = schema_parity_directory / "schema-property-diff-current.json"
schema_parity_manifest_path = EVIDENCE / "schema-parity-run-manifest.json"
require(current_run_manifest_path.is_file(), "missing current Rust Release run manifest")
require(historical_schema_comparison_path.is_file(), "missing historical Rust schema comparison")
require(schema_comparison_path.is_file(), "missing current-reference schema comparison")
require(schema_parity_manifest_path.is_file(), "missing schema-parity run manifest")
current_run = json.loads(current_run_manifest_path.read_text(encoding="utf-8"))
historical_schema_comparison = json.loads(
    historical_schema_comparison_path.read_text(encoding="utf-8")
)
schema_comparison = json.loads(schema_comparison_path.read_text(encoding="utf-8"))
schema_parity_manifest = json.loads(
    schema_parity_manifest_path.read_text(encoding="utf-8")
)
require(current_run["client"]["python_product_launched"] is False,
        "current recheck must not claim the Python product was launched")
require(historical_schema_comparison["saved_python_catalogs_refreshed_in_this_run"] is False,
        "historical schema comparison must retain its saved-catalog scope")
schema_profiles = schema_comparison["profiles"]
run_profiles = {profile["profile"]: profile for profile in current_run["profiles"]}
require(set(schema_profiles) == set(run_profiles) == {"observe", "model", "files", "full"},
        "current-reference Rust profile set mismatch")
expected_profile_counts = {"observe": 7, "model": 52, "files": 59, "full": 60}
for profile_name, schema_profile in schema_profiles.items():
    run_profile = run_profiles[profile_name]
    expected_count = expected_profile_counts[profile_name]
    require(schema_profile["expected_tool_count"] == expected_count
            and schema_profile["reference_tool_count"] == expected_count
            and schema_profile["rust_tool_count"] == expected_count,
            f"current-reference tool count mismatch for {profile_name}")
    require(schema_profile["ordered_names_match"] is True
            and run_profile["ordered_names_equal"] is True,
            f"current ordered tool names differ for {profile_name}")
    require(schema_profile["input_schema_matches"] == expected_count
            and schema_profile["output_schema_matches"] == expected_count
            and schema_profile["annotation_matches"] == expected_count
            and schema_profile["recursive_difference_count"] == 0
            and schema_profile["exact_schema_and_annotation_parity"] is True,
            f"current nested schema/annotation parity differs for {profile_name}")
for capture in current_run["captures"]:
    path = current_run_directory / capture["path"]
    require(path.is_file(), f"missing current-run capture: {capture['path']}")
    data = path.read_bytes()
    require(len(data) == capture["bytes"],
            f"current-run capture byte count mismatch: {capture['path']}")
    require(digest(path) == capture["sha256"],
            f"current-run capture SHA-256 mismatch: {capture['path']}")
    require(data[:8] == b"\x89PNG\r\n\x1a\n",
            f"current-run capture is not a PNG: {capture['path']}")
    require(list(struct.unpack(">II", data[16:24])) == [capture["width"], capture["height"]],
            f"current-run capture dimensions mismatch: {capture['path']}")
current_schema_matches = all(
    profile["exact_schema_and_annotation_parity"] is True
    and profile["recursive_difference_count"] == 0
    for profile in schema_profiles.values()
)
require(current_schema_matches, "latest current-reference schema parity is not complete")
require(feature["implementation"] == "pass",
        "RB-09 catalog/schema implementation status must reflect current parity")
require(feature["automated"] == "pass",
        "RB-09 catalog/schema automation status must reflect current parity")
require("schema-parity-current-reference" in component_by_id["modeler3d.mcp.stdio-gateway"]["contracts"],
        "gateway component must record current-reference schema parity")
require(str(current_run["acceptance_boundary"]["schema_parity"]).startswith("open"),
        "historical 2026-11-06 run must retain its open schema status")
require(schema_parity_manifest["completion_state"]["schema_parity"] == "pass",
        "schema-parity run manifest must report the verified schema criterion")
require(schema_parity_manifest["completion_state"]["rb09_acceptance"] == "open"
        and schema_parity_manifest["completion_state"]["rb10_started"] is False,
        "schema-parity run must keep overall RB-09 open and stop before RB-10")
require(schema_comparison["custom_metadata_difference_count"] == 3,
        "custom metadata difference must remain disclosed separately")
require(schema_parity_manifest["owner_approved_deviation_claimed"] is False,
        "schema-parity evidence must not claim owner approval")
result_log_path = ROOT / schema_parity_manifest["result_validation"]["live_result_log"]
require(result_log_path.is_file(), "missing live Release result log")
result_events = [
    json.loads(line)
    for line in result_log_path.read_text(encoding="utf-8").splitlines()
    if line.strip()
]
successful_results = [
    event for event in result_events
    if event.get("event") == "tool_result" and event.get("is_error") is False
]
require(len(successful_results)
        == schema_parity_manifest["result_validation"]["successful_structured_result_count"],
        "live result success count differs from schema-parity manifest")
result_validation_log = ROOT / schema_parity_manifest["result_validation"]["validation_test_log"]
require(result_validation_log.is_file(), "missing captured Release result-validation test log")
require("test result: ok. 1 passed; 0 failed" in result_validation_log.read_text(encoding="utf-8"),
        "captured Release result-validation test did not pass")
require(feature["interactive"] == "pass", "RB-09 live interactive status is not pass")
require(feature["visual"] == "pending", "visual review must remain pending")
require(feature["owner_acceptance"] == "pending", "owner acceptance must remain pending")
require(len(feature["visual_capture_pair"]) == 4, "expected Light/Dark capture pairs")
require(
    "modeler3d.mcp.stdio-gateway" in feature["rust_component_ids"],
    "stdio gateway component missing from parity row",
)
for component_id in feature["rust_component_ids"]:
    require(component_id in component_by_id, f"unknown parity component: {component_id}")

for raw_path in feature["evidence"]:
    path = raw_path.split("::", 1)[0]
    require((ROOT / path).is_file(), f"missing parity evidence path: {path}")
for raw_path in feature.get("schema_evidence", []):
    path = raw_path.split("::", 1)[0]
    require((ROOT / path).is_file(), f"missing parity schema evidence path: {path}")

for component_id in (
    "modeler3d.mcp.desktop-bridge",
    "modeler3d.mcp.stdio-gateway",
    "modeler3d.mcp.server",
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
current_source_tree = schema_parity_manifest["rust_source_trees"]["current"]
require(len(source_files) == current_source_tree["file_count"],
        "current Rust source file count mismatch")
require(source_hash.hexdigest() == current_source_tree["manifest_sha256"],
        "current Rust source manifest hash mismatch")

pre_edit_source_path = ROOT / schema_parity_manifest["rust_source_trees"]["pre_edit"]["manifest_path"]
require(pre_edit_source_path.is_file(), "missing pre-edit Rust source manifest")
pre_edit_source_tree = json.loads(pre_edit_source_path.read_text(encoding="utf-8"))
pre_edit_records = pre_edit_source_tree["files"]
pre_edit_paths = [record["path"] for record in pre_edit_records]
require(pre_edit_paths == sorted(pre_edit_paths) and len(pre_edit_paths) == len(set(pre_edit_paths)),
        "pre-edit Rust source manifest paths are not uniquely sorted")
pre_edit_hash = hashlib.sha256()
for record in pre_edit_records:
    require(re.fullmatch(r"[0-9a-f]{64}", record["sha256"]) is not None,
            "invalid pre-edit Rust source SHA-256")
    pre_edit_hash.update(record["path"].encode("utf-8") + b"\0")
    pre_edit_hash.update(bytes.fromhex(record["sha256"]) + b"\0")
require(len(pre_edit_records) == pre_edit_source_tree["file_count"],
        "pre-edit Rust source manifest file count mismatch")
require(pre_edit_hash.hexdigest() == pre_edit_source_tree["manifest_sha256"],
        "pre-edit Rust source manifest hash mismatch")
historical_source_tree = manifest["source_tree"]
require(pre_edit_source_tree["file_count"] == historical_source_tree["rust_source_file_count"],
        "pre-edit snapshot source count differs from historical build manifest")
require(pre_edit_source_tree["manifest_sha256"] == historical_source_tree["rust_source_manifest_sha256"],
        "pre-edit snapshot source hash differs from historical build manifest")

identity = parsed_json["artifact-identity-recheck.json"]
require(identity["schema_version"] == 1, "unsupported artifact identity recheck schema")
require(identity["slice"] == "RB-09", "artifact identity recheck slice mismatch")
require(identity["source_tree"]["rust_source_file_count"] == pre_edit_source_tree["file_count"],
        "historical identity source count differs from pre-edit snapshot")
require(identity["source_tree"]["rust_source_manifest_sha256"] == pre_edit_source_tree["manifest_sha256"],
        "historical identity source manifest differs from pre-edit snapshot")
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
require("Compiling modeler3d-mcp" in bloat_log and "Compiling modeler3d-app" in bloat_log,
        "workspace bloat rebuild log does not show app and gateway rebuilds")
require("Compiling modeler3d-app" in restore_log and "Finished" in restore_log,
        "app restore rebuild log does not show a completed app rebuild")

release = schema_parity_manifest["release_validation"]
expected_release_artifacts = (
    ("application", "target/release/modeler3d-app.exe", "application_sha256", "application_bytes"),
    ("gateway", "target/release/r4_mcp.exe", "gateway_sha256", "gateway_bytes"),
)
for artifact_name, expected_relative_path, hash_key, bytes_key in expected_release_artifacts:
    expected_path = ROOT / expected_relative_path
    require(expected_path.is_file(), f"missing current release binary: {expected_path}")
    require(digest(expected_path) == release[hash_key],
            f"current schema-parity Release binary hash mismatch: {artifact_name}")
    require(expected_path.stat().st_size == release[bytes_key],
            f"current schema-parity Release binary size mismatch: {artifact_name}")

launch_path = ROOT / release["gui_launch_record"]
require(launch_path.is_file(), "missing current schema-parity GUI lifecycle record")
launch_record = json.loads(launch_path.read_text(encoding="utf-8"))
require(launch_record["sha256"] == release["application_sha256"],
        "GUI lifecycle record app hash differs from the tested Release app")
require(launch_record["pid"] == release["gui_pid"],
        "GUI lifecycle record PID differs from the schema-parity manifest")
require(launch_record["responding"] is True and launch_record["gui_closed_normally"] is True,
        "current schema-parity Release GUI did not respond and close normally")
require(launch_record["runtime_descriptor_removed"] is True,
        "current schema-parity runtime descriptor was not removed")
require(identity["previous_wrap_up_failure"]["app_hash_at_failure"]
        == identity["release_artifacts"]["rust_app"]["sha256"],
        "previous wrap-up failure app hash differs from the current identity recheck")
require(identity["previous_wrap_up_failure"]["gateway_hash_at_failure"]
        == identity["release_artifacts"]["rust_mcp_gateway"]["sha256"],
        "previous wrap-up failure gateway hash differs from the current identity recheck")
app_process = identity["release_artifacts"]["rust_app"]["process"]
require(app_process["pid"] == identity["cleanup"]["app_pid"], "app process ID mismatch")
require(Path(app_process["executable_path"]).resolve()
        == (ROOT / "target/release/modeler3d-app.exe").resolve(),
        "app process path differs from the current Release app path")
require(app_process["exited_after_close"] is True, "identity recheck app process did not close")
require(identity["release_artifacts"]["rust_mcp_gateway"]["gateway_process_exited_after_client_close"] is True,
        "identity recheck gateway process did not close")
require(identity["cleanup"]["app_process_exited"] is True,
        "identity recheck app process remains running")
require(identity["cleanup"]["rust_builder_process_count_after_shutdown"] == 0,
        "Rust 3D Modeler process remained after identity recheck")
require(identity["cleanup"]["runtime_descriptor_removed"] is True,
        "identity recheck runtime descriptor was not removed")

host = identity["host"]
require(host["input_desktop"] == "Default", "identity screenshot was not on the Default desktop")
require(host["session_state"] == 0, "identity screenshot session was not active")
require(host["lockapp_owned_session"] is False, "LockApp owned the identity recheck session")
require(host["foreground_process"] == "modeler3d-app", "identity screenshot app was not foreground")
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
expected_tool_names = established_tool_names()
check_tool_name_regression(expected_tool_names)
require(ordered_tool_names_match(ready_event["tools"], expected_tool_names),
        "identity-only client ordered tool list differs from Rust MCP TOOL_CATALOG")
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
print(f"historical Rust source manifest verified: {len(pre_edit_records)} files; current source manifest verified: {len(source_files)} files")
print(f"current schema-parity Release identity verified: app {release['application_sha256']}; gateway {release['gateway_sha256']}")
print("identity-only official-client probe verified: 60 tools; get_status app_state=ready; 1296x879 interactive screenshot")
print(f"current Rust Release run verified: {len(current_run['captures'])} historical capture records; current-reference schema parity pass for 7/52/59/60 tools; {len(successful_results)} successful live result envelopes validated")
print("JSON manifests, rebuild logs, official-client results, and accepted/rejected NDJSON journals valid")
