# RB-02 Verification

## Engineering checks

Final engineering checks:

- `cargo fmt --all -- --check` — passed.
- `cargo test --workspace --locked` — passed, 203 tests total: app 67, core 81 (including integration tests), MCP 14, platform 15, viewport 26. No failures.
- `cargo check --workspace --locked` — passed.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` — passed.
- `cargo deny check` — passed; advisories, bans, licenses, and sources are OK. Cargo Deny emitted existing duplicate-version warnings for transitive dependencies.
- Python 3.13 `tomllib` parsed the parity ledger, capture metadata, and fixture manifest — passed.
- `cargo audit` — not run; `cargo-audit.exe` is not installed.
- `cargo bloat --release --crates -p modeler3d-app --locked` — passed. The release executable is 14,135,808 bytes (13.48 MiB); the `.text` section is 11.0 MiB. `cargo bloat` reports per-crate `.text` estimates and notes that the estimates are approximate.

## Native run

- Executable: `E:\3D-Modeler\target\debug\modeler3d-app.exe`
- Executable SHA-256: `4b197161771239c8943fb8fc2f73d6739245145769c301c32931b01e5480da87`
- Runtime adapter: see [runtime-adapter.txt](runtime-adapter.txt); Quadro RTX 4000, Vulkan, NVIDIA 581.42.
- App window: restored and visible; measured client area 1280 x 840 logical and physical pixels, 96 DPI / 100% scale.
- Primary display: 2560 x 1440 physical pixels; 2560 x 1392 work area.
- Capture method: GDI `CopyFromScreen` of the foreground app client rectangle. The capture helper first checked the foreground title and refused non-Modeler3D foreground windows.
- Capture set: 25 images in [captures/](captures/), every image decoded and confirmed to be 1280 x 840. Per-image process, timestamp, byte length, and SHA-256 are in [capture-metadata.toml](capture-metadata.toml).
- Native interactions used the task-local Windows input helper to send pointer and keyboard events to the visible app window; the resulting state was read from the app UI. The helper is under ignored build output and is not application behavior.
- App processes were closed between Light and Dark walkthroughs. The Python source at `L:\Builder3D` was read-only and was not launched for this Rust-only RB-02 workflow evidence.
- Native steps and observed state: [walkthrough.md](walkthrough.md).

## Scope and remaining gates

Native evidence covers New, Cube, STL import with mm, native picker cancellation (Light), shaded rendering, camera orbit/pan/zoom, exact geometry selection, translation-gizmo movement, undo/redo, save, and reopening in both themes. It does not establish all model formats, all DPI scales, or the complete GPU/platform resource matrix.

The 43,340,965-byte source-art payload and 14,135,808-byte release executable total 57,476,773 bytes (54.79 MiB) before installer/runtime sidecars, exceeding the 41,943,040-byte target. This separate size issue remains open and was not changed by RB-02. The owner accepted the current visual baseline; fine-tuning may happen later. RB-03 remains unstarted.
