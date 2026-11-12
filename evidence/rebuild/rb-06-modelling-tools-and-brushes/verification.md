# RB-06 verification

All commands below were run from `E:\Builder3D-Rust` on 2026-11-01. The pinned
build-only libclang path was set for Rust commands that compile the Qhull
binding.

## Automated gates

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | pass |
| `cargo check --workspace --locked` | pass |
| `cargo test --workspace --locked` | pass; 271 tests across the workspace, 0 failed/ignored |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | pass |
| `cargo deny check` | pass; advisories, bans, licenses, and sources okay; existing duplicate-version warnings remain informational |
| `cargo metadata --locked --format-version 1 --no-deps` | pass |
| `cargo build --locked -p builder3d-app --bin builder3d-app --release` | pass |
| `cargo bloat --release --bin builder3d-app -n 20` | pass; 11.7 MiB `.text`, 14.4 MiB reported file |
| Debug `builder3d-app.exe --smoke` | pass; `objects=2 selected=1 tabs=5` |
| Release `builder3d-app.exe --smoke` | pass; `objects=2 selected=1 tabs=5` |

The final debug executable used by native evidence is 61,454,336 bytes with
SHA-256 `82284b6fd85221df3a804790cafb1171c469c5cb614460cb73199068ac551d14`.
The final release executable is 15,055,872 bytes with SHA-256
`874b2076060a29cdadcee2497c2da2ba3a178048623508f9dddd99c72de2879a`.

## Python reference gates

The read-only command was:

```powershell
$env:PYTHONDONTWRITEBYTECODE = '1'
& 'L:\Builder3D\venv\Scripts\python.exe' -B pytest -q -p no:cacheprovider
```

Result: **272 passed**, two existing trimesh `RuntimeWarning` messages, and no
Python source write. The output is preserved in
[python-reference-tests.txt](python-reference-tests.txt).

## Native gates

| Replay | Records | Themes | Conditions | Result |
|---|---:|---|---|---|
| Python reference | 6 | 3 Light / 3 Dark | visible reference window; actual OS Cube input; 1280 x 840, 96 DPI | pass |
| Rust application | 72 | 36 Light / 36 Dark | visible native app; actual OS pointer/keyboard/Shift-drag input; one executable SHA | pass |
| Accepted PNG integrity | 78 | both | six Python plus 72 Rust; per-record SHA and 1280 x 840 / 96 DPI guards | pass |

Rust's complete journal is [rust-native-captures.ndjson](rust-native-captures.ndjson);
its structured result list is [rust-native-results.json](rust-native-results.json).
The final result set contains no error journal and no running app process. Both
final Subtract frames show `Subtract: 10496 face(s)`.

## Rejection and preservation boundary

Rejected intermediate native runs, the first missing-libclang bloat attempt, and
the corrected test assertion are recorded in [rejected-attempts.md](rejected-attempts.md).
Python source preservation and the no-stage/no-commit rule are recorded in
[source-preservation.md](source-preservation.md). Visual review, owner
acceptance, alternate DPI, and cross-host coverage remain outside this claim.

## Snapshot

The final verified additive snapshot and exact file comparison are recorded in
[snapshot-receipt.md](snapshot-receipt.md) and [checkpoint.md](checkpoint.md).
