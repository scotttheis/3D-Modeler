# R7-02 — desktop selection and clipboard command coverage

Component: `modeler3d.app.commands.selection`

## Selection

The catalog's next command follow-up covered the object-panel actions that
were still absent from the portable application seam: select all, clear and
invert selection, group/ungroup, and cut. The shell remains conversion-first;
this slice adds functional controls without beginning visual polish.

## Implemented contract

- Added typed command identifiers and dispatch for `SelectAll`, `DeselectAll`,
  `InvertSelection`, `Group`, `Ungroup`, and `Cut`.
- Selection-only commands update the top-level selection without creating
  model history entries.
- Group and ungroup delegate to the existing core scene graph commands, so
  object identity, child membership, selection, undo, and redo remain core
  responsibilities.
- Cut copies the selected scene objects and removes them through the core
  grouped scene command; paste continues to restore the clipboard through the
  existing path.
- Added bounded object-panel buttons for the converted actions and routed
  their results through `CommandState`.
- Added a focused test covering selection history isolation, group/ungroup,
  cut, and paste.

The Python tree at `L:\Builder3D` remained a read-only behavioral reference;
no Python source or existing snapshot was modified.

## Deliberate boundary

This slice does not add transform-editor controls, print-preparation workflow,
recent-file integration, background import/preview, or broad shell polish.
