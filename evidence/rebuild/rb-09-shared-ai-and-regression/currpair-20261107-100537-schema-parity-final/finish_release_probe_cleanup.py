from __future__ import annotations

import asyncio
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import sys
from mcp import Client
from mcp.client.stdio import StdioServerParameters

OPERATIONS = (
    "6ee3bd7a564d4ac48057aea78fe8bacb",
    "0b2f6fa0e7024ed88e07add6295bb317",
)

def encode(value):
    if hasattr(value, "model_dump"):
        return value.model_dump(by_alias=True, mode="json", exclude_none=True)
    return value

def append(path: Path, record):
    record = {"timestamp_utc": datetime.now(timezone.utc).isoformat(), **record}
    with path.open("a", encoding="utf-8", newline="\n") as stream:
        stream.write(json.dumps(record, ensure_ascii=False, sort_keys=True) + "\n")
        stream.flush()

async def main():
    gateway = Path("target/release/r4_mcp.exe").resolve()
    log_path = Path("evidence/rebuild/rb-09-shared-ai-and-regression/currpair-20261107-100537-schema-parity-final/logs/rebuilt-app-probe-cleanup.jsonl")
    environment = os.environ.copy()
    environment["BUILDER3D_MCP_PROFILE"] = "full"
    async with Client(StdioServerParameters(command=str(gateway), args=[], env=environment)) as client:
        for operation_id in OPERATIONS:
            for attempt in range(100):
                result = await client.call_tool("operation_status", {"operation_id": operation_id})
                record = {
                    "event": "tool_result",
                    "name": "operation_status",
                    "arguments": {"operation_id": operation_id},
                    "is_error": bool(result.is_error),
                    "structured_content": result.structured_content,
                    "content": [encode(block) for block in result.content],
                }
                append(log_path, record)
                if result.is_error:
                    raise RuntimeError(f"operation_status failed for {operation_id}")
                state = (result.structured_content or {}).get("state")
                if state in {"completed", "failed", "cancelled"}:
                    if state != "completed":
                        raise RuntimeError(f"operation {operation_id} ended in state {state}")
                    break
                await asyncio.sleep(0.1)
            else:
                raise RuntimeError(f"operation {operation_id} did not finish")
        result = await client.call_tool("new_project", {})
        record = {
            "event": "tool_result",
            "name": "new_project",
            "arguments": {},
            "is_error": bool(result.is_error),
            "structured_content": result.structured_content,
            "content": [encode(block) for block in result.content],
        }
        append(log_path, record)
        if result.is_error:
            raise RuntimeError(f"new_project cleanup failed: {result.structured_content}")

if __name__ == "__main__":
    asyncio.run(main())