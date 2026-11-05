"""Exercise the Rust stdio MCP gateway with the installed official MCP SDK.

This probe intentionally points at a missing runtime descriptor. It verifies
the real protocol/client path and the bounded desktop-unavailable response;
it does not claim a live Rust desktop integration check.
"""

from __future__ import annotations

import asyncio
import json
import os
from pathlib import Path

from mcp import Client
from mcp.client.stdio import StdioServerParameters


PROJECT_ROOT = Path(__file__).resolve().parents[3]
MISSING_DESCRIPTOR = (
    PROJECT_ROOT
    / "evidence"
    / "rebuild"
    / "rb-09-shared-ai-and-regression"
    / "missing-rust-desktop-runtime.json"
)


async def main() -> None:
    environment = os.environ.copy()
    environment["BUILDER3D_RUNTIME_FILE"] = str(MISSING_DESCRIPTOR)
    environment["BUILDER3D_MCP_PROFILE"] = "full"
    server = StdioServerParameters(
        command=str(PROJECT_ROOT / "target" / "release" / "r4_mcp.exe"),
        args=[],
        env=environment,
    )
    async with Client(server) as client:
        tools = await client.list_tools()
        names = [tool.name for tool in tools.tools]
        if len(names) != 60 or names[0] != "get_status" or names[-1] != "launch_slicer":
            raise AssertionError(f"unexpected Rust tool catalog: {len(names)} tools")
        result = await client.call_tool("get_status", {})
        structured = result.structured_content
        if not isinstance(structured, dict) or structured.get("code") != "desktop_unavailable":
            raise AssertionError(f"unexpected unavailable response: {structured!r}")
        if not result.is_error:
            raise AssertionError(
                f"structured unavailable result was not marked as an MCP tool error: {result!r}"
            )
        print(
            json.dumps(
                {
                    "client": "official Python MCP SDK 2.2.0",
                    "transport": "stdio",
                    "protocol": "initialized and tools/list succeeded",
                    "tool_count": len(names),
                    "first_tool": names[0],
                    "last_tool": names[-1],
                    "closed_desktop_call": structured,
                    "live_desktop_edit": "unverified; no Rust desktop process is running",
                },
                indent=2,
                sort_keys=True,
            )
        )


if __name__ == "__main__":
    asyncio.run(main())
