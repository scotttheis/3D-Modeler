"""Probe the Rust server's pre-initialize discovery response over stdio."""

import json
import os
import subprocess
from pathlib import Path


def main() -> None:
    project_root = Path(__file__).resolve().parents[2]
    binary = project_root / "target" / "debug" / "r4_mcp.exe"
    environment = os.environ.copy()
    environment["BUILDER3D_MCP_PROFILE"] = "full"
    environment["BUILDER3D_AI_APPROVED_ROOTS"] = str(project_root / "evidence" / "r4a")
    request = {
        "jsonrpc": "2.0",
        "id": 1,
        "method": "server/discover",
        "params": {
            "_meta": {
                "io.modelcontextprotocol/protocolVersion": "2026-07-28",
                "io.modelcontextprotocol/clientCapabilities": {},
            }
        },
    }
    initialize = {
        "jsonrpc": "2.0",
        "id": 2,
        "method": "initialize",
        "params": {
            "protocolVersion": "2026-07-28",
            "capabilities": {},
            "clientInfo": {"name": "r4-raw-probe", "version": "0.1"},
        },
    }
    initialized = {
        "jsonrpc": "2.0",
        "method": "notifications/initialized",
        "params": {},
    }
    completed = subprocess.run(
        [str(binary)],
        input="\n".join(json.dumps(message) for message in (request, initialize, initialized)) + "\n",
        text=True,
        capture_output=True,
        env=environment,
        timeout=10,
        check=False,
    )
    lines = [line for line in completed.stdout.splitlines() if line.strip()]
    if not lines:
        raise RuntimeError(f"server returned no discovery response: {completed.stderr}")
    response = next(json.loads(line) for line in lines if json.loads(line).get("id") == 1)
    if "error" in response:
        raise RuntimeError(response["error"])
    result = response["result"]
    print(
        {
            "exit_code": completed.returncode,
            "supported_versions": result["supportedVersions"],
            "server_info": result["_meta"]["io.modelcontextprotocol/serverInfo"],
            "tools_capability": "tools" in result["capabilities"],
        }
    )


if __name__ == "__main__":
    main()
