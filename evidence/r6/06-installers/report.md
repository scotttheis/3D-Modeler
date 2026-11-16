# R6-06 installers

## Delivered behavior

- Added native Windows PowerShell, Linux shell, and macOS command installers
  plus guarded uninstall scripts. Default destinations are private user roots:
  `%LOCALAPPDATA%\\Modeler3D`, `$HOME/.local/opt/modeler3d`, and
  `$HOME/Applications/Modeler3D`.
- Windows scripts pass PowerShell parsing. POSIX scripts use quoted paths,
  reject broad system roots, and uninstaller paths must match their private
  default before removal. No installer or launcher invokes Python.
- Added Rust package staging with bounded recursive file enumeration, regular
  file/symlink checks, per-file SHA-256 entries, a `PACKAGE-MANIFEST.json`,
  deterministic ZIP member order, and explicit 40 MiB installed / 15 MiB
  compressed gates.
- The size-profile native package was staged and verified at:
  `target\\r6-package-r6-06` and `target\\r6-package-r6-06.zip`.

## Measured package

| Result | Measurement | Gate |
| --- | ---: | ---: |
| Installed files | 13 | — |
| Installed bytes | 9,425,015 (8.988 MiB) | ≤ 41,943,040 bytes — passed |
| Compressed ZIP | 4,190,185 (3.996 MiB) | ≤ 15,728,640 bytes — passed |
| Manifest entries | 12 + manifest | all SHA-256 checks passed |
| ZIP entries | 13 | manifest/package paths matched |

Archive SHA-256:
`9f441174880d8041f8ee03143d9474e51b7ee9cb936d0f1d70479a5e5f3de8ce`.

## Verification

- `cargo fmt --all -- --check` — passed.
- `cargo test -p modeler3d-app` — 17 passed.
- `cargo clippy -p modeler3d-app --all-targets --all-features -- -D warnings` —
  passed.
- Native `--package` staging — passed with both size gates.
- PowerShell installer parsing — 2 files, 0 parse errors.
- POSIX shell syntax check was not available on this Windows host; the scripts
  were statically inspected and the package paths were manifest/hash verified.
- Python slicer/icon reference checks — 113 passed. The known separate setup
  contract selector still has one stale `K:\\builder3D.png` assertion in the
  pre-existing dirty Python docs; no Python file was changed.

R6 implementation is complete. Final workspace/MCP/security verification and
the final additive snapshot are still required.
