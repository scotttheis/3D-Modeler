# S1-5b: dialog title bar colour in dark theme

Capture `evidence/rebuild/captures/2026-11-19-polish-m5/save-dark.png`: the dialog
title bar is a neutral dark grey that clashes with the navy dialog body; light
theme is fine. In the shared dialog frame helper (chrome.rs / dialogs.rs), derive
the title-bar fill from `ThemePalette` (a slightly raised/lighter variant of the
dialog surface, or the existing panel/header colour), never a literal grey, so it
matches in both themes. Also make the primary action (Save/Apply/Export) visibly
primary (accent fill, existing accent colour with the contrast already tested)
and Cancel/Close secondary. Keep all dialog tests satisfiable.
fmt + clippy only (Windows: `. .\scripts\Initialize-BuildClang.ps1`, cmd /c redirect
to a file outside target/). No tests, captures or commit.
