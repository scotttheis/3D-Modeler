# Native installers

For the Windows MSI handed to testers, see `docs/packaging.md` and
`packaging/msi/`. The scripts below are the older script-based installers.

The package contains platform-specific installation and removal scripts. They
operate only on their documented private Builder3D destination by default and
refuse the broad system roots. The scripts expect a native `Builder3D.exe` or
`Builder3D` at the package root; they never install Python or a Python
environment.

The Windows installer defaults to `%LOCALAPPDATA%\Builder3D`. Linux defaults
to `$HOME/.local/opt/builder3d`; macOS defaults to `$HOME/Applications/Builder3D`.
An explicit `BUILDER3D_INSTALL_ROOT` is accepted only by the POSIX installers
and is still checked against the uninstaller's private default before removal.
