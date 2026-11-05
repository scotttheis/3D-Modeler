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
    "validate_rb09_evidence.py",
)
for name in required_evidence:
    require((EVIDENCE / name).is_file(), f"missing required evidence: {name}")

json_names = (
    "capture-metadata.json",
    "build-manifest.json",
    "fixture-manifest.json",
    "rust_stdio_client_probe-result.json",
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
for role, expected_path in (
    ("live Rust app", ROOT / "target/release/builder3d-app.exe"),
    ("current post-run Rust MCP gateway", ROOT / "target/release/r4_mcp.exe"),
):
    binary = next(item for item in manifest["builds"] if item["role"] == role)
    require(expected_path.is_file(), f"missing release binary: {expected_path}")
    require(digest(expected_path) == binary["sha256"], f"release binary hash mismatch: {role}")
source_files = sorted((ROOT / "crates").rglob("*.rs"), key=lambda path: path.relative_to(ROOT).as_posix())
source_hash = hashlib.sha256()
for path in source_files:
    source_hash.update(path.relative_to(ROOT).as_posix().encode("utf-8") + b"\0")
    source_hash.update(bytes.fromhex(digest(path)) + b"\0")
require(len(source_files) == manifest["source_tree"]["rust_source_file_count"],
        "Rust source file count mismatch")
require(source_hash.hexdigest() == manifest["source_tree"]["rust_source_manifest_sha256"],
        "Rust source manifest hash mismatch")

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
print(f"live interactive status pass; {len(captures)} PNG hashes/dimensions and 2 clean scene pairs verified")
print(f"fixture hashes verified: {len(fixtures['files'])}; Rust source manifest verified: {len(source_files)} files")
print("JSON manifests, official-client result, and accepted/rejected NDJSON journals valid")
