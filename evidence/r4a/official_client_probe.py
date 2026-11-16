"""One-shot official Python MCP SDK probe for the Rust R4 stdio server."""

import asyncio
import os
from pathlib import Path

from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client


async def main() -> None:
    project_root = Path(__file__).resolve().parents[2]
    binary = project_root / "target" / "debug" / "r4_mcp.exe"
    environment = os.environ.copy()
    environment["MODELER3D_MCP_PROFILE"] = "full"
    environment["MODELER3D_AI_APPROVED_ROOTS"] = str(project_root / "evidence" / "r4a")
    parameters = StdioServerParameters(command=str(binary), args=[], env=environment)
    async with stdio_client(parameters) as (read, write):
        async with ClientSession(read, write) as client:
            await client.initialize()
            tools = await client.list_tools()
            status = await client.call_tool("get_status", {})
            added = await client.call_tool(
                "add_primitive",
                {"primitive": "cube", "dimensions_mm": [10, 10, 10]},
            )
            summary = await client.call_tool("scene_summary", {})
            print(
                {
                    "protocol_version": str(client.protocol_version),
                    "tool_count": len(tools.tools),
                    "first_tool": tools.tools[0].name,
                    "last_tool": tools.tools[-1].name,
                    "status_structured": status.structured_content is not None,
                    "add_structured": added.structured_content is not None,
                    "summary_objects": len(summary.structured_content["objects"]),
                }
            )


if __name__ == "__main__":
    asyncio.run(main())
