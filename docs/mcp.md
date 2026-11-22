# Using 3D Modeler with an AI assistant (MCP)

3D Modeler can be driven by an AI assistant through the Model Context Protocol
(MCP). The assistant uses the **same tools you click**, one command at a time,
and every change can be undone from the desktop. The catalog has **76 tools**
(contract version `2026-11-17.1`).

## How it fits together

```
AI client  <--stdio-->  r4_mcp (gateway)  <--loopback HTTP-->  3D Modeler desktop app
```

- The desktop app starts a small server on `127.0.0.1` (this computer only)
  and writes a descriptor file, `mcp-runtime.json`, in its settings folder.
- `r4_mcp` is a small program the AI client starts. It reads the descriptor,
  checks the app is alive and answers MCP calls by forwarding them to the app.
- Both the desktop and the MCP call the same command layer, so a result is the
  same either way, in millimetres.

The app must be running. If it is not, tools answer that the desktop is unavailable.

## Setup

1. Build the gateway from source (it is not in the installers yet):
   `cargo build --release -p modeler3d-mcp` gives `target/release/r4_mcp`
   (`r4_mcp.exe` on Windows).
2. Start 3D Modeler.
3. Add the gateway to your MCP client. For Claude Desktop or Claude Code, an
   entry like this:

```json
{
  "mcpServers": {
    "modeler3d": {
      "command": "E:\\3D-Modeler\\target\\release\\r4_mcp.exe",
      "env": {
        "MODELER3D_MCP_PROFILE": "model",
        "MODELER3D_AI_APPROVED_ROOTS": "C:\\Users\\you\\Documents\\Models"
      }
    }
  }
}
```

| Variable | Meaning |
|---|---|
| `MODELER3D_MCP_PROFILE` | `observe`, `model`, `files` or `full`. Default is `full`. |
| `MODELER3D_AI_APPROVED_ROOTS` | Folders the file tools may use (separated by `;` on Windows, `:` elsewhere). Unset means **no file access**. |
| `MODELER3D_RUNTIME_FILE` | Optional: a different path for the descriptor file. |

## Profiles

Each profile includes everything in the one before it.

| Profile | Tools | What it allows |
|---|---|---|
| `observe` | 10 | Look only: status, scene summary, print check, mesh health, measure, view state, settings, update status. |
| `model` | 62 | Adds everything that edits the model or the view, and undo/redo. |
| `files` | 73 | Adds file and project tools, inside approved folders only. |
| `full` | 76 | Adds launching a slicer, quitting the app and answering update prompts. |

Start with `observe` or `model` and move up only when you need to.

## Tools by group

- **Inspection:** `get_status`, `scene_summary`, `scene_changes`, `print_check`, `mesh_health`, `measure`
- **Operations:** `operation_status`, `cancel_operation`
- **Creation and selection:** `add_primitive`, `select`
- **Transforms:** `apply_actions`, `translate`, `set_transform`, `transform_selection`, `snap_translate`, `set_snapping`, `settle`, `mirror`, `align`, `arrange`
- **Mesh editing:** `boolean_merge`, `boolean_subtract`, `boolean_intersect`, `hollow`, `emboss_text`, `repair`, `simplify`, `subdivide`, `smooth`, `smooth_brush`, `elastic_grab`, `crease_polish`, `blob`, `normalize_face_density`, `set_torus_parameters`
- **Organization:** `delete_selected`, `copy_selected`, `cut_selected`, `paste`, `duplicate_selected`, `group_selected`, `ungroup_selected`, `rename_object`, `set_visibility`
- **Appearance:** `set_color`, `paint`, `apply_texture`, `remove_texture`, `sample_image_color`
- **History:** `undo`, `redo`
- **View:** `view_state`, `set_view`, `frame_view`, `orbit_view`, `pan_view`, `zoom_view`, `view_preset`
- **Project and files:** `new_project`, `open_project`, `save_project`, `save_project_as`, `import_file`, `export_file`, `recent_files`, `open_recent`, `capture_window`
- **Print:** `discover_slicers`, `launch_slicer`
- **Application:** `app_settings`, `set_app_settings`, `customize_ribbon`, `show_screen`, `get_update_status`, `respond_to_update`, `exit_app`

Long operations (such as a boolean on a dense model) return an operation id;
poll it with `operation_status` or stop it with `cancel_operation`.

## Safety

- Loopback only; the descriptor carries a random secret the gateway must present.
- File tools refuse any path outside the approved folders.
- The MCP never simulates mouse or keyboard input; it asks for the result (for
  example "subtract this from that"), the way the buttons do.
- One undo entry per operation, and no entry when nothing changed.
- Updates cannot be pointed at a client-supplied URL or command.

## Example session

> "Add a cube and a cylinder, subtract the cylinder from the cube, check it
> prints, and export 3MF to my Models folder."

1. `add_primitive` (cube), `add_primitive` (cylinder), `set_transform` to place them
2. `select` the cylinder, then `boolean_subtract`
3. `print_check` and, if it reports problems, `repair`
4. `export_file` with a `.3mf` path inside an approved folder (profile `files` or `full`)
