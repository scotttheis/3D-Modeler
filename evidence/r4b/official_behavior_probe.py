"""Exercise the R4 catalog through the official Python MCP client."""

import asyncio
import os
from pathlib import Path

from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client


async def main() -> None:
    project_root = Path(__file__).resolve().parents[2]
    binary = project_root / "target" / "debug" / "r4_mcp.exe"
    environment = os.environ.copy()
    environment["BUILDER3D_MCP_PROFILE"] = "full"
    environment["BUILDER3D_AI_APPROVED_ROOTS"] = str(project_root / "evidence" / "r4b")
    parameters = StdioServerParameters(command=str(binary), args=[], env=environment)
    async with stdio_client(parameters) as (read, write):
        async with ClientSession(read, write) as client:
            await client.initialize()
            listed = await client.list_tools()

            async def call(name: str, arguments: dict | None = None):
                result = await client.call_tool(name, arguments or {})
                if result.is_error:
                    raise RuntimeError(f"{name} failed: {result.content}")
                if result.structured_content is None:
                    raise RuntimeError(f"{name} returned no structured content")
                return result.structured_content

            status = await call("get_status")
            added = await call(
                "add_primitive",
                {"primitive": "cube", "dimensions_mm": [10, 10, 10]},
            )
            object_id = added["object_id"]
            await call(
                "apply_actions",
                {
                    "actions": [
                        {"action": "translate", "object_id": object_id, "offset_mm": [1, 0, 0]},
                        {"action": "set_color", "object_id": object_id, "color": [0.2, 0.4, 0.6, 1.0]},
                    ]
                },
            )
            await call("select", {"indices": [0]})
            await call("set_snapping", {"enabled": True})
            await call("translate", {"object_id": object_id, "offset_mm": [1, 0, 0]})
            await call("set_transform", {"object_id": object_id, "position_mm": [2, 0, 0]})
            await call("transform_selection", {"translation_mm": [1, 0, 0]})
            await call("snap_translate", {"axis": "x", "proposed_delta_mm": 1.0, "indices": [0]})
            await call("settle", {"index": 0})
            await call("mirror", {"axis": "x", "index": 0})
            await call("align", {"axis": "x", "indices": [0]})
            await call("arrange", {"axis": "x", "indices": [0]})
            await call("set_color", {"index": 0, "color": [0.1, 0.2, 0.3, 1.0]})
            await call("mesh_health", {"index": 0})
            await call("print_check", {"bed_size_mm": 200.0})
            await call("view_state")
            await call("set_view", {"yaw_deg": 15.0})
            await call("frame_view", {"target": "all"})
            await call("orbit_view", {"yaw_delta_deg": 5.0, "pitch_delta_deg": 2.0})
            await call("pan_view", {"offset_mm": [1.0, 0.0, 0.0]})
            await call("zoom_view", {"factor": 1.1})
            await call("view_preset", {"preset": "front"})
            slicers = await call("discover_slicers")
            started = await call("smooth", {"index": 0, "iterations": 1})
            operation_id = started["operation_id"]
            terminal = None
            for _ in range(100):
                operation = await call("operation_status", {"operation_id": operation_id})
                if operation["state"] in {"completed", "failed", "cancelled"}:
                    terminal = operation
                    break
                await asyncio.sleep(0.01)
            if terminal is None or terminal["state"] != "completed":
                raise RuntimeError(f"smooth did not complete: {terminal}")
            summary = await call("scene_summary")
            print(
                {
                    "protocol_version": str(client.protocol_version),
                    "tool_count": len(listed.tools),
                    "initial_app_state": status["app_state"],
                    "object_count": len(summary["objects"]),
                    "slicer_count": len(slicers["slicers"]),
                    "smooth_state": terminal["state"],
                }
            )


if __name__ == "__main__":
    asyncio.run(main())
