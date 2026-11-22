# Open File on the home page

- Goal: put an Open File tile between New Scene and Open Project.
- Owner request: 2026-11-22; use the generic insert/import flow.
- Files: `crates/modeler3d-app/src/shell.rs`, `docs/STATUS.md`.
- Reuse `CommandId::ImportModel`, its native picker, artwork, supported formats,
  import units, and existing scene transition; keep the three tiles in order
  when the layout wraps on narrow windows.
- Verify: scripted Fast gate for `modeler3d-app`; before/after home captures
  in both themes, normal and big UI; rebuild the current Release app with
  `scripts/Update-CurrentBuild.ps1` and capture its home page.
- Evidence: `evidence/rebuild/captures/2026-11-22-home-open-file/`.
- Recovery baseline: the verified completed Emboss snapshot
  `G:\3D-Modeler_snapshot_20261122-063535_hide-emboss`.
- Follow `E:\RustDeveloper.md` and the project `AGENTS.md` for future work.
