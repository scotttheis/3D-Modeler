# RB-06 source preservation

The Python product at `L:\Builder3D` was used as a read-only behavioral and
test reference. The focused test command used its existing
`L:\Builder3D\venv` Python 3.11.9 environment with `-B` and pytest's cache
provider disabled. The native reference script wrote only RB-06-owned output
under this Rust evidence directory; it did not write Python source, tests,
configuration, or caches.

The Rust work is confined to `E:\3D-Modeler` source, tests, documentation,
and additive evidence. `target/`, `.cache/`, and logs are excluded from the
verified snapshot. Existing `G:\` snapshot directories were not changed; the
RB-06 complete snapshot is a new additive directory.

`git -C E:\3D-Modeler status --short --branch` was inspected only. The
repository has no commits yet and its existing files are untracked; no
`git add`, staging, commit, reset, or checkout was performed. The Python tree's
pre-existing dirty state was not normalized or changed.

Python native evidence is limited to six baseline records. The broader Python
operation behavior is represented by the read-only 272-test result, while Rust
native evidence covers the repaired button and input routes. Visual review and
owner acceptance are deliberately left pending.
