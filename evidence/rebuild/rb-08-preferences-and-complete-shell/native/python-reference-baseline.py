"""RB-08 native baseline probe for the read-only Python reference."""
from __future__ import annotations

import ctypes
import json
import sys
import time
import traceback
from datetime import datetime, timezone
from pathlib import Path

if __name__ != "__main__":
    raise SystemExit(0)

sys.dont_write_bytecode = True
ROOT = Path("E:/Builder3D-Rust/evidence/rebuild/rb-08-preferences-and-complete-shell")
RUN = ROOT / "native" / ("python-baseline-" + datetime.now().strftime("%Y%m%d-%H%M%S"))
CAPTURES = RUN / "captures"
SETTINGS = RUN / "settings"
CAPTURES.mkdir(parents=True, exist_ok=False)
SETTINGS.mkdir(parents=True, exist_ok=True)

REFERENCE_ROOT = Path("L:/Builder3D")
sys.path.insert(0, str(ROOT.parent / "rb-05-viewport-and-input-completion"))
sys.path.insert(0, str(REFERENCE_ROOT))
from native_desktop_guard import require_default_desktop  # noqa: E402
from src.main import configure_opengl_defaults  # noqa: E402
from src.qt_compat import QSettings  # noqa: E402

# Route QSettings into this evidence run so the reference user's Windows
# registry values remain untouched while its real adapter logic is exercised.
QSettings.setDefaultFormat(QSettings.Format.IniFormat)
QSettings.setPath(QSettings.Format.IniFormat, QSettings.Scope.UserScope, str(SETTINGS))
QSettings.setPath(QSettings.Format.IniFormat, QSettings.Scope.SystemScope, str(SETTINGS))
configure_opengl_defaults()

from PyQt6.QtCore import Qt  # noqa: E402
from PyQt6.QtWidgets import QApplication, QComboBox, QDialog, QMessageBox, QPushButton, QToolButton  # noqa: E402
from PIL import ImageGrab  # noqa: E402
from src.main_window import MainWindow  # noqa: E402

USER = ctypes.windll.user32
KERNEL = ctypes.windll.kernel32
LEFT_DOWN = 0x0002
LEFT_UP = 0x0004
KEY_UP = 0x0002
VK_RETURN = 0x0D
VK_ESCAPE = 0x1B
VK_END = 0x23

class RECT(ctypes.Structure):
    _fields_ = [("left", ctypes.c_long), ("top", ctypes.c_long), ("right", ctypes.c_long), ("bottom", ctypes.c_long)]

class POINT(ctypes.Structure):
    _fields_ = [("x", ctypes.c_long), ("y", ctypes.c_long)]

USER.GetClientRect.argtypes = [ctypes.c_void_p, ctypes.POINTER(RECT)]
USER.ClientToScreen.argtypes = [ctypes.c_void_p, ctypes.POINTER(POINT)]
USER.GetForegroundWindow.restype = ctypes.c_void_p
USER.GetWindowThreadProcessId.argtypes = [ctypes.c_void_p, ctypes.POINTER(ctypes.c_ulong)]
USER.AttachThreadInput.argtypes = [ctypes.c_ulong, ctypes.c_ulong, ctypes.c_bool]
USER.ShowWindowAsync.argtypes = [ctypes.c_void_p, ctypes.c_int]
USER.BringWindowToTop.argtypes = [ctypes.c_void_p]
USER.SetForegroundWindow.argtypes = [ctypes.c_void_p]
KERNEL.GetCurrentThreadId.restype = ctypes.c_ulong

app = QApplication([])
records: list[dict[str, object]] = []
actions: list[dict[str, object]] = []
failures: list[str] = []
window = MainWindow()
window.resize(1280, 840)
window.show()
window.raise_()
window.activateWindow()

def pause(milliseconds: int = 250) -> None:
    app.processEvents()
    time.sleep(milliseconds / 1000.0)
    app.processEvents()

def activate(handle: int | None = None) -> None:
    require_default_desktop()
    handle = handle or int(window.winId())
    foreground = int(USER.GetForegroundWindow() or 0)
    foreground_pid = ctypes.c_ulong()
    foreground_thread = (
        USER.GetWindowThreadProcessId(ctypes.c_void_p(foreground), ctypes.byref(foreground_pid))
        if foreground
        else 0
    )
    current_thread = KERNEL.GetCurrentThreadId()
    attached = bool(foreground_thread and foreground_thread != current_thread)
    if attached:
        USER.AttachThreadInput(foreground_thread, current_thread, True)
    USER.ShowWindowAsync(ctypes.c_void_p(handle), 9)
    USER.BringWindowToTop(ctypes.c_void_p(handle))
    USER.SetForegroundWindow(ctypes.c_void_p(handle))
    if attached:
        USER.AttachThreadInput(foreground_thread, current_thread, False)
    pause(120)
    if int(USER.GetForegroundWindow() or 0) != handle:
        raise RuntimeError("reference target did not become foreground")

def click_screen(x: int, y: int) -> None:
    USER.SetCursorPos(int(x), int(y))
    time.sleep(0.05)
    USER.mouse_event(LEFT_DOWN, 0, 0, 0, 0)
    time.sleep(0.05)
    USER.mouse_event(LEFT_UP, 0, 0, 0, 0)
    pause(250)

def click_widget(widget) -> None:
    activate(int(widget.window().winId()))
    point = widget.mapToGlobal(widget.rect().center())
    click_screen(point.x(), point.y())

def press_key(key: int) -> None:
    USER.keybd_event(key, 0, 0, 0)
    time.sleep(0.04)
    USER.keybd_event(key, 0, KEY_UP, 0)
    pause(150)

def capture(name: str, theme: str, route: str, *, modal: str | None = None) -> None:
    pause(180)
    handle = int(window.winId())
    client = RECT()
    origin = POINT(0, 0)
    if not USER.GetClientRect(ctypes.c_void_p(handle), ctypes.byref(client)):
        raise RuntimeError("GetClientRect failed")
    if not USER.ClientToScreen(ctypes.c_void_p(handle), ctypes.byref(origin)):
        raise RuntimeError("ClientToScreen failed")
    width = client.right - client.left
    height = client.bottom - client.top
    image_path = CAPTURES / f"python-{theme}-{name}.png"
    image = ImageGrab.grab(bbox=(origin.x, origin.y, origin.x + width, origin.y + height))
    image.save(image_path)
    import hashlib
    record = {
        "app": "Python reference",
        "theme": theme,
        "route": route,
        "modal": modal,
        "window_title": window.windowTitle(),
        "logical_client_size": [window.width(), window.height()],
        "captured_size": [image.width, image.height],
        "dpi": float(window.screen().logicalDotsPerInch()),
        "image": image_path.relative_to(ROOT).as_posix(),
        "image_sha256": hashlib.sha256(image_path.read_bytes()).hexdigest(),
        "captured_at_utc": datetime.now(timezone.utc).isoformat(),
    }
    records.append(record)

def action_text(action) -> str:
    return action.text().replace("&", "").strip()

try:
    pause(1200)
    activate()
    assert window._theme_mode == "dark", f"first-run theme was {window._theme_mode!r}"
    capture("welcome-empty", "dark", "first-run empty welcome surface")
    actions.append({"check": "first_run_default_theme", "result": window._theme_mode})
    actions.append({"check": "first_run_recent_files", "result": list(window._recent_files)})

    menu_button = window.findChild(QToolButton, "RibbonHamburger")
    if menu_button is None or menu_button.menu() is None:
        raise RuntimeError("hamburger menu is not reachable")
    click_widget(menu_button)
    menu = menu_button.menu()
    pause(150)
    menu_labels = [action_text(action) for action in menu.actions() if not action.isSeparator()]
    expected = {"New Scene", "Open Project", "Save", "Save As", "Settings", "Exit"}
    if not expected.issubset(set(menu_labels)):
        raise RuntimeError(f"reference menu is missing expected entries: {menu_labels}")
    capture("file-menu", "dark", "native hamburger click", modal="QMenu")
    settings_action = next((action for action in menu.actions() if action_text(action) == "Settings"), None)
    if settings_action is None:
        raise RuntimeError("Settings menu action was not found")
    point = menu.mapToGlobal(menu.actionGeometry(settings_action).center())
    click_screen(point.x(), point.y())
    pause(300)
    dialog = app.activeModalWidget()
    if not isinstance(dialog, QDialog):
        raise RuntimeError("Settings did not open as a native modal dialog")
    button_labels = [
        button.text().strip()
        for button in dialog.findChildren(QPushButton)
        if button.isVisible()
    ]
    combo_items = []
    for combo in dialog.findChildren(QComboBox):
        combo_items.append([combo.itemText(index) for index in range(combo.count())])
    required_buttons = {"Change Slicer...", "Clear Preference", "Clear Recent Files List", "About", "Close"}
    if not required_buttons.issubset(set(button_labels)):
        raise RuntimeError(f"reference Settings controls differ: {button_labels}")
    capture("settings-dark", "dark", "File menu -> Settings", modal="Settings")
    actions.append({"check": "settings_buttons", "result": button_labels})
    actions.append({"check": "settings_combos", "result": combo_items})

    theme_combo = next(
        combo for combo in dialog.findChildren(QComboBox)
        if [combo.itemText(index) for index in range(combo.count())] == ["Dark Mode", "Light Mode"]
    )
    click_widget(theme_combo)
    press_key(VK_END)
    press_key(VK_RETURN)
    pause(450)
    if window._theme_mode != "light":
        raise RuntimeError(f"native theme selection did not apply: {window._theme_mode}")
    capture("settings-theme-light", "light", "Settings appearance combobox -> Light Mode", modal="Settings")
    actions.append({"check": "theme_applies_immediately", "result": window._theme_mode})

    about_button = next(
        button for button in dialog.findChildren(QPushButton)
        if button.text().strip() == "About"
    )
    click_widget(about_button)
    about = app.activeModalWidget()
    if not isinstance(about, QMessageBox):
        about = next((item for item in QApplication.topLevelWidgets() if isinstance(item, QMessageBox) and item.isVisible()), None)
    if about is None:
        raise RuntimeError("About did not open a native message dialog")
    capture("about-from-settings", "light", "Settings -> About", modal="About Builder3D")
    press_key(VK_RETURN)
    pause(200)

    clear_button = next(
        button for button in dialog.findChildren(QPushButton)
        if button.text().strip() == "Clear Recent Files List"
    )
    click_widget(clear_button)
    if window._recent_files:
        raise RuntimeError("Clear Recent Files List left recent paths in memory")
    recent_store = QSettings("Builder3D", "Builder3D").value("recent_files", [])
    if recent_store not in ([], None):
        raise RuntimeError(f"Clear Recent Files List did not persist the empty list: {recent_store!r}")
    actions.append({"check": "clear_recents_persists_empty", "result": recent_store})
    capture("settings-clear-recents", "light", "Settings -> Clear Recent Files List", modal="Settings")

    change_slicer = next(
        button for button in dialog.findChildren(QPushButton)
        if button.text().strip() == "Change Slicer..."
    )
    click_widget(change_slicer)
    chooser = app.activeModalWidget()
    if not isinstance(chooser, QDialog):
        raise RuntimeError("Change Slicer did not open its chooser")
    chooser_buttons = [
        button.text().strip()
        for button in chooser.findChildren(QPushButton)
        if button.isVisible()
    ]
    if not {"Browse for slicer...", "Cancel", "Use Slicer"}.issubset(set(chooser_buttons)):
        raise RuntimeError(f"reference slicer chooser controls differ: {chooser_buttons}")
    capture("slicer-chooser", "light", "Settings -> Change Slicer", modal="Choose 3D Slicer")
    press_key(VK_ESCAPE)
    pause(200)
    close_button = next(button for button in dialog.findChildren(QPushButton) if button.text().strip() == "Close")
    click_widget(close_button)
    pause(300)
    capture("welcome-after-settings", "light", "Settings closed after native actions")

    result_path = RUN / "results.json"
    result_path.write_text(
        json.dumps({
            "slice": "RB-08 baseline",
            "target": "read-only Python reference",
            "reference_root": str(REFERENCE_ROOT),
            "settings_backend": "isolated QSettings IniFormat under evidence/native",
            "captures": records,
            "actions": actions,
            "failures": failures,
        }, indent=2),
        encoding="utf-8",
    )
    if failures:
        raise RuntimeError("; ".join(failures))
    print(json.dumps({"run": str(RUN), "captures": len(records), "actions": actions}, indent=2))
except Exception:
    (RUN / "error.txt").write_text(traceback.format_exc(), encoding="utf-8")
    raise
finally:
    window._set_scene_dirty(False)
    window.close()
    app.quit()
    app.processEvents()




