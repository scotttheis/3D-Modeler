# RB-07 verification

## Tests

| Target | Result |
|---|---:|
| Rust app focused | 150 passed, 0 failed |
| Rust core focused | 88 passed, 0 failed |
| Rust platform focused | 15 passed, 0 failed |
| Rust viewport focused | 34 passed, 0 failed |
| Rust workspace (cargo test --workspace --locked) | 301 passed, 0 failed, 0 ignored |
| Python focused reference suite | 103 passed |
| Python full reference suite | 785 passed, 1 failed; 2 existing trimesh warnings |

The only Python full-suite failure is tests/test_setup_contract.py::test_private_use_guide_covers_launcher_icon_and_recovery, which expects K:\builder3D.png in the pre-existing L:\Builder3D\docs\private-use.md. The reference tree is read-only; this known machine-path assertion was preserved. The two warnings are RuntimeWarning: invalid value encountered in divide from trimesh in existing repair/boolean tests.

Focused logs: rust-focused-app-tests-final.txt, rust-focused-core-tests.txt, rust-focused-platform-tests.txt, rust-focused-viewport-tests-lock-update.txt, python-focused-tests.txt. Full logs: quality-gates/cargo-test-workspace.txt and python-full-tests.txt.

## Documented quality gates

All 10 rows in quality-gates/results.tsv are PASS:

1. cargo fmt --all -- --check
2. cargo check --workspace --locked
3. cargo test --workspace --locked
4. cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
5. cargo deny check
6. cargo metadata --locked --format-version 1 --no-deps
7. native app smoke check
8. release app build
9. cargo bloat --release --workspace
10. cargo bloat --release -p modeler3d-app --bin modeler3d-app

cargo audit was not installed. No optional tool was installed. The initial focused viewport run with --locked requested a lockfile update and was rerun successfully after the existing serde_json dev dependency was recorded in Cargo.lock; the focused viewport result is 34 passed. The workspace bloat command's log records that the installed cargo-bloat invocation ignored --workspace; the app and workspace binaries were measured separately, and the app-specific gate passed.

## Native evidence

- Accepted set: 117 PNGs, verified against per-file hashes in capture-metadata.json and accepted-journal.ndjson.
- Python: 34 images, 17 Light / 17 Dark. There are 32 result-JSON records plus two manually captured clean Print Prep reports. Heavy Smooth progress/cancel/cancelled/retry is represented in both themes.
- Rust 3D Modeler: 82 images, 40 Light / 42 Dark, final debug build SHA-256 0BF7753FE460A6A854906457F86363DDC905B455FA790760B6D9122F6F9CB0D6.
- External slicer: one Bambu Studio image after the actual Rust launch of the staged STL.
- 3D Modeler windows: 1280x840 logical and physical at 96 DPI / 100% scale. The Bambu capture is 2553x1385.
- Native Python sequence preceded the Rust sequence. All attempts that were mislabeled, stale-build, wrong-theme, or too late to prove cancellation are outside captures/ and listed under rejected/.

## Executables

| Build | Size | SHA-256 |
|---|---:|---|
| target/debug/modeler3d-app.exe | 61,850,624 bytes | 0BF7753FE460A6A854906457F86363DDC905B455FA790760B6D9122F6F9CB0D6 |
| target/release/modeler3d-app.exe | 15,261,184 bytes | E26C37DE652B6434727AE7F6396012CDF4AA911299857B621D336AFB4DA38448 |

## Reference tree and processes

The ten behavior-critical Python source/test hashes listed in source-preservation.md still match their pre-test values. L:\Builder3D has the same 116 pre-existing Git status records. No reference files were written. Before final snapshot verification, 3D Modeler Rust, 3D Modeler Python UI, and Bambu Studio processes were closed. No stage/commit was made.

