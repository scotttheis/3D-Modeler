# RB-08 Paired Native Comparison Board

## Corrected Python reference

The Python reference process was captured at 1280×840 and 96 DPI / 100% in
both themes, then at the available narrow, wide and maximized sizes. It also
covered settings, About, slicer browsing, open/save dialogs, dirty guards,
keyboard actions, restart persistence and recent-preview fallbacks. The
accepted run is
`native/python-live-baseline-20261102-094708/captures/` (**50 PNGs**).

| State | Python reference capture | Rust final capture | Comparison status |
|---|---|---|---|
| Empty welcome, Dark, 1280×840 | `python-dark-welcome-empty.png` | Pending unlock | Python accepted; Rust pending |
| Application menu, Dark | `python-dark-file-menu-final.png` | Pending unlock | Python accepted; Rust pending |
| Settings and About, Dark | `python-dark-settings-final.png`, `python-dark-about-final.png` | Pending unlock | Python accepted; Rust pending |
| Settings and restart, Light | `python-light-settings-final.png`, `python-light-welcome-after-restart.png` | Pending unlock | Python accepted; Rust pending |
| Layout, Light/Dark, 980×640 | `python-light-layout-980x640.png`, `python-dark-layout-980x640.png` | Pending unlock | Python accepted; Rust pending |
| Layout, Light/Dark, 1600×900 | `python-light-layout-1600x900.png`, `python-dark-layout-1600x900.png` | Pending unlock | Python accepted; Rust pending |
| Layout, Light/Dark, maximized | `python-light-layout-maximized.png`, `python-dark-layout-maximized.png` | Pending unlock | Python accepted; Rust pending |
| Slicer setting, recent clear and preview fallbacks | `python-light-settings-slicer-selected.png`, `python-light-welcome-after-clear.png`, `python-light-welcome-recent-fallbacks-scroll.png` | Pending unlock | Python accepted; Rust pending |
| Text entry, keyboard and dirty modal cases | `python-light-save-dialog-text-entry.png`, `python-light-shortcut-copy-paste-selected.png`, `python-light-dirty-open-confirmation.png`, `python-light-dirty-exit-confirmation.png` | Pending unlock | Python accepted; Rust pending |

## Adjudication

The pre-edit Python observations were collected before Rust implementation but
used an ineffective Windows `QSettings` isolation attempt. They remain in their
original run folders and are excluded from accepted metadata. The corrected
reference runner replaces the production adapter’s QSettings constructor with
an explicit INI-format constructor. The initial Rust release launch imported
the corrected registry values into its isolated JSON runtime, but Windows was
locked and no Rust frame is accepted from that run. The black diagnostic image
and the rejected-attempt journal are retained in the adjacent `native/` and
`rejected-attempts.md` records.

Matched Rust captures, exact hashes, sizes, DPI, theme and routes will be added
after the desktop is unlocked. This board does not claim visual pass or owner
acceptance. Available system scale is 96 DPI / 100%; no other display scale is
available for this run.
