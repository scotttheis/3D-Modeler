# RB-05 verification

## Automated gates

All gates were run from `E:\Builder3D-Rust` after initializing the pinned build-only libclang environment.

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | pass |
| `cargo check --workspace --locked` | pass |
| `cargo test --workspace --locked` | **265 passed, 0 failed, 0 ignored** |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | pass |
| `cargo deny check` | pass; dependency policy reports advisories/bans/licenses/sources okay |
| `cargo metadata --locked --format-version 1 --no-deps` | pass |
| Focused Python reference suite | **145 passed**: `tests/test_viewport.py`, `tests/test_gizmo.py`, `tests/test_snapping.py` |
| Debug app smoke | pass; `objects=2 selected=1 tabs=5` |
| Release app smoke | pass; `objects=2 selected=1 tabs=5` |
| Explicit app bloat | pass; 11.6 MiB `.text`, 14.3 MiB file reported by cargo-bloat |

The release executable after the explicit bloat build is 15,006,720 bytes with SHA-256 `ddd70cb0e6a560d9395db51dc857187b3011e4f895e943e424567be6a65e0908`. The final native replay intentionally remains tied to the debug executable hash recorded in its journal.

## Native replay gates

| Replay | Records | Themes | Input/evidence | Result |
|---|---:|---|---|---|
| Python reference | 30 | 15 Light / 15 Dark | Visible Qt window; Win32 pointer, middle/right, wheel, and ribbon input | pass |
| Rust viewport/input | 52 | 26 Light / 26 Dark | Visible eframe window; guarded Win32 pointer, wheel, keyboard, project-menu input | pass |
| Accepted PNG integrity | 82 | both | 1280 x 840, 96 DPI; zero missing paths | pass |

The Python capture journal records logical/captured size `[1280, 840]`, DPR `1.0`, and 96 logical DPI. The Rust journal records a 1280 x 840 client and DPI 96 for every accepted frame. The final Rust journal contains one executable SHA-256 only: `4a42ec94338baf986806740e7a59c7fc7db2faa9bdf2027af4d8f6160ac69232`.

The native environment was Windows 11 Enterprise on a Quadro RTX 4000, NVIDIA driver 581.42. The runtime capability report selected a usable discrete GPU path; CPU fallback floor/grid and display paths are covered by viewport tests rather than claimed as a native fallback capture.

## Preservation and limits

- Python source was only imported and exercised read-only. Its working tree already contained unrelated/pre-existing changes; no Python file was edited by RB-05. Relevant source hashes and the observed status are recorded in [source-preservation.md](source-preservation.md).
- G-drive snapshots are additive and immutable. The final receipt for `G:\Builder3D-Rust_snapshot_20261101-064700_RB-05-complete` is recorded in [checkpoint.md](checkpoint.md) and `WORK_LOG.md` after the completed evidence tree is copied and SHA-256 checked.
- Visual comparison, owner acceptance, alternate DPI, installed-package size, and cross-host coverage remain outside this verification claim.
- RB-03 and RB-04 pending statuses remain preserved. RB-06 has not begun.

## Floor scale labels remain attached during pan (2026-11-10)

The previous Rust labels were positioned relative to `camera.center()`. Panning
therefore moved the label anchors with the camera instead of keeping them on the
floor. `build_floor_scale_labels` now places zero at the fixed world origin and
each metric mark at fixed world-axis coordinates. Camera pan changes the view
projection, so the labels move on screen with their floor-grid positions.

The regression `floor_scale_labels_remain_world_anchored_when_camera_pans`
checks that all label values and world positions remain unchanged after a
camera pan. The focused viewport suite passed: 40 passed, 0 failed.
`cargo fmt --all -- --check` and viewport Clippy with warnings denied passed.
The optimized app build and `builder3d-app.exe --smoke` passed
(`objects=2 selected=1 tabs=5`).

In the visible Release app (PID 20188), a right-button drag in the empty scene
from client point `(400, 420)` to `(480, 460)` kept the metric labels aligned
with the grid while the floor moved in the view. The 1280 x 840 client at 96
DPI was responsive after the drag. Before/after captures:

- [Before pan](captures/rust-floor-label-pan-20261110-viewport-before.png) - SHA-256 `C560EC3542EDE1AC3F5DBF750FDA55C0483BBD318DB2806A43CBAEE93CDE96CB`
- [After pan](captures/rust-floor-label-pan-20261110-viewport-after.png) - SHA-256 `27123D2318E58377D29A2C35396C3CB6E4B2CB79754C712FA4BFBB8C3556B61B`

The native check ran on Windows 11 Enterprise with the Quadro RTX 4000 and
NVIDIA driver 581.42. The Release window remains open for owner review. This
focused check supplements the historical RB-05 replay; it does not alter its
capture count or establish overall RB-05 or product acceptance.
