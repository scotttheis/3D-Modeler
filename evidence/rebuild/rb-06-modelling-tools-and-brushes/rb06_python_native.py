"""RB-06 paired reference evidence using real Windows input events.

The reference checkout is imported read-only.  All generated files belong to
the Rust evidence tree; this script disables Python bytecode and pytest cache
generation before importing the reference application.
"""

from __future__ import annotations

import ctypes
import json
import sys
import time
import traceback
from pathlib import Path

# Windows multiprocessing can re-import the entry script as ``__mp_main__``
# when a reference service starts a worker.  This evidence runner owns one
# foreground UI and must never duplicate it in a child process.
if __name__ != "__main__":
    raise SystemExit(0)

sys.dont_write_bytecode = True

ROOT = Path("E:/3D-Modeler/evidence/rebuild/rb-06-modelling-tools-and-brushes")
CAPTURES = ROOT / "captures"
FIXTURES = ROOT / "fixtures"
RESULTS = ROOT / "python-native-results.json"
CAPTURES.mkdir(exist_ok=True)
FIXTURES.mkdir(exist_ok=True)

RB05_ROOT = Path("E:/3D-Modeler/evidence/rebuild/rb-05-viewport-and-input-completion")
sys.path.insert(0, str(RB05_ROOT))
from native_desktop_guard import require_default_desktop  # noqa: E402

sys.path.insert(0, "L:/Builder3D")
from src.main import configure_opengl_defaults  # noqa: E402

configure_opengl_defaults()

from PyQt6.QtCore import QPoint  # noqa: E402
from PyQt6.QtWidgets import QApplication, QAbstractButton  # noqa: E402

from src.main_window import MainWindow  # noqa: E402
from src.mesh_ops import make_box  # noqa: E402
from src.project_io import save_project  # noqa: E402
from src.style import apply_style  # noqa: E402


USER = ctypes.windll.user32
USER.GetForegroundWindow.restype = ctypes.c_void_p
USER.GetWindowThreadProcessId.argtypes = [ctypes.c_void_p, ctypes.c_void_p]
USER.AttachThreadInput.argtypes = [ctypes.c_ulong, ctypes.c_ulong, ctypes.c_bool]
USER.keybd_event.argtypes = [ctypes.c_ubyte, ctypes.c_ubyte, ctypes.c_ulong, ctypes.c_void_p]
KERNEL = ctypes.windll.kernel32
KERNEL.GetCurrentThreadId.restype = ctypes.c_ulong

LEFT_DOWN = 0x0002
LEFT_UP = 0x0004
SHIFT = 0x10
KEY_UP = 0x0002


def pause(milliseconds: int = 240) -> None:
    app.processEvents()
    time.sleep(milliseconds / 1000.0)
    app.processEvents()


def activate() -> None:
    require_default_desktop()
    handle = int(window.winId())
    foreground = int(USER.GetForegroundWindow() or 0)
    foreground_thread = USER.GetWindowThreadProcessId(foreground, None) if foreground else 0
    current_thread = KERNEL.GetCurrentThreadId()
    if foreground_thread and foreground_thread != current_thread:
        USER.AttachThreadInput(foreground_thread, current_thread, True)
    USER.ShowWindowAsync(handle, 9)
    USER.BringWindowToTop(handle)
    USER.SetForegroundWindow(handle)
    if foreground_thread and foreground_thread != current_thread:
        USER.AttachThreadInput(foreground_thread, current_thread, False)
    pause(80)
    if int(USER.GetForegroundWindow()) != handle:
        raise RuntimeError("reference window did not become foreground")


def viewport_point(x: float, y: float) -> tuple[int, int]:
    point = window.viewport.mapToGlobal(QPoint(round(x), round(y)))
    return point.x(), point.y()


def pointer(x: float, y: float) -> None:
    activate()
    gx, gy = viewport_point(x, y)
    USER.SetCursorPos(gx, gy)
    pause(70)


def click_widget(widget: QAbstractButton) -> None:
    point = widget.mapToGlobal(widget.rect().center())
    USER.SetCursorPos(point.x(), point.y())
    pause(60)
    USER.mouse_event(LEFT_DOWN, 0, 0, 0, 0)
    pause(90)
    USER.mouse_event(LEFT_UP, 0, 0, 0, 0)
    pause(300)


def visible_buttons() -> list[QAbstractButton]:
    result: list[QAbstractButton] = []
    for top_level in QApplication.topLevelWidgets():
        result.extend(top_level.findChildren(QAbstractButton))
    return result


def button_named(name: str) -> QAbstractButton | None:
    needle = name.casefold()
    for button in visible_buttons():
        label = f"{button.text()} {button.toolTip()}".casefold()
        if needle in label and button.isVisible() and button.isEnabled():
            return button
    return None


def click_named(name: str) -> None:
    button = button_named(name)
    if button is None:
        raise RuntimeError(f"reference button is not reachable: {name}")
    click_widget(button)


def click_tab(name: str) -> None:
    button = button_named(name)
    if button is None:
        raise RuntimeError(f"reference tab is not reachable: {name}")
    if not button.isChecked():
        click_widget(button)


def press_key(key: int) -> None:
    activate()
    USER.keybd_event(key, 0, 0, 0)
    pause(60)
    USER.keybd_event(key, 0, KEY_UP, 0)
    pause(260)


def shift_drag(start: tuple[float, float], finish: tuple[float, float]) -> None:
    pointer(*start)
    USER.keybd_event(SHIFT, 0, 0, 0)
    try:
        USER.mouse_event(LEFT_DOWN, 0, 0, 0, 0)
        pause(100)
        for step in range(1, 7):
            fraction = step / 6.0
            pointer(
                start[0] + (finish[0] - start[0]) * fraction,
                start[1] + (finish[1] - start[1]) * fraction,
            )
        USER.mouse_event(LEFT_UP, 0, 0, 0, 0)
    finally:
        USER.keybd_event(SHIFT, 0, KEY_UP, 0)
    pause(420)


def scene_state() -> list[dict]:
    return [
        {
            "name": obj.name,
            "visible": bool(obj.visible),
            "transform": obj.transform.tolist(),
        }
        for obj in window.scene.objects
    ]


def setup_scene(kind: str) -> str:
    window.scene.clear()
    window.scene.add_object(make_box((20, 30, 40)), "Box A")
    first = window.scene.objects[-1]
    first.translate(0, 15, 0)
    window.scene.add_object(make_box((10, 20, 30)), "Box B")
    second = window.scene.objects[-1]
    second.translate(40, 10, 0)
    if kind == "hidden":
        second.visible = False
    window.scene.set_selection({0} if kind != "multiple" else {0, 1})
    window.scene.objects_changed.emit()
    window._show_workspace()
    window.viewport.camera.reset_view()
    window.viewport.frame_scene()
    pause(600)
    fixture = FIXTURES / f"python-{kind}.3dp"
    save_project(window.scene, str(fixture))
    return fixture.relative_to(ROOT).as_posix()


def capture(action: str, theme: str, fixture: str, *, overlay: bool = False, input_route: str) -> None:
    activate()
    pause(180)
    # Qt's widget grab is deterministic for the paired logical surface and
    # remains safe while a modal child owns the native input queue.
    pixmap = window.grab()
    path = CAPTURES / f"python-{theme}-{action}.png"
    if not pixmap.save(str(path)):
        raise RuntimeError(f"could not save {path}")
    records.append(
        {
            "action": action,
            "theme": theme,
            "fixture": fixture,
            "input_route": input_route,
            "logical_window_size": [window.width(), window.height()],
            "captured_size": [pixmap.width(), pixmap.height()],
            "device_pixel_ratio": float(window.devicePixelRatioF()),
            "screen_dpi": float(window.screen().logicalDotsPerInch()),
            "objects": scene_state(),
            "selection": sorted(int(index) for index in window.scene.selected_indices),
            "image": path.relative_to(ROOT).as_posix(),
            "overlay_capture": overlay,
        }
    )


def select(indices: set[int]) -> None:
    window.scene.set_selection(indices)
    window.scene.objects_changed.emit()
    pause(260)


app = QApplication([])
apply_style(app, "dark")
window = MainWindow()
window.resize(1280, 840)
window.show()
window.raise_()
window.activateWindow()
records: list[dict] = []

try:
    pause(1400)
    for theme in ("light", "dark"):
        window._set_theme_mode(theme)
        pause(550)

        window.scene.clear()
        window._show_workspace()
        window.viewport.camera.reset_view()
        capture("empty-rest", theme, "empty", input_route="native reference window capture")

        window.scene.clear()
        window._show_workspace()
        click_tab("Insert")
        click_named("Cube")
        capture("primitive-cube", theme, "primitive-cube", input_route="actual OS Insert/Cube click")

        fixture = setup_scene("single")
        capture("modelling-fixture", theme, fixture, input_route="reference fixture setup")

    RESULTS.write_text(json.dumps(records, indent=2), encoding="utf-8")
except Exception:
    (ROOT / "python-native-error.txt").write_text(traceback.format_exc(), encoding="utf-8")
    raise
finally:
    window.close()
    app.quit()
    app.processEvents()
