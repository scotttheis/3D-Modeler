# Native installers

For the Windows MSI handed to testers, see `docs/packaging.md` and
`packaging/msi/`. The scripts below are the older script-based installers.

The package contains platform-specific installation and removal scripts. They
operate only on their documented private 3D Modeler destination by default and
refuse the broad system roots. The scripts expect a native `Modeler3D.exe` or
`Modeler3D` at the package root; they never install Python or a Python
environment.

The Windows installer defaults to `%LOCALAPPDATA%\Modeler3D`. Linux defaults
to `$HOME/.local/opt/modeler3d`; macOS defaults to `$HOME/Applications/Modeler3D`.
An explicit `MODELER3D_INSTALL_ROOT` is accepted only by the POSIX installers
and is still checked against the uninstaller's private default before removal.
