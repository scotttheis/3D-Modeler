# Tests for the ledger gaps (finish plan E3)

The behaviour ledger says no test covers: Center View, Frame (selection), the
Snap toggle, and the window-icon load. Add focused unit tests (no behaviour
change, no new features):

1. `CommandId::CenterView` (commands.rs ~1028): runs with and without objects,
   sets the view/status as the code does; one history rule: no undo entry.
2. `CommandId::FrameSelection` (~1034): errors with `SelectionRequired("Frame")`
   when nothing is selected; with a selection sets `ViewOrientation::FrameSelection`.
3. Snap toggle: find the command/setting that toggles snapping; assert it flips,
   persists to settings as the code does, and adds no undo entry.
4. Window icon: assert the embedded window icon bytes decode to a non-empty
   RGBA image of the expected size (find the loader in the app crate).

Follow the existing test style in the same files. If a test finds a real bug,
do not fix it: stop and report. Run only: `cargo fmt --check`,
`cargo clippy -p modeler3d-app --all-targets -- -D warnings`,
`cargo test -p modeler3d-app <new test names>`; for cargo on Windows run
`. .\scripts\Initialize-BuildClang.ps1` first, and redirect cargo output with
`cmd /c "cargo ... > file 2>&1"` (PowerShell wraps stderr as errors).
Do not commit. Tier: feature (Sonnet).
