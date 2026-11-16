"""Capture the current L: MCP catalog without starting its gateway or desktop.

Run with the L:\\Builder3D Python 3.12 environment and bytecode writes disabled.
The script asks the installed MCP SDK to build the same in-memory tool records
used by MCPServer.tools/list, but it never calls MCPServer.run or a tool handler.
"""

from __future__ import annotations

import asyncio
from datetime import datetime, timezone
from importlib.metadata import version
import json
from pathlib import Path
import platform
import sys
from typing import Any


REFERENCE_ROOT = Path(r"L:\Builder3D")
EVIDENCE_ROOT = Path(__file__).resolve().parent
IDENTITY_PATH = EVIDENCE_ROOT / "reference-identity.json"

sys.dont_write_bytecode = True
sys.path.insert(0, str(REFERENCE_ROOT))

from mcp.server.mcpserver import MCPServer  # noqa: E402
from mcp.types import ToolAnnotations  # noqa: E402
from scripts import modeler3d_mcp as gateway  # noqa: E402
from scripts.mcp_catalog import (  # noqa: E402
    CATALOG_VERSION,
    PROFILES,
    catalog_by_name,
    profile_names,
)


def _build_server(profile: str) -> MCPServer:
    allowed_names = set(profile_names(profile))
    specs = catalog_by_name()
    server = MCPServer(gateway.mcp.name)

    for function in gateway.mcp._tools:
        spec = specs[function.__name__]
        if spec.name not in allowed_names:
            continue
        server.tool(
            name=spec.name,
            title=spec.name.replace("_", " ").title(),
            annotations=ToolAnnotations(
                readOnlyHint=spec.read_only,
                destructiveHint=spec.destructive,
                idempotentHint=spec.idempotent,
                openWorldHint=spec.open_world,
            ),
            meta={
                "com.modeler3d/catalogVersion": CATALOG_VERSION,
                "com.modeler3d/category": spec.category,
                "com.modeler3d/profile": spec.profile,
                "com.modeler3d/launchProfile": profile,
                "com.modeler3d/longRunning": spec.long_running,
            },
            structured_output=True,
        )(function)
    return server


async def _tool_records(profile: str) -> list[dict[str, Any]]:
    server = _build_server(profile)
    tools = await server.list_tools()
    return [tool.model_dump(by_alias=True, mode="json", exclude_none=True) for tool in tools]


def _unknown_field_runtime_probe() -> dict[str, Any]:
    server = _build_server("full")
    tools = {tool.name: tool for tool in server._tool_manager.list_tools()}

    top_level_input = {"axis": "x", "__rb09_unknown_probe__": "ignored"}
    validated_top_level = tools["mirror"].fn_metadata.validate_arguments(top_level_input)

    no_argument_input = {"__rb09_unknown_probe__": "ignored"}
    validated_no_arguments = tools["get_status"].fn_metadata.validate_arguments(no_argument_input)

    nested_input = {
        "actions": [
            {
                "action": "translate",
                "args": {
                    "object_id": "schema-probe",
                    "offset_mm": [0.0, 0.0, 0.0],
                    "__rb09_unknown_probe__": "rejected",
                },
            }
        ],
        "__rb09_unknown_probe__": "ignored-at-tool-root",
    }
    try:
        tools["apply_actions"].fn_metadata.validate_arguments(nested_input)
    except Exception as error:  # The nested StrictModel intentionally rejects extras.
        nested_outcome = {
            "accepted": False,
            "exception_type": type(error).__name__,
            "message": str(error),
        }
    else:
        nested_outcome = {"accepted": True}

    return {
        "method": "Tool.fn_metadata.validate_arguments, which Tool.run calls before invoking the handler",
        "handler_invoked": False,
        "top_level_extra": {
            "tool": "mirror",
            "input": top_level_input,
            "accepted": True,
            "validated_arguments": validated_top_level,
            "extra_property_retained": "__rb09_unknown_probe__" in validated_top_level,
        },
        "no_argument_tool_extra": {
            "tool": "get_status",
            "input": no_argument_input,
            "accepted": True,
            "validated_arguments": validated_no_arguments,
        },
        "nested_strict_model_extra": {
            "tool": "apply_actions",
            "input": nested_input,
            **nested_outcome,
        },
    }


def main() -> None:
    identity = json.loads(IDENTITY_PATH.read_text(encoding="utf-8"))
    captures: dict[str, dict[str, Any]] = {}

    for profile in PROFILES:
        tools = asyncio.run(_tool_records(profile))
        payload = {
            "schema_version": 1,
            "capture_label": "current-designated-L-reference-source",
            "captured_utc": datetime.now(timezone.utc).isoformat(),
            "reference_identity": {
                "path": identity["designated_reference"],
                "git_head": identity["git_head"],
                "git_status_sha256": identity["git_status_sha256"],
            },
            "client": {
                "method": "in-memory MCPServer.list_tools schema generation; no transport or app was started",
                "sdk_package": "mcp",
                "sdk_version": version("mcp"),
                "protocol_version": "2026-07-28",
                "python_version": platform.python_version(),
            },
            "profile": profile,
            "tool_count": len(tools),
            "tools": tools,
        }
        output_path = EVIDENCE_ROOT / f"python-{profile}.json"
        output_path.write_text(
            json.dumps(payload, ensure_ascii=False, sort_keys=True, indent=2) + "\n",
            encoding="utf-8",
            newline="\n",
        )
        captures[profile] = {
            "path": output_path.name,
            "tool_count": len(tools),
            "ordered_names": [tool["name"] for tool in tools],
            "input_schemas_with_additionalProperties": sum(
                "additionalProperties" in tool["inputSchema"] for tool in tools
            ),
            "distinct_output_schemas": len(
                {
                    json.dumps(tool.get("outputSchema"), sort_keys=True, separators=(",", ":"))
                    for tool in tools
                }
            ),
        }

    behavior = {
        "schema_version": 1,
        "captured_utc": datetime.now(timezone.utc).isoformat(),
        "reference_identity": {
            "path": identity["designated_reference"],
            "git_head": identity["git_head"],
            "git_status_sha256": identity["git_status_sha256"],
        },
        "sdk_version": version("mcp"),
        "python_version": platform.python_version(),
        "probe": _unknown_field_runtime_probe(),
    }
    behavior_path = EVIDENCE_ROOT / "python-unknown-field-runtime.json"
    behavior_path.write_text(
        json.dumps(behavior, ensure_ascii=False, sort_keys=True, indent=2) + "\n",
        encoding="utf-8",
        newline="\n",
    )

    summary = {
        "reference": identity["designated_reference"],
        "git_head": identity["git_head"],
        "worktree_dirty": identity["git_worktree_dirty"],
        "sdk_version": version("mcp"),
        "python_version": platform.python_version(),
        "profiles": captures,
        "unknown_field_probe": behavior["probe"],
    }
    (EVIDENCE_ROOT / "python-capture-summary.json").write_text(
        json.dumps(summary, ensure_ascii=False, sort_keys=True, indent=2) + "\n",
        encoding="utf-8",
        newline="\n",
    )
    print(json.dumps(summary, sort_keys=True))


if __name__ == "__main__":
    main()
