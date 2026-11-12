"""Capture the live MCP tool contract through the official Python SDK."""

from __future__ import annotations

import asyncio
from importlib.metadata import version
import json
import os
from pathlib import Path

from mcp import Client
from mcp.client.stdio import StdioServerParameters


async def main() -> None:
    command = os.environ.get("RB09_MCP_COMMAND")
    output = os.environ.get("RB09_CATALOG_OUTPUT")
    if not command or not output:
        raise RuntimeError("RB09_MCP_COMMAND and RB09_CATALOG_OUTPUT are required")

    arguments = json.loads(os.environ.get("RB09_MCP_ARGUMENTS", "[]"))
    profile = os.environ.get("BUILDER3D_MCP_PROFILE", "full")
    label = os.environ.get("RB09_CATALOG_LABEL", "unspecified")
    server = StdioServerParameters(
        command=command,
        args=arguments,
        env=os.environ.copy(),
    )

    async with Client(server) as client:
        result = await client.list_tools()
        tools = [
            tool.model_dump(by_alias=True, mode="json", exclude_none=True)
            for tool in result.tools
        ]

    payload = {
        "schema_version": 1,
        "capture_label": label,
        "profile": profile,
        "client": {
            "name": "official Python MCP SDK",
            "package": "mcp",
            "version": version("mcp"),
            "transport": "stdio",
        },
        "tool_count": len(tools),
        "tools": tools,
    }
    output_path = Path(output)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(
        json.dumps(payload, ensure_ascii=False, sort_keys=True, indent=2) + "\n",
        encoding="utf-8",
        newline="\n",
    )
    print(
        json.dumps(
            {
                "capture_label": label,
                "profile": profile,
                "tool_count": len(tools),
                "sdk_version": payload["client"]["version"],
                "output": str(output_path),
            },
            sort_keys=True,
        )
    )


if __name__ == "__main__":
    asyncio.run(main())
