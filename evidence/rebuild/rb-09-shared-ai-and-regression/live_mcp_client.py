"""Interactive MCP SDK client used by the RB-09 live desktop run.

The client starts the configured stdio gateway through the official Python
MCP SDK, keeps that session open, and accepts one JSON request per stdin line.
It records the wire-level tool result without going through product internals.
"""

from __future__ import annotations

import asyncio
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import sys
from typing import Any

from mcp import Client
from mcp.client.stdio import StdioServerParameters


def _json_block(block: Any) -> Any:
    if hasattr(block, "model_dump"):
        return block.model_dump(by_alias=True, mode="json", exclude_none=True)
    return str(block)


def _append_log(record: dict[str, Any]) -> None:
    path = os.environ.get("RB09_CLIENT_LOG")
    if not path:
        return
    record = {"timestamp_utc": datetime.now(timezone.utc).isoformat(), **record}
    log_path = Path(path)
    log_path.parent.mkdir(parents=True, exist_ok=True)
    with log_path.open("a", encoding="utf-8", newline="\n") as stream:
        stream.write(json.dumps(record, ensure_ascii=False, sort_keys=True) + "\n")
        stream.flush()


def _emit(record: dict[str, Any]) -> None:
    _append_log(record)
    sys.stdout.write(json.dumps(record, ensure_ascii=False, sort_keys=True) + "\n")
    sys.stdout.flush()


async def _main() -> None:
    command = os.environ.get("RB09_MCP_COMMAND")
    if not command:
        raise RuntimeError("RB09_MCP_COMMAND is required")
    arguments = json.loads(os.environ.get("RB09_MCP_ARGUMENTS", "[]"))
    server = StdioServerParameters(
        command=command,
        args=arguments,
        env=os.environ.copy(),
    )
    async with Client(server) as client:
        tools = (await client.list_tools()).tools
        _emit(
            {
                "event": "ready",
                "client": "official Python MCP SDK",
                "tool_count": len(tools),
                "tools": [tool.name for tool in tools],
            }
        )
        while True:
            line = await asyncio.to_thread(sys.stdin.readline)
            if not line:
                break
            try:
                request = json.loads(line)
                if request.get("command") == "close":
                    _emit({"event": "closing"})
                    break
                if request.get("command") == "list_tools":
                    selected = request.get("names")
                    visible = [
                        tool
                        for tool in tools
                        if selected is None or tool.name in selected
                    ]
                    _emit(
                        {
                            "event": "tools",
                            "tools": [
                                {
                                    "name": tool.name,
                                    "input_schema": tool.input_schema,
                                    "annotations": _json_block(tool.annotations)
                                    if tool.annotations is not None
                                    else None,
                                }
                                for tool in visible
                            ],
                        }
                    )
                    continue
                name = request.get("name") or request.get("tool")
                if not isinstance(name, str) or not name:
                    raise ValueError("request must include a non-empty name")
                tool_arguments = request.get("arguments", {})
                if not isinstance(tool_arguments, dict):
                    raise ValueError("arguments must be a JSON object")
                result = await client.call_tool(name, tool_arguments)
                _emit(
                    {
                        "event": "tool_result",
                        "name": name,
                        "arguments": tool_arguments,
                        "is_error": bool(result.is_error),
                        "structured_content": result.structured_content,
                        "content": [_json_block(block) for block in result.content],
                    }
                )
            except Exception as error:  # Keep the harness alive for the next case.
                _emit(
                    {
                        "event": "client_error",
                        "exception_type": type(error).__name__,
                        "message": str(error),
                    }
                )


if __name__ == "__main__":
    asyncio.run(_main())
