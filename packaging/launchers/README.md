# Native launchers

These launchers are for the private native 3D Modeler package. They resolve the
executable beside the launcher and pass the user's arguments as an argument
vector. They do not locate or invoke Python, a shell command string, or an
arbitrary executable path.

| Target | File | Required sibling |
| --- | --- | --- |
| Windows Explorer/console | `Modeler3D.bat` | `Modeler3D.exe` |
| Windows PowerShell | `Modeler3D.ps1` | `Modeler3D.exe` |
| Linux | `modeler3d.sh` | executable `Modeler3D` |
| macOS Finder/Terminal | `Modeler3D.command` | executable `Modeler3D` |
| Linux desktop menu | `modeler3d.desktop` | installed `Modeler3D` and icon |

Installer scripts preserve these fixed sibling relationships.
