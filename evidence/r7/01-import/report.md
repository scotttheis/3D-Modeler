# R7-01 — desktop model import command coverage

Component: `builder3d.app.commands.coverage`

## Selection

The catalog identified `builder3d.app.commands.coverage` as the next follow-up
from the ported application command surface. The shell entry remains `polish`
and was not expanded. The first concrete coverage gap was the Insert tab's
`Add/Open` action: before this slice it opened the project dialog and could not
insert a mesh file into the current scene.

## Implemented contract

- Added typed `CommandId::ImportModel`, `DialogKind::ImportModel`, and a
  path/unit import dialog. The existing `OpenProject` command remains limited
  to `.3dp` projects.
- Added the bounded `ImportUnit` choices (`mm`, `cm`, `m`, `in`, `ft`) for
  unitless STL, OBJ, and PLY files. glTF/GLB and 3MF use their format-owned
  millimetre conversion.
- Reused the core loaders for STL/OBJ/PLY, glTF/GLB, and 3MF. Imported item
  names, representative colors, 3MF metadata, and scene transforms cross the
  application seam without putting format parsing in the shell.
- Centers the imported world bounds with the reference build-plane placement
  contract before committing the scene objects.
- Commits all imported items, transforms, and selection as one
  `GroupedCommand`; a loader or validation failure occurs before mutation, and
  the grouped command rolls back if a later command fails.
- Added focused tests for the Add/Open ribbon identity, dialog defaults,
  unit scaling and placement, atomic selection/history, and unsupported WRL
  rejection.

The Python tree at `L:\Builder3D` was read as the behavioral reference only;
no Python source or existing snapshot was modified.

## Deliberate boundary

This slice covers the semantic import command. Background import progress,
viewport preview preparation, recent-file bookkeeping, and the remaining
selection/workflow coverage are separate catalog slices. No visual polish was
added while conversion coverage is incomplete.
