"""RB-05 paired reference-window evidence using real Windows input events."""

from __future__ import annotations

import ctypes
import json
import math
import sys
import time
import traceback
from pathlib import Path

from native_desktop_guard import require_default_desktop

sys.dont_write_bytecode = True
sys.path.insert(0, "L:/Builder3D")

from src.main import configure_opengl_defaults

configure_opengl_defaults()

from PyQt6.QtCore import QPoint
from PyQt6.QtWidgets import QApplication, QAbstractButton

from src.main_window import MainWindow
from src.mesh_ops import make_box
from src.project_io import save_project
from src.style import apply_style


ROOT = Path("E:/3D-Modeler/evidence/rebuild/rb-05-viewport-and-input-completion")
CAPTURES = ROOT / "captures"
CAPTURES.mkdir(exist_ok=True)
FIXTURES = ROOT / "fixtures"
FIXTURES.mkdir(exist_ok=True)
RESULTS = ROOT / "python-native-results.json"

USER = ctypes.windll.user32
USER.GetForegroundWindow.restype = ctypes.c_void_p
USER.GetWindowThreadProcessId.argtypes = [ctypes.c_void_p, ctypes.c_void_p]
USER.AttachThreadInput.argtypes = [ctypes.c_ulong, ctypes.c_ulong, ctypes.c_bool]
KERNEL = ctypes.windll.kernel32
KERNEL.GetCurrentThreadId.restype = ctypes.c_ulong
MOUSE_MOVE = 0x0001
LEFT_DOWN = 0x0002
LEFT_UP = 0x0004
RIGHT_DOWN = 0x0008
RIGHT_UP = 0x0010
MIDDLE_DOWN = 0x0020
MIDDLE_UP = 0x0040
WHEEL = 0x0800
WHEEL_DELTA = 120


def pause(milliseconds: int = 220) -> None:
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


def mouse(button_down: int, button_up: int) -> None:
    require_default_desktop()
    USER.mouse_event(button_down, 0, 0, 0, 0)
    pause(100)
    USER.mouse_event(button_up, 0, 0, 0, 0)
    pause(180)


def drag(start: tuple[float, float], finish: tuple[float, float], button_down: int, button_up: int) -> None:
    pointer(*start)
    USER.mouse_event(button_down, 0, 0, 0, 0)
    pause(90)
    for step in range(1, 7):
        fraction = step / 6.0
        pointer(
            start[0] + (finish[0] - start[0]) * fraction,
            start[1] + (finish[1] - start[1]) * fraction,
        )
    USER.mouse_event(button_up, 0, 0, 0, 0)
    pause(260)


def wheel(x: float, y: float, delta: int) -> None:
    pointer(x, y)
    USER.mouse_event(WHEEL, 0, 0, delta, 0)
    pause(280)


def click_widget(widget: QAbstractButton) -> None:
    point = widget.mapToGlobal(widget.rect().center())
    USER.SetCursorPos(point.x(), point.y())
    pause(60)
    mouse(LEFT_DOWN, LEFT_UP)


def button_named(name: str) -> QAbstractButton | None:
    needle = name.casefold()
    for button in window.findChildren(QAbstractButton):
        label = f"{button.text()} {button.toolTip()}".casefold()
        if needle in label and button.isVisible() and button.isEnabled():
            return button
    return None


def click_named(name: str) -> bool:
    button = button_named(name)
    if button is None:
        return False
    click_widget(button)
    return True


def scene_state() -> list[dict]:
    result = []
    for obj in window.scene.objects:
        result.append(
            {
                "id": str(obj.object_id),
                "name": obj.name,
                "visible": bool(obj.visible),
                "transform": obj.transform.tolist(),
            }
        )
    return result


def camera_state() -> dict:
    camera = window.viewport.camera
    return {
        "yaw": float(camera.yaw),
        "pitch": float(camera.pitch),
        "distance": float(camera.dist),
        "center": camera.center.tolist(),
    }


def capture(action: str, theme: str, fixture: str, *, mode: str = "rest", input_route: str = "") -> None:
    activate()
    pause(180)
    pixmap = window.grab()
    path = CAPTURES / f"python-{theme}-{action}.png"
    if not pixmap.save(str(path)):
        raise RuntimeError(f"could not save {path}")
    records.append(
        {
            "action": action,
            "theme": theme,
            "fixture": fixture,
            "mode": mode,
            "input_route": input_route,
            "logical_window_size": [window.width(), window.height()],
            "captured_size": [pixmap.width(), pixmap.height()],
            "device_pixel_ratio": float(window.devicePixelRatioF()),
            "screen_dpi": float(window.screen().logicalDotsPerInch()),
            "camera": camera_state(),
            "selection": sorted(int(index) for index in window.scene.selected_indices),
            "objects": scene_state(),
            "image": path.relative_to(ROOT).as_posix(),
        }
    )


def select_scene(indices: set[int]) -> None:
    window.scene.set_selection(indices)
    window.scene.objects_changed.emit()
    pause(220)


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
    if kind == "grouped":
        window.scene.set_selection({0, 1})
        window.scene.group_selected()
    select_scene({0} if kind != "hidden" else {0, 1})
    window._show_workspace()
    window.viewport.camera.reset_view()
    window.viewport.frame_scene()
    pause(500)
    fixture = FIXTURES / f"python-{kind}.3dp"
    save_project(window.scene, str(fixture))
    return fixture.relative_to(ROOT).as_posix()


def projected_point(point: list[float]) -> tuple[float, float] | None:
    camera = window.viewport.camera
    matrix = camera.get_view_matrix()
    position = camera.get_position()
    vector = matrix @ [point[0], point[1], point[2], 1.0]
    if vector[2] >= -1e-5:
        return None
    focal = 1.0 / math.tan(math.radians(45.0) * 0.5)
    width, height = window.viewport.viewport_size
    return (
        (1.0 + vector[0] * focal / (width / height) / -vector[2]) * width * 0.5,
        (1.0 - vector[1] * focal / -vector[2]) * height * 0.5,
    )


def find_gizmo_point(mode: str, desired: str) -> tuple[float, float]:
    viewport = window.viewport
    for primitive in viewport.gizmo.build_render_data(viewport.camera, viewport.viewport_size):
        vertices = primitive.get("vertices", [])
        step = 3 if primitive.get("kind") == "triangles" else 2
        for offset in range(0, len(vertices), step):
            block = vertices[offset : offset + step]
            if len(block) == 0:
                continue
            point = [sum(float(vertex[index]) for vertex in block) / len(block) for index in range(3)]
            projected = projected_point(point)
            if projected is None:
                continue
            x, y = projected
            if 0 <= x < viewport.width() and 0 <= y < viewport.height():
                origin, direction = viewport.screen_ray(int(x), int(y))
                if viewport.gizmo.hit_test(origin, direction, viewport.camera, viewport.viewport_size) == desired:
                    return float(x), float(y)
    raise RuntimeError(f"could not find visible reference {mode} handle {desired}")


def transform_gizmo(mode: str, handle: str, theme: str, fixture: str) -> None:
    button = getattr(window.transform_bar, f"{mode}_btn", None)
    if button is None:
        raise RuntimeError(f"missing reference transform button {mode}")
    click_widget(button)
    start = find_gizmo_point(mode, handle)
    capture(f"{mode}-rest", theme, fixture, mode="rest", input_route="visible Qt/OpenGL window")
    drag(start, (start[0] + 38, start[1] + 24), LEFT_DOWN, LEFT_UP)
    capture(
        f"{mode}-interaction",
        theme,
        fixture,
        mode="interaction",
        input_route="actual OS primary pointer drag",
    )


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
        pause(500)
        window.scene.clear()
        window._show_workspace()
        window.viewport.camera.reset_view()
        capture("empty-rest", theme, "empty", input_route="native window capture")
        fixture = setup_scene("single")
        capture("single-rest", theme, fixture, input_route="native window capture")
        center = (window.viewport.width() * 0.5, window.viewport.height() * 0.5)
        pointer(*center)
        mouse(LEFT_DOWN, LEFT_UP)
        pause(90)
        mouse(LEFT_DOWN, LEFT_UP)
        capture("double-click-frame", theme, fixture, input_route="actual OS double primary click")
        drag((center[0] - 70, center[1] - 35), (center[0] + 35, center[1] + 20), LEFT_DOWN, LEFT_UP)
        capture("orbit-after", theme, fixture, input_route="actual OS primary pointer drag")
        drag((center[0], center[1]), (center[0] + 46, center[1] + 24), MIDDLE_DOWN, MIDDLE_UP)
        capture("pan-middle-after", theme, fixture, input_route="actual OS middle pointer drag")
        drag((center[0], center[1]), (center[0] - 36, center[1] + 16), RIGHT_DOWN, RIGHT_UP)
        capture("pan-right-after", theme, fixture, input_route="actual OS secondary pointer drag")
        wheel(center[0], center[1], WHEEL_DELTA)
        capture("wheel-after", theme, fixture, input_route="actual OS wheel")
        if not click_named("View"):
            raise RuntimeError("reference View tab was not available")
        pause(180)
        if not click_named("Grid"):
            raise RuntimeError("reference Grid control was not available")
        capture("grid-off", theme, fixture, input_route="actual OS View ribbon button click")
        if not click_named("Grid"):
            raise RuntimeError("reference Grid control did not restore")
        if not click_named("Wireframe"):
            raise RuntimeError("reference Wireframe control was not available")
        if not click_named("X-Ray"):
            raise RuntimeError("reference X-Ray control was not available")
        capture("wireframe-xray", theme, fixture, input_route="actual OS ribbon button clicks")
        if not click_named("Snap"):
            raise RuntimeError("reference Snap control was not available")
        capture("snap-toggled", theme, fixture, input_route="actual OS ribbon button click")
        transform_gizmo("move", "x", theme, fixture)
        fixture = setup_scene("multiple")
        capture("multiple-rest", theme, fixture, input_route="native window capture")
        fixture = setup_scene("grouped")
        capture("grouped-rest", theme, fixture, input_route="native window capture")
        fixture = setup_scene("hidden")
        capture("hidden-rest", theme, fixture, input_route="native window capture")
    RESULTS.write_text(json.dumps(records, indent=2), encoding="utf-8")
except Exception:
    (ROOT / "python-native-error.txt").write_text(traceback.format_exc(), encoding="utf-8")
    raise
finally:
    window.close()
    app.processEvents()
