# R7-09 — application preferences and recent-file coverage

Component: `modeler3d.app.preferences`

## Implemented contract

- Added pure, adapter-independent preferences for default ambiguous-import
  units, transform display units, ribbon pinning/favorites/order, and recent
  files.
- Recent files are newest-first, deduplicated, removable, clearable, and
  bounded to the reference ten entries.
- Import-unit resolution preserves authoritative 3MF/glTF/GLB units and uses
  explicit overrides or the persisted default for unitless formats.
- Connected command-state open/import/save paths to the recent-file list and
  made default imports honor the application preference.
- Added focused tests for unit normalization, authoritative suffixes, bounded
  recent ordering, ribbon preference deduplication, and display conversion.

Persistence remains an adapter concern; no Qt/settings backend was introduced
while shell polish is deferred.

No Python source, R1–R6 evidence, or existing snapshot was modified.
