# Project format fixtures

`python-current.3dp` is a project written by the Python reference
`src.project_io.save_project` serializer. It contains one 2 × 4 × 6 mm box
with a non-identity transform, a non-default RGBA color, metadata, a stable
object ID, and an invisible state.

`python-legacy.3dp` is the same Python-produced archive downgraded to the
unversioned schema. It omits the format marker, project ID, revision, object
ID, transform, color, visibility, and metadata so the loader's legacy defaults
are exercised. `python-future.3dp` changes only the schema version to 99.
`python-invalid-transform.3dp`, `python-missing-mesh.3dp`, and
`python-malformed-json.3dp` cover invalid persisted data and structural
failures. The corresponding contracts are asserted in `tests/project_golden.rs`.
