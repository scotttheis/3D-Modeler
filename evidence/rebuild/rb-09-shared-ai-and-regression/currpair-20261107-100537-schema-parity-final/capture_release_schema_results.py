"""Capture current Release MCP results from a live 3D Modeler desktop."""

from __future__ import annotations

import argparse
import asyncio
from datetime import datetime, timezone
import json
import os
from pathlib import Path
from typing import Any

from mcp import Client
from mcp.client.stdio import StdioServerParameters


def to_json(value: Any) -> Any:
    if hasattr(value, "model_dump"):
        return value.model_dump(by_alias=True, mode="json", exclude_none=True)
    return value


def append_record(path: Path, record: dict[str, Any]) -> None:
    record = {"timestamp_utc": datetime.now(timezone.utc).isoformat(), **record}
    with path.open("a", encoding="utf-8", newline="\n") as stream:
        stream.write(json.dumps(record, ensure_ascii=False, sort_keys=True) + "\n")
        stream.flush()


async def call_and_record(
    client: Client, path: Path, name: str, arguments: dict[str, Any]
) -> dict[str, Any]:
    result = await client.call_tool(name, arguments)
    record = {
        "event": "tool_result",
        "name": name,
        "arguments": arguments,
        "is_error": bool(result.is_error),
        "structured_content": result.structured_content,
        "content": [to_json(block) for block in result.content],
    }
    append_record(path, record)
    return record


async def call_and_wait(
    client: Client, path: Path, name: str, arguments: dict[str, Any]
) -> dict[str, Any]:
    record = await call_and_record(client, path, name, arguments)
    if record["is_error"]:
        return record
    structured = record.get("structured_content") or {}
    operation_id = structured.get("operation_id")
    if structured.get("state") != "queued" or not isinstance(operation_id, str):
        return record

    for _ in range(100):
        await asyncio.sleep(0.1)
        status = await call_and_record(
            client, path, "operation_status", {"operation_id": operation_id}
        )
        if status["is_error"]:
            raise RuntimeError(f"operation_status failed for {operation_id}")
        state = (status.get("structured_content") or {}).get("state")
        if state in {"completed", "failed", "cancelled"}:
            if state != "completed":
                raise RuntimeError(f"{name} operation ended in state {state}")
            return record
    raise RuntimeError(f"{name} operation {operation_id} did not finish")


async def main(gateway: Path, fixture: Path, output: Path) -> None:
    output.parent.mkdir(parents=True, exist_ok=True)
    if output.exists():
        raise RuntimeError(f"refusing to overwrite existing log: {output}")

    environment = os.environ.copy()
    environment["MODELER3D_MCP_PROFILE"] = "full"
    server = StdioServerParameters(
        command=str(gateway.resolve()), args=[], env=environment
    )

    async with Client(server) as client:
        tools = (await client.list_tools()).tools
        names = [tool.name for tool in tools]
        if len(names) != 60:
            raise RuntimeError(f"expected 60 full-profile tools, received {len(names)}")
        append_record(
            output,
            {
                "event": "ready",
                "client": "official Python MCP SDK",
                "tool_count": len(names),
                "tools": names,
                "gateway": str(gateway.resolve()),
            },
        )

        requests = [
            json.loads(line)
            for line in fixture.read_text(encoding="utf-8").splitlines()
            if line.strip()
        ]
        for request in requests:
            await call_and_wait(
                client,
                output,
                request["name"],
                request.get("arguments", {}),
            )

    records = [
        json.loads(line)
        for line in output.read_text(encoding="utf-8").splitlines()
        if line.strip()
    ]
    results = [record for record in records if record.get("event") == "tool_result"]
    successful = [record for record in results if record.get("is_error") is False]
    errors = [record for record in results if record.get("is_error") is True]
    if len(successful) != 25 or [record["name"] for record in errors] != ["apply_actions"]:
        raise RuntimeError(
            f"expected 25 successful outputs and one nested-input rejection; "
            f"received {len(successful)} successes and {[record['name'] for record in errors]}"
        )


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("gateway", type=Path)
    parser.add_argument("fixture", type=Path)
    parser.add_argument("output", type=Path)
    parsed = parser.parse_args()
    asyncio.run(main(parsed.gateway, parsed.fixture, parsed.output))