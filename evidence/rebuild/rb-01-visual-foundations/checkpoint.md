# RB-01 Checkpoint

- **Objective:** Finish safe RB-01 live evidence and stop before RB-02.
- **Status:** Four matched live captures and 1:1 comparisons are complete and inspected. Owner visual review and explicit approval remain pending.
- **Capture state:** Empty Welcome with Insert active; no recent projects, selected object, or scene. All four client areas measured 1280 × 840 logical and physical pixels at 96 DPI (100%).
- **Host:** Primary display 2560 × 1440 physical pixels; work area 2560 × 1392. Per-window DPI was 96.
- **Serial workflow:** Python Dark, Python Light, Rust Dark, Rust Light, with each app closed before the next session. Background MCP gateway left running and untouched.
- **Isolation:** Python QSettings and runtime paths and Rust app-data paths were isolated. The L:\Builder3D source and user settings were not modified.
- **Build:** cargo build --package builder3d-app --locked succeeded. No tests were run.
- **Review findings:** The comparisons show different Welcome alignment and Recent layout, the right panel is present only in Python's captured Welcome state, and the Rust hamburger appears as an outlined square. See capture-board.md.
- **Limitations:** New/Cube, native import, shaded viewport, exact pick, gizmo move, undo, and save/reopen were not exercised. No selected-scene or transform-strip state was captured.
- **Approval gate:** RB-02 is not started. Await owner review of the matched images and explicit approval before proceeding.
- **Recovery:** Existing G: snapshots remain unchanged. Planned additive snapshot: G:\Builder3D-Rust_snapshot_20261031-081447_RB-01-live-captures. It will include Git metadata, source, docs, evidence, and this work log; exclude only root target/, root .cache/, and reparse entries. Verify Robocopy status, relative paths, file count, bytes, and every path/size/SHA-256 tuple, then append the receipt after copying.
- **Next permitted work:** Owner review and approval of this RB-01 evidence. Do not start RB-02 in this checkpoint.