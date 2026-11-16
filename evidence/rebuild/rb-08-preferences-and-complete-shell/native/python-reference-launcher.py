"""Run the real reference MainWindow with an isolated QSettings backend."""
from __future__ import annotations

import os
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, "L:/Builder3D")

from src.qt_compat import QApplication, QSettings, exec_application
from src.main import configure_opengl_defaults
from src.style import (
    DEFAULT_THEME_MODE,
    THEME_MODE_SETTINGS_KEY,
    apply_style,
    normalize_theme_mode,
)
from src.main_window import MainWindow
from src import desktop_adapters

settings_root = Path(os.environ["RB08_REFERENCE_SETTINGS_DIR"])


def isolated_settings(organization: str, application: str):
    """Keep both current and legacy QSettings files inside the evidence run."""

    path = settings_root / f"{organization}-{application}.ini"
    return QSettings(str(path), QSettings.Format.IniFormat)


# QSettings(organization, application) selects the native Windows registry
# format explicitly, so setDefaultFormat/setPath alone do not isolate it.
# Route the production settings adapter through IniFormat files under E:.
desktop_adapters.QSettings = isolated_settings

configure_opengl_defaults()
app = QApplication(sys.argv)
saved_theme = isolated_settings("3D Modeler", "3D Modeler").value(
    THEME_MODE_SETTINGS_KEY,
    DEFAULT_THEME_MODE,
)
apply_style(app, normalize_theme_mode(saved_theme))
window = MainWindow()
window.resize(1280, 840)
window.show()
sys.exit(exec_application(app))

