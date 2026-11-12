"""Run one revision race through two independent official MCP clients."""

from __future__ import annotations

import asyncio
import json
import os
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from mcp import Client
from mcp.client.stdio import StdioServerParameters


def _dump(value: Any) -> Any:
    if hasattr(value, "model_dump"):
        return value.model_dump(by_alias=True, mode="json", exclude_none=True)
    return str(value)


async def _main() -> None:
    command = os.environ["RB09_MCP_COMMAND"]
    arguments = json.loads(os.environ.get("RB09_MCP_ARGUMENTS", "[]"))
    server_a = StdioServerParameters(command=command, args=arguments, env=os.environ.copy())
    server_b = StdioServerParameters(command=command, args=arguments, env=os.environ.copy())
    project_id = os.environ["RB09_RACE_PROJECT_ID"]
    object_id = os.environ["RB09_RACE_OBJECT_ID"]
    expected_revision = int(os.environ["RB09_RACE_REVISION"])

    async with Client(server_a) as client_a, Client(server_b) as client_b:
        tools_a = (await client_a.list_tools()).tools
        tools_b = (await client_b.list_tools()).tools
        if [tool.name for tool in tools_a] != [tool.name for tool in tools_b]:
            raise RuntimeError("the two clients received different tool catalogs")

        async def edit(client: Client, color: list[float], key: str) -> dict[str, Any]:
            result = await client.call_tool(
                "set_color",
                {
                    "object_id": object_id,
                    "color": color,
                    "expected_project_id": project_id,
                    "expected_scene_revision": expected_revision,
                    "idempotency_key": key,
                },
            )
            return {
                "is_error": bool(result.is_error),
                "structured_content": result.structured_content,
                "content": [_dump(item) for item in result.content],
            }

        results = await asyncio.gather(
            edit(client_a, [0.9, 0.1, 0.2, 1.0], "rb09-current-race-a"),
            edit(client_b, [0.1, 0.8, 0.3, 1.0], "rb09-current-race-b"),
            return_exceptions=True,
        )

    payload = {
        "timestamp_utc": datetime.now(timezone.utc).isoformat(),
        "client": "official Python MCP SDK",
        "tool_count": len(tools_a),
        "project_id": project_id,
        "object_id": object_id,
        "expected_scene_revision": expected_revision,
        "results": [
            {"client": label, **result}
            if isinstance(result, dict)
            else {
                "client": label,
                "exception_type": type(result).__name__,
                "message": str(result),
            }
            for label, result in zip(("A", "B"), results, strict=True)
        ],
    }
    output = Path(os.environ["RB09_RACE_OUTPUT"])
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(
        json.dumps(payload, ensure_ascii=False, sort_keys=True, indent=2) + "\n",
        encoding="utf-8",
        newline="\n",
    )
    print(json.dumps({"output": str(output), "tool_count": payload["tool_count"]}))


if __name__ == "__main__":
    asyncio.run(_main())
