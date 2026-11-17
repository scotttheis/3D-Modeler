"""Exercise model import through the running app's authenticated MCP bridge.
Uses an isolated settings directory and task-local descriptor; never prints credentials.
"""
import json
import os
from pathlib import Path
import subprocess
import time
import uuid


ROOT = Path(__file__).resolve().parents[4]
OUTPUT = Path(__file__).resolve().parent / "live"
OUTPUT.mkdir(exist_ok=True)
descriptor_path = OUTPUT / f"runtime-{time.time_ns()}.json"
env = os.environ.copy()
for key in list(env):
    if key.startswith("MODELER3D_REVIEW_"):
        del env[key]
env.update(LOCALAPPDATA=str(OUTPUT / "profile"), MODELER3D_RUNTIME_FILE=str(descriptor_path), MODELER3D_AI_APPROVED_ROOTS=str(ROOT), MODELER3D_MCP_PROFILE="full")
process = subprocess.Popen([str(ROOT / "target/release/modeler3d-app.exe")], cwd=ROOT, env=env)
report = []
descriptor = None

gateway = None
request_id = 0

def rpc(method, params, notification=False):
    global request_id
    request_id += 1
    request = {"jsonrpc":"2.0", "method":method, "params":params}
    if not notification: request["id"] = request_id
    gateway.stdin.write(json.dumps(request) + "\n")
    gateway.stdin.flush()
    if notification: return None
    while True:
        line = gateway.stdout.readline()
        if not line: raise RuntimeError("gateway closed")
        reply = json.loads(line)
        if reply.get("id") == request_id:
            if "error" in reply: raise RuntimeError(str(reply["error"]))
            return reply["result"]

def raw(name, arguments):
    # Use one replay key across retries so a dropped reply cannot repeat a mutation.
    schema = json.loads((ROOT / "crates/modeler3d-mcp/src/r4/schema_contract.json").read_text())["tools"][name]["input_schema"]
    if "idempotency_key" in schema.get("properties", {}):
        arguments = dict(arguments, idempotency_key=str(uuid.uuid4()))
    for attempt in range(8):
        result = rpc("tools/call", {"name":name, "arguments":arguments})
        if not result.get("isError"):
            return result["structuredContent"]
        if not result.get("structuredContent", {}).get("retryable"):
            break
        time.sleep(0.15 * (attempt + 1))
    raise RuntimeError(f"{name}: {result}")

def call(name, **arguments):
    result = raw(name, arguments)
    if "operation_id" in result:
        deadline = time.monotonic() + 45
        while result.get("state") not in ("completed", "failed", "cancelled"):
            if time.monotonic() > deadline:
                raise RuntimeError("operation timed out")
            time.sleep(0.05)
            result = raw("operation_status", {"operation_id": result["operation_id"]})
        if result["state"] != "completed":
            raise RuntimeError(f"{name}: {result}")
        return result.get("result", result)
    return result

try:
    deadline = time.monotonic() + 30
    while not descriptor_path.exists():
        if process.poll() is not None or time.monotonic() > deadline:
            raise RuntimeError("app bridge did not start")
        time.sleep(0.1)
    descriptor = json.loads(descriptor_path.read_text())
    gateway = subprocess.Popen([str(ROOT / "target/debug/r4_mcp.exe")], cwd=ROOT, env=env, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
    rpc("initialize", {"protocolVersion":"2026-07-28", "capabilities":{}, "clientInfo":{"name":"import-check", "version":"1.0"}})
    rpc("notifications/initialized", {}, notification=True)
    call("import_file", path=str(ROOT / "crates/modeler3d-core/tests/fixtures/formats/python_box.stl"))
    assert len(call("scene_summary")["objects"]) == 1
    for extension in ["stl", "obj", "ply", "3mf", "gltf", "glb", "wrl", "3dp"]:
        path = OUTPUT / f"export.{extension}"
        call("save_project" if extension == "3dp" else "export_file", path=str(path), overwrite=True)
        call("import_file", path=str(path))
        state = call("scene_summary")
        assert len(state["objects"]) == 2, extension
        call("undo")
        assert len(call("scene_summary")["objects"]) == 1, extension
        call("redo")
        assert len(call("scene_summary")["objects"]) == 2, extension
        call("undo")
        report.append({"format": extension, "export_import_undo_redo": "pass"})
    before = call("scene_summary")
    bad = OUTPUT / "invalid.stl"
    bad.write_text("this is not a model")
    try:
        call("import_file", path=str(bad))
    except RuntimeError:
        pass
    else:
        raise AssertionError("invalid import unexpectedly succeeded")
    after = call("scene_summary")
    assert before["scene_revision"] == after["scene_revision"]
    assert before["objects"] == after["objects"]
    report.append({"invalid_import_preserves_scene_and_revision": "pass"})
    (OUTPUT / "results.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report))
finally:
    if gateway:
        try:
            raw("exit_app", {"if_unsaved": "discard"})
        except Exception:
            pass
    if gateway:
        gateway.terminate()
        gateway.wait(timeout=10)
    try:
        process.wait(timeout=10)
    except subprocess.TimeoutExpired:
        process.terminate()
