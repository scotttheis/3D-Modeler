# Native launchers

These launchers are for the private native Builder3D package. They resolve the
executable beside the launcher and pass the user's arguments as an argument
vector. They do not locate or invoke Python, a shell command string, or an
arbitrary executable path.

| Target | File | Required sibling |
| --- | --- | --- |
| Windows Explorer/console | `Builder3D.bat` | `Builder3D.exe` |
| Windows PowerShell | `Builder3D.ps1` | `Builder3D.exe` |
| Linux | `builder3d.sh` | executable `Builder3D` |
| macOS Finder/Terminal | `Builder3D.command` | executable `Builder3D` |
| Linux desktop menu | `builder3d.desktop` | installed `Builder3D` and icon |

Installer scripts preserve these fixed sibling relationships.
