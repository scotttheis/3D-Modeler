"""Capture Release Rust MCP tools/list schemas with the official MCP client."""

from __future__ import annotations

import asyncio
from datetime import datetime, timezone
import hashlib
from importlib.metadata import version
import json
import os
from pathlib import Path
import platform
import sys
from typing import Any

from mcp import Client
from mcp.client.stdio import StdioServerParameters


PROFILE_COUNTS = {"observe": 7, "model": 52, "files": 59, "full": 60}


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


async def capture_profile(binary: Path, profile: str) -> list[dict[str, Any]]:
    environment = os.environ.copy()
    environment["MODELER3D_MCP_PROFILE"] = profile
    server = StdioServerParameters(
        command=str(binary),
        args=[],
        env=environment,
    )
    async with Client(server) as client:
        result = await client.list_tools()
        return [
            tool.model_dump(by_alias=True, mode="json", exclude_none=True)
            for tool in result.tools
        ]


async def main(binary: Path, output_directory: Path) -> dict[str, Any]:
    captures: dict[str, dict[str, Any]] = {}
    output_directory.mkdir(parents=True, exist_ok=True)
    for profile, expected_count in PROFILE_COUNTS.items():
        tools = await capture_profile(binary, profile)
        if len(tools) != expected_count:
            raise RuntimeError(
                f"{profile} returned {len(tools)} tools; expected {expected_count}"
            )
        if any("inputSchema" not in tool or "outputSchema" not in tool for tool in tools):
            raise RuntimeError(f"{profile} returned a tool without both schemas")
        capture = {
            "schema_version": 1,
            "capture_label": "current-rust-release-mcp-tools-list",
            "captured_utc": datetime.now(timezone.utc).isoformat(),
            "client": {
                "method": "official Python MCP SDK 2.2.0 over stdio; tools/list only",
                "sdk_package": "mcp",
                "sdk_version": version("mcp"),
                "python_version": platform.python_version(),
                "rust_binary": str(binary),
                "rust_binary_sha256": sha256(binary),
                "tool_handlers_invoked": False,
            },
            "profile": profile,
            "tool_count": len(tools),
            "tools": tools,
        }
        output_path = output_directory / f"rust-{profile}-live.json"
        output_path.write_text(
            json.dumps(capture, ensure_ascii=False, sort_keys=True, indent=2) + "\n",
            encoding="utf-8",
            newline="\n",
        )
        captures[profile] = {
            "path": output_path.name,
            "tool_count": len(tools),
            "ordered_names": [tool["name"] for tool in tools],
            "input_schemas": sum("inputSchema" in tool for tool in tools),
            "output_schemas": sum("outputSchema" in tool for tool in tools),
        }
    summary = {
        "captured_utc": datetime.now(timezone.utc).isoformat(),
        "binary": str(binary),
        "binary_sha256": sha256(binary),
        "sdk_version": version("mcp"),
        "python_version": platform.python_version(),
        "profiles": captures,
    }
    (output_directory / "rust-capture-summary.json").write_text(
        json.dumps(summary, ensure_ascii=False, sort_keys=True, indent=2) + "\n",
        encoding="utf-8",
        newline="\n",
    )
    return summary


if __name__ == "__main__":
    if len(sys.argv) != 3:
        raise SystemExit("usage: capture_rust_catalog.py <r4_mcp.exe> <output-directory>")
    binary_path = Path(sys.argv[1]).resolve(strict=True)
    output_path = Path(sys.argv[2]).resolve()
    print(json.dumps(asyncio.run(main(binary_path, output_path)), sort_keys=True))
