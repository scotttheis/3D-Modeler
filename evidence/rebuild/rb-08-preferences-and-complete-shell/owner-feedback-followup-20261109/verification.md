# Owner visual feedback follow-up — 2026-11-09

## Reference used

- Compared the supplied `erros/Floor 3D Builder 45 degree.png` floor reference and the supplied Rust issue screenshots.
- Compared the saved Python dark-shell captures under `native/python-live-baseline-20261102-094708/captures/`. The Python app was not launched or modified.

## Repair

- Corrected the camera near plane that clipped the near half of the empty-scene floor.
- Kept the light 50 mm checker floor, metric labels, and gradual far-side fade.
- Retained the compact shortcut strip with names and short descriptions, drag reordering, right-click pin/unpin, tooltips, three-line dropdown menu, and selection actions in the side panel.

## Live Release captures

- `release-new-scene-20261109.png` — before the near-plane fix, showing the floor cutoff.
- `release-final-open-20261109.png` — after the fix, left open on the Side view.
- `release-top-view-after-floorfix-20261109.png` — Top view floor coverage.
- `release-side-view-after-floorfix-20261109.png` — Side view floor coverage.
- `release-menu-open-20261109.png` — three-line menu and dropdown.
- `release-welcome-focused-20261109.png` — Release window foreground check.

## Checks

- `cargo test -p builder3d-viewport near_build_floor_stays_inside_projection_for_an_empty_scene --locked` — passed.
- `cargo test -p builder3d-app mcp_bridge::tests::loopback_bridge_authenticates_queues_and_removes_only_its_descriptor --locked -- --exact` — passed. The first workspace run had one loopback connection reset; this isolated test and the repeat workspace run passed.
- `cargo test --workspace --locked` — passed on retry.
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` — passed.
- `cargo fmt --all -- --check` — passed.
- `cargo deny check` — passed; it reported duplicate-version warnings and a yanked locked crate, with advisories, bans, licenses, and sources passing.
- `cargo build --release -p builder3d-app --bin builder3d-app --locked` — passed. The Release GUI launched and remained responsive.

## Window stacking follow-up

The owner clarified that only the splash screen should stay in front of other
windows. The main Release window had been temporarily set to stay in front for
review; it was reopened normally and checked to confirm it is not topmost. It
is left open for the next review session. No app source change was needed, and
splash-screen behavior was not changed.

## Recovery copy

The pre-edit snapshot and its file/byte counts and critical hashes are recorded in [`recovery-snapshot.json`](recovery-snapshot.json). It is a recovery copy, not an RB-08 or product completion snapshot.

This evidence records the implemented changes and the current Release captures. It does not record owner visual acceptance. RB-08's native matrix and overall product acceptance remain open; RB-10 has not started.
