"""Exercise update tools through the real stdio gateway and isolated desktop.

Build the debug modeler3d-app and r4_mcp binaries first. Standard library only;
all settings, descriptors and logs stay in a fresh project logs/ directory.
"""

import json
import os
from pathlib import Path
import queue
import subprocess
import threading
import time
import uuid


def main():
    root = Path(__file__).resolve().parent.parent
    output = root / "logs" / ("update-client-" + uuid.uuid4().hex)
    output.mkdir(parents=True)
    environment = dict(os.environ)
    environment["LOCALAPPDATA"] = str(output / "profile")
    environment["MODELER3D_RUNTIME_FILE"] = str(output / "runtime.json")
    environment["MODELER3D_AI_APPROVED_ROOTS"] = str(root)
    children = []
    with (output / "desktop.log").open("w", encoding="utf-8") as app_log, (output / "gateway.log").open("w", encoding="utf-8") as gateway_log:
        try:
            desktop = subprocess.Popen([str(root / "target/debug/modeler3d-app.exe")], cwd=root, env=environment, stdout=app_log, stderr=app_log)
            children.append(desktop)
            deadline = time.monotonic() + 40
            while not Path(environment["MODELER3D_RUNTIME_FILE"]).exists():
                if desktop.poll() is not None or time.monotonic() >= deadline:
                    raise RuntimeError("Isolated desktop did not start; inspect its log")
                time.sleep(0.1)
            gateway = subprocess.Popen([str(root / "target/debug/r4_mcp.exe")], cwd=root, env=environment, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=gateway_log, text=True, encoding="utf-8")
            children.append(gateway)
            replies = queue.Queue()

            def read_replies():
                for line in gateway.stdout:
                    replies.put(json.loads(line))

            threading.Thread(target=read_replies, daemon=True).start()
            sequence = 0

            def send(method, params, notification=False):
                nonlocal sequence
                sequence += 1
                request = {"jsonrpc": "2.0", "method": method, "params": params}
                if not notification:
                    request["id"] = sequence
                gateway.stdin.write(json.dumps(request) + "\n")
                gateway.stdin.flush()
                if notification:
                    return None
                deadline = time.monotonic() + 30
                while True:
                    result = replies.get(timeout=max(0.01, deadline - time.monotonic()))
                    if result.get("id") == sequence:
                        if "error" in result:
                            raise RuntimeError(result["error"])
                        return result["result"]

            send("initialize", {"protocolVersion": "2026-07-28", "capabilities": {}, "clientInfo": {"name": "update-contract-check", "version": "1"}})
            send("notifications/initialized", {}, notification=True)
            tools = send("tools/list", {})["tools"]
            names = {tool["name"] for tool in tools}
            assert {"get_update_status", "respond_to_update"} <= names
            state = send("tools/call", {"name": "get_update_status", "arguments": {}})
            assert not state.get("isError"), state
            value = state["structuredContent"]
            assert value["available_version"] is None
            rejected = send("tools/call", {"name": "respond_to_update", "arguments": {"response": "install", "version": "999.0.0"}})
            assert rejected.get("isError"), rejected
            assert desktop.poll() is None, "A rejected install closed the desktop"
            send("tools/call", {"name": "exit_app", "arguments": {}})
            desktop.wait(timeout=15)
            summary = {"tools": len(tools), "phase": value["phase"], "unavailable_install_rejected": True, "desktop_closed_normally": True}
            (output / "result.json").write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
            print(json.dumps(summary))
        finally:
            for child in reversed(children):
                if child.poll() is None:
                    child.terminate()
                    child.wait(timeout=10)


if __name__ == "__main__":
    main()
