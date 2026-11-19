# S1-5c: fix failing test after the dialog-frame change

`cargo test -p modeler3d-app --lib dialogs::tests::settings_application_about_and_close_share_one_line`
now fails at dialogs.rs ~2490: `about.bottom() > widget("Clear Recent Files List").bottom()`.
The test's intent: the Application / About / Close footer stays pinned BELOW all
Settings content (it is never overlapped by, or scrolled above, the content).
After S1-5 the Settings dialog layout changed (see
`evidence/rebuild/captures/2026-11-19-polish-m5/settings-dark.png`).
Find why (Clear Recent Files List ends below the footer: content not inside the
scroll area, or footer no longer pinned at the dialog bottom) and fix the LAYOUT
so the footer is pinned below the scrolling content, as before S1-5
(`git show HEAD~1:crates/modeler3d-app/src/dialogs.rs` is the old behaviour).
Do not weaken or delete the assertion. Run that single test and
`dialogs::tests` (all) to confirm, plus fmt + clippy (Windows:
`. .\scripts\Initialize-BuildClang.ps1`, `cmd /c "cargo ... > file 2>&1"`, file outside
target/). Do not commit.
