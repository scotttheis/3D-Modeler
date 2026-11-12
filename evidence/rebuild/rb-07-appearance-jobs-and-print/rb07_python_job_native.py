"""Capture the Python reference's real native mesh-job lifecycle."""

from __future__ import annotations

import ctypes
import json
import sys
import time
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path("E:/Builder3D-Rust/evidence/rebuild/rb-07-appearance-jobs-and-print")
CAPTURES = ROOT / "captures"
FIXTURE = ROOT / "fixtures/slow-job-closed-cube-heavy.ply"
RB05_ROOT = Path("E:/Builder3D-Rust/evidence/rebuild/rb-05-viewport-and-input-completion")
sys.path.insert(0, str(RB05_ROOT))
from native_desktop_guard import require_default_desktop  # noqa: E402

sys.path.insert(0, "L:/Builder3D")
from src.main import configure_opengl_defaults  # noqa: E402

configure_opengl_defaults()

from PIL import ImageGrab  # noqa: E402
from PyQt6.QtCore import QPoint  # noqa: E402
from PyQt6.QtWidgets import QAbstractButton, QApplication, QWidget  # noqa: E402
import trimesh  # noqa: E402

from src.main_window import MainWindow  # noqa: E402

USER = ctypes.windll.user32
KERNEL = ctypes.windll.kernel32
USER.GetForegroundWindow.restype = ctypes.c_void_p
USER.GetWindowThreadProcessId.argtypes = [ctypes.c_void_p, ctypes.POINTER(ctypes.c_ulong)]
USER.AttachThreadInput.argtypes = [ctypes.c_ulong, ctypes.c_ulong, ctypes.c_bool]
USER.ShowWindowAsync.argtypes = [ctypes.c_void_p, ctypes.c_int]
USER.BringWindowToTop.argtypes = [ctypes.c_void_p]
USER.SetForegroundWindow.argtypes = [ctypes.c_void_p]
USER.GetClientRect.argtypes = [ctypes.c_void_p, ctypes.POINTER(ctypes.c_long * 4)]
USER.ClientToScreen.argtypes = [ctypes.c_void_p, ctypes.POINTER(ctypes.c_long * 2)]
KERNEL.GetCurrentThreadId.restype = ctypes.c_ulong
LEFT_DOWN = 0x0002
LEFT_UP = 0x0004


class MemorySettings:
    """Keep native evidence preferences out of the reference user's settings."""

    def __init__(self):
        self.values: dict[str, object] = {}

    def contains(self, key: str) -> bool:
        return key in self.values

    def get(self, key: str, default=None):
        return self.values.get(key, default)

    def set(self, key: str, value) -> None:
        self.values[key] = value

    def sync(self) -> None:
        return None


class RECT(ctypes.Structure):
    _fields_ = [("left", ctypes.c_long), ("top", ctypes.c_long), ("right", ctypes.c_long), ("bottom", ctypes.c_long)]


class POINT(ctypes.Structure):
    _fields_ = [("x", ctypes.c_long), ("y", ctypes.c_long)]


USER.GetClientRect.argtypes = [ctypes.c_void_p, ctypes.POINTER(RECT)]
USER.ClientToScreen.argtypes = [ctypes.c_void_p, ctypes.POINTER(POINT)]


def pause(milliseconds: int = 200) -> None:
    app.processEvents()
    time.sleep(milliseconds / 1000.0)
    app.processEvents()


def activate_handle(handle: int) -> None:
    require_default_desktop()
    foreground = int(USER.GetForegroundWindow() or 0)
    foreground_pid = ctypes.c_ulong()
    foreground_thread = (
        USER.GetWindowThreadProcessId(ctypes.c_void_p(foreground), ctypes.byref(foreground_pid))
        if foreground
        else 0
    )
    current_thread = KERNEL.GetCurrentThreadId()
    if foreground_thread and foreground_thread != current_thread:
        USER.AttachThreadInput(foreground_thread, current_thread, True)
    USER.ShowWindowAsync(ctypes.c_void_p(handle), 9)
    USER.BringWindowToTop(ctypes.c_void_p(handle))
    USER.SetForegroundWindow(ctypes.c_void_p(handle))
    if foreground_thread and foreground_thread != current_thread:
        USER.AttachThreadInput(foreground_thread, current_thread, False)
    pause(100)
    if int(USER.GetForegroundWindow() or 0) != handle:
        raise RuntimeError("reference Builder3D window did not become foreground")


def click_widget(widget: QWidget) -> None:
    activate_handle(int(widget.window().winId()))
    point = widget.mapToGlobal(widget.rect().center())
    USER.SetCursorPos(point.x(), point.y())
    time.sleep(0.05)
    USER.mouse_event(LEFT_DOWN, 0, 0, 0, 0)
    time.sleep(0.05)
    USER.mouse_event(LEFT_UP, 0, 0, 0, 0)
    pause(250)


def button_exact(name: str) -> QAbstractButton | None:
    needle = name.casefold()
    for button in window.findChildren(QAbstractButton):
        if not button.isVisible() or not button.isEnabled():
            continue
        text = str(button.text()).replace("&", "").strip().casefold()
        command_text = str(getattr(button, "_command_text", "")).casefold()
        if text == needle or command_text == needle:
            return button
    return None


def click_tab(name: str) -> None:
    if getattr(window.ribbon, "_active_tab_name", None) == name:
        return
    button = button_exact(name)
    if button is None:
        raise RuntimeError(f"native ribbon tab is not reachable: {name}")
    click_widget(button)
    if getattr(window.ribbon, "_active_tab_name", None) != name:
        raise RuntimeError(f"native click did not activate ribbon tab {name}")


def capture(action: str, theme: str, route: str) -> dict:
    pause(120)
    handle = ctypes.c_void_p(int(window.winId()))
    client = RECT()
    origin = POINT(0, 0)
    if not USER.GetClientRect(handle, ctypes.byref(client)) or not USER.ClientToScreen(
        handle, ctypes.byref(origin)
    ):
        raise RuntimeError("could not resolve the native reference client rectangle")
    width = client.right - client.left
    height = client.bottom - client.top
    image = ImageGrab.grab(bbox=(origin.x, origin.y, origin.x + width, origin.y + height))
    path = CAPTURES / f"python-{theme}-{action}.png"
    image.save(path)

    active = window._active_operation
    progress = None
    if active is not None:
        dialog = active.dialog._dialog
        cancel = active.dialog._cancel_button
        progress = {
            "title": active.title,
            "state": window._operation_service.status(active.operation_id)["state"],
            "visible": dialog.isVisible(),
            "label": dialog.labelText(),
            "value": dialog.value(),
            "minimum": dialog.minimum(),
            "maximum": dialog.maximum(),
            "cancel_enabled": bool(cancel is not None and cancel.isEnabled()),
        }
    record = {
        "app": "Python reference",
        "theme": theme,
        "action": action,
        "input_route": route,
        "logical_window_size": [window.width(), window.height()],
        "captured_size": [image.width, image.height],
        "screen_dpi": float(window.screen().logicalDotsPerInch()),
        "scene_objects": len(window.scene.objects),
        "selection": sorted(int(index) for index in window.scene.selected_indices),
        "status": window.statusBar().currentMessage(),
        "progress": progress,
        "image": path.relative_to(ROOT).as_posix(),
    }
    records.append(record)
    return record


def wait_until_idle(timeout_seconds: float = 45.0) -> None:
    deadline = time.monotonic() + timeout_seconds
    while window._active_operation is not None and time.monotonic() < deadline:
        app.processEvents()
        time.sleep(0.02)
    app.processEvents()
    if window._active_operation is not None:
        raise TimeoutError("native mesh operation did not reach a terminal state")


def setup_scene() -> tuple[int, int]:
    mesh = trimesh.load(str(FIXTURE), force="mesh", process=False)
    if not isinstance(mesh, trimesh.Trimesh) or len(mesh.faces) < 1_800_000:
        raise RuntimeError("heavy slow-job PLY did not load as the expected dense mesh")
    window.scene.clear()
    window.scene.add_object(mesh, "RB-07 Slow Job Cube")
    window.scene.set_selection({0})
    window.scene._emit_objects_changed()
    window._show_workspace()
    window.viewport.camera.reset_view()
    window.viewport.frame_scene()
    window.viewport.update()
    pause(500)
    return int(window.scene.revision), len(window.scene.undo_stack)


app = QApplication([])
records: list[dict] = []
window = MainWindow()
window._settings = MemorySettings()
window.resize(1280, 840)
window.show()
window.raise_()
window.activateWindow()

try:
    pause(1400)
    activate_handle(int(window.winId()))
    for theme in ("light", "dark"):
        window._set_theme_mode(theme)
        revision_before, history_before = setup_scene()
        click_tab("Edit")
        capture("job-heavy-scene-ready", theme, "native Edit tab with one selected 1,920,000-face mesh")

        smooth_button = button_exact("Smooth")
        if smooth_button is None:
            raise RuntimeError("native Smooth command button is not reachable")
        click_widget(smooth_button)
        if window._active_operation is None:
            raise RuntimeError("native Smooth command did not start a background operation")

        running_state = window._active_operation
        progress_dialog = running_state.dialog._dialog
        if not progress_dialog.isVisible():
            raise RuntimeError("native Smooth progress dialog did not become visible")
        capture("job-heavy-smooth-progress", theme, "native Edit tab -> Smooth; visible QProgressDialog")
        cancel_button = running_state.dialog._cancel_button
        if cancel_button is None or not cancel_button.isEnabled():
            raise RuntimeError("native Smooth cancel button was not enabled while running")
        click_widget(cancel_button)
        pause(120)
        capture("job-heavy-smooth-cancel-requested", theme, "native QProgressDialog -> Cancel")
        wait_until_idle()
        if window.scene.revision != revision_before or len(window.scene.undo_stack) != history_before:
            raise RuntimeError("cancelled native Smooth committed partial scene/history changes")
        if "cancelled" not in window.statusBar().currentMessage().casefold():
            raise RuntimeError("native Smooth cancellation did not report a terminal status")
        capture("job-heavy-smooth-cancelled", theme, "terminal native cancellation; scene/history unchanged")

        smooth_button = button_exact("Smooth")
        if smooth_button is None:
            raise RuntimeError("native Smooth retry button is not reachable")
        click_widget(smooth_button)
        wait_until_idle()
        if "smoothed 1 object" not in window.statusBar().currentMessage().casefold():
            raise RuntimeError("native Smooth retry did not complete successfully")
        if len(window.scene.undo_stack) != history_before + 1:
            raise RuntimeError("successful Smooth retry did not make one history commit")
        capture("job-heavy-smooth-retry-complete", theme, "native Edit tab -> Smooth after cancelled run")

    (ROOT / "python-job-heavy-native-results.json").write_text(
        json.dumps({"captures": records, "failures": []}, indent=2),
        encoding="utf-8",
    )
finally:
    if window._active_operation is not None:
        window._cancel_active_operation()
        wait_until_idle()
    window._set_scene_dirty(False)
    window.close()
    app.quit()
    app.processEvents()
