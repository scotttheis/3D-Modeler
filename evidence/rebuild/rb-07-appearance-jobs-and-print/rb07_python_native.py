"""RB-07 paired Python reference capture using native Windows input/screen."""

from __future__ import annotations

import ctypes
import json
import sys
import time
import traceback
from pathlib import Path

if __name__ != "__main__":
    raise SystemExit(0)

sys.dont_write_bytecode = True
ROOT = Path("E:/Builder3D-Rust/evidence/rebuild/rb-07-appearance-jobs-and-print")
CAPTURES = ROOT / "captures"
FIXTURES = ROOT / "fixtures"
CAPTURES.mkdir(exist_ok=True)
FIXTURES.mkdir(exist_ok=True)

RB05_ROOT = Path("E:/Builder3D-Rust/evidence/rebuild/rb-05-viewport-and-input-completion")
sys.path.insert(0, str(RB05_ROOT))
from native_desktop_guard import require_default_desktop  # noqa: E402

sys.path.insert(0, "L:/Builder3D")
from src.main import configure_opengl_defaults  # noqa: E402

configure_opengl_defaults()

from PyQt6.QtCore import QPoint, QTimer  # noqa: E402
from PyQt6.QtGui import QColor  # noqa: E402
from PyQt6.QtWidgets import QApplication, QAbstractButton, QColorDialog, QFileDialog, QLabel, QWidget  # noqa: E402
from PIL import Image, ImageGrab  # noqa: E402

from src.main_window import MainWindow  # noqa: E402
from src.mesh_ops import make_box  # noqa: E402
from src.qt_compat import QMessageBox  # noqa: E402

USER = ctypes.windll.user32
KERNEL = ctypes.windll.kernel32
USER.GetForegroundWindow.restype = ctypes.c_void_p
USER.GetWindowThreadProcessId.argtypes = [ctypes.c_void_p, ctypes.POINTER(ctypes.c_ulong)]
USER.AttachThreadInput.argtypes = [ctypes.c_ulong, ctypes.c_ulong, ctypes.c_bool]
USER.ShowWindowAsync.argtypes = [ctypes.c_void_p, ctypes.c_int]
USER.BringWindowToTop.argtypes = [ctypes.c_void_p]
USER.SetForegroundWindow.argtypes = [ctypes.c_void_p]
KERNEL.GetCurrentThreadId.restype = ctypes.c_ulong
LEFT_DOWN = 0x0002
LEFT_UP = 0x0004
KEY_UP = 0x0002
VK_RETURN = 0x0D
VK_ESCAPE = 0x1B


class RECT(ctypes.Structure):
    _fields_ = [("left", ctypes.c_long), ("top", ctypes.c_long), ("right", ctypes.c_long), ("bottom", ctypes.c_long)]


class POINT(ctypes.Structure):
    _fields_ = [("x", ctypes.c_long), ("y", ctypes.c_long)]


USER.GetClientRect.argtypes = [ctypes.c_void_p, ctypes.POINTER(RECT)]
USER.ClientToScreen.argtypes = [ctypes.c_void_p, ctypes.POINTER(POINT)]


class MemorySettings:
    """Keep the evidence runner's theme and slicer choices out of user settings."""

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


def pause(milliseconds: int = 250) -> None:
    app.processEvents()
    time.sleep(milliseconds / 1000.0)
    app.processEvents()


def activate_handle(handle: int) -> None:
    require_default_desktop()
    foreground = int(USER.GetForegroundWindow() or 0)
    foreground_pid = ctypes.c_ulong()
    foreground_thread = USER.GetWindowThreadProcessId(
        ctypes.c_void_p(foreground), ctypes.byref(foreground_pid)
    ) if foreground else 0
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


def activate() -> None:
    activate_handle(int(window.winId()))


def click_screen(x: int, y: int) -> None:
    USER.SetCursorPos(int(x), int(y))
    time.sleep(0.05)
    USER.mouse_event(LEFT_DOWN, 0, 0, 0, 0)
    time.sleep(0.05)
    USER.mouse_event(LEFT_UP, 0, 0, 0, 0)


def click_client(x: int, y: int, *, activate_window: bool = True) -> None:
    handle = ctypes.c_void_p(int(window.winId()))
    point = POINT(int(x), int(y))
    if not USER.ClientToScreen(handle, ctypes.byref(point)):
        raise RuntimeError("could not map native client input to the desktop")
    if activate_window:
        activate()
    click_screen(point.x, point.y)
    pause(300)


def press_key(key: int) -> None:
    USER.keybd_event(key, 0, 0, 0)
    time.sleep(0.05)
    USER.keybd_event(key, 0, KEY_UP, 0)


def click_widget(widget: QWidget) -> None:
    activate_handle(int(widget.window().winId()))
    point = widget.mapToGlobal(widget.rect().center())
    click_screen(point.x(), point.y())
    pause(300)


def visible_buttons() -> list[QWidget]:
    return [
        widget
        for widget in window.findChildren(QWidget)
        if callable(getattr(widget, "click", None))
    ]


def button_named(name: str) -> QWidget | None:
    needle = name.casefold()
    for button in visible_buttons():
        text_getter = getattr(button, "text", None)
        labels = [text_getter() if callable(text_getter) else "", button.toolTip()]
        command_text = getattr(button, "_command_text", "")
        if command_text:
            labels.append(command_text)
        labels.extend(label.text() for label in button.findChildren(QLabel))
        label = " ".join(labels).casefold()
        if needle in label and button.isVisible() and button.isEnabled():
            return button
    return None


def click_named(name: str) -> None:
    button = button_named(name)
    if button is None:
        controls = [
            {
                "text": button.text() if callable(getattr(button, "text", None)) else "",
                "tooltip": button.toolTip(),
                "object_name": button.objectName(),
                "visible": button.isVisible(),
                "enabled": button.isEnabled(),
            }
            for button in visible_buttons()
            if button.isVisible()
        ]
        ribbon = window.ribbon
        catalog = {
            "active_tab": getattr(ribbon, "_active_tab_name", None),
            "command_row_visible": not ribbon.command_container.isHidden(),
            "buttons": controls,
            "command_row_children": [
                {
                    "type": type(child).__name__,
                    "object_name": child.objectName(),
                    "text": getattr(child, "text", lambda: "")(),
                    "visible": child.isVisible(),
                }
                for child in ribbon.command_container.children()
                if isinstance(child, QWidget)
            ],
        }
        (ROOT / "python-control-catalog.json").write_text(json.dumps(catalog, indent=2), encoding="utf-8")
        raise RuntimeError(f"reference button is not reachable: {name}; controls cataloged")
    click_widget(button)


def click_tab(name: str) -> None:
    button = button_named(name)
    if button is None:
        raise RuntimeError(f"reference ribbon tab is not reachable: {name}")
    if getattr(window.ribbon, "_active_tab_name", None) != name:
        click_widget(button)
    if getattr(window.ribbon, "_active_tab_name", None) != name:
        raise RuntimeError(f"native click did not activate ribbon tab {name}")


def capture(action: str, theme: str, route: str, *, modal: str | None = None) -> str:
    pause(120)
    handle = ctypes.c_void_p(int(window.winId()))
    client = RECT()
    origin = POINT(0, 0)
    if not USER.GetClientRect(handle, ctypes.byref(client)) or not USER.ClientToScreen(handle, ctypes.byref(origin)):
        raise RuntimeError("could not resolve the native reference client rectangle")
    width = client.right - client.left
    height = client.bottom - client.top
    pixmap = ImageGrab.grab(
        bbox=(origin.x, origin.y, origin.x + width, origin.y + height)
    )
    path = CAPTURES / f"python-{theme}-{action}.png"
    pixmap.save(path)
    record = {
        "app": "Python reference",
        "theme": theme,
        "action": action,
        "input_route": route,
        "logical_window_size": [window.width(), window.height()],
        "captured_size": [pixmap.width, pixmap.height],
        "physical_client_size": [width, height],
        "screen_dpi": float(window.screen().logicalDotsPerInch()),
        "scene_objects": len(window.scene.objects),
        "selection": sorted(int(index) for index in window.scene.selected_indices),
        "colors": [obj.color.tolist() for obj in window.scene.get_selected_objects()],
        "status": window.statusBar().currentMessage(),
        "modal": modal,
        "image": path.relative_to(ROOT).as_posix(),
    }
    records.append(record)
    return record["image"]


def modal_window(kind):
    active = app.activeModalWidget()
    if active is not None and isinstance(active, kind):
        return active
    return next(
        (
            widget
            for widget in QApplication.topLevelWidgets()
            if isinstance(widget, kind) and widget.isVisible()
        ),
        None,
    )


def schedule_color_choice(theme: str, action: str, *, cancel: bool = False) -> None:
    deadline = time.monotonic() + 10.0

    def handle():
        dialog = modal_window(QColorDialog)
        if dialog is None:
            if time.monotonic() >= deadline:
                failures.append(f"{theme}-{action}: QColorDialog did not appear")
                return
            QTimer.singleShot(100, handle)
            return
        capture(action, theme, "Paint tab -> Material -> native QColorDialog", modal="QColorDialog")
        if not cancel:
            dialog.setCurrentColor(QColor(70, 125, 220, 255))
        press_key(VK_ESCAPE if cancel else VK_RETURN)

    QTimer.singleShot(450, handle)


def schedule_file_choice(theme: str, action: str, path: Path) -> None:
    deadline = time.monotonic() + 12.0

    def handle():
        dialog = modal_window(QFileDialog)
        if dialog is None:
            if time.monotonic() >= deadline:
                failures.append(f"{theme}-{action}: QFileDialog did not appear")
                return
            QTimer.singleShot(100, handle)
            return
        capture(action + "-picker", theme, "Paint tab -> Color -> native image file chooser", modal="QFileDialog")
        dialog.setDirectory(str(path.parent))
        dialog.selectFile(path.name)
        # The selection is made in the real file chooser and accepted by a real
        # Windows keyboard event; the application handler performs the sample.
        QTimer.singleShot(180, lambda: press_key(VK_RETURN))

    QTimer.singleShot(500, handle)


def schedule_message_choice(theme: str, action: str) -> None:
    deadline = time.monotonic() + 10.0

    def handle():
        dialog = modal_window(QMessageBox)
        if dialog is None:
            if time.monotonic() >= deadline:
                failures.append(f"{theme}-{action}: message dialog did not appear")
                return
            QTimer.singleShot(100, handle)
            return
        capture(action, theme, "File menu -> Print Prep Check", modal=dialog.windowTitle())
        press_key(VK_RETURN)

    QTimer.singleShot(450, handle)


def schedule_print_prep_menu(theme: str) -> None:
    deadline = time.monotonic() + 10.0

    def handle():
        menu = window.ribbon._menu_button.menu()
        if not menu.isVisible():
            if time.monotonic() >= deadline:
                failures.append(f"{theme}-print-prep: File menu did not open")
                return
            QTimer.singleShot(100, handle)
            return
        action = next(
            (item for item in menu.actions() if item.text().replace("&", "").strip() == "Print Prep Check"),
            None,
        )
        if action is None:
            failures.append(f"{theme}-print-prep: menu actions were {[item.text() for item in menu.actions()]}")
            return
        capture("file-menu-open", theme, "native File menu -> Print Prep Check", modal="QMenu")
        point = menu.mapToGlobal(menu.actionGeometry(action).center())
        click_screen(point.x(), point.y())

        def dispatch_if_click_missed():
            if modal_window(QMessageBox) is not None:
                return
            dispatch_notes.append(
                f"{theme}: Print Prep Check QAction dispatched after native menu click did not open its report"
            )
            if menu.isVisible():
                menu.close()
            action.trigger()

        QTimer.singleShot(650, dispatch_if_click_missed)

    QTimer.singleShot(350, handle)


def setup_scene() -> None:
    window.scene.clear()
    window.scene.add_object(make_box((20, 30, 40)), "RB-07 Box A")
    window.scene.objects[-1].translate(0, 15, 0)
    window.scene.add_object(make_box((10, 20, 30)), "RB-07 Box B")
    window.scene.objects[-1].translate(40, 10, 0)
    window.scene.set_selection({0, 1})
    window.scene._emit_objects_changed()
    window._show_workspace()
    window.viewport.camera.reset_view()
    window.viewport.frame_scene()
    window.viewport.update()
    pause(450)


app = QApplication([])
records: list[dict] = []
failures: list[str] = []
dispatch_notes: list[str] = []
sample_path = FIXTURES / "color-sample.png"
corrupt_path = FIXTURES / "corrupt-image.png"
sample = Image.new("RGB", (20, 20), (20, 60, 220))
for x in range(10):
    for y in range(20):
        sample.putpixel((x, y), (220, 40, 20))
sample.save(sample_path)
corrupt_path.write_bytes(b"RB-07 corrupt image fixture\x00\x01")

window = MainWindow()
window._settings = MemorySettings()
window.resize(1280, 840)
window.show()
window.raise_()
window.activateWindow()

try:
    pause(1400)
    activate()
    for theme in ("light", "dark"):
        window._set_theme_mode(theme)
        setup_scene()
        capture("workspace", theme, "native reference window; two selected printable objects")

        click_tab("Paint")
        capture("paint-tab-actions", theme, "native Paint tab click")
        schedule_color_choice(theme, "material-color-dialog", cancel=False)
        click_client(52, 100)
        capture("material-color-applied", theme, "Paint tab -> Material -> choose and apply solid color")

        schedule_color_choice(theme, "material-cancel-dialog", cancel=True)
        click_client(136, 100)
        capture("material-cancel-kept-color", theme, "Paint tab -> Paint -> cancel native QColorDialog")

        schedule_file_choice(theme, "corrupt-image", corrupt_path)
        click_client(220, 100)
        capture("corrupt-image-visible-error", theme, "Paint tab -> Color -> choose corrupt image")

        schedule_file_choice(theme, "sample-image", sample_path)
        click_client(220, 100)
        capture("sample-image-applied-solid-color", theme, "Paint tab -> Color -> choose image and sample average")

        menu_button = getattr(window, "_menu_button", None) or window.findChild(QAbstractButton, "RibbonHamburger")
        if menu_button is None:
            menu_button = next((b for b in visible_buttons() if "menu" in b.toolTip().casefold()), None)
        if menu_button is None:
            raise RuntimeError("reference File menu button is not reachable")
        schedule_message_choice(theme, "print-prep-report")
        schedule_print_prep_menu(theme)
        click_widget(menu_button)
        pause(500)

        setup_scene()

    out = ROOT / "python-native-results.json"
    out.write_text(
        json.dumps({"captures": records, "failures": failures, "dispatch_notes": dispatch_notes}, indent=2),
        encoding="utf-8",
    )
    if failures:
        raise RuntimeError("; ".join(failures))
except Exception:
    (ROOT / "python-native-error.txt").write_text(traceback.format_exc(), encoding="utf-8")
    (ROOT / "python-native-results.json").write_text(
        json.dumps({"captures": records, "failures": failures, "dispatch_notes": dispatch_notes}, indent=2),
        encoding="utf-8",
    )
    raise
finally:
    window._set_scene_dirty(False)
    window.close()
    app.quit()
    app.processEvents()
