# Rejected Native Attempts

- **Rust initial presentation, 2026-11-02.** The release process was verified as
  PID 27176 (`builder3d-app.exe`) with window `Builder3D - Untitled`, but the
  foreground process was Windows `LockApp` PID 9808 (`Windows Default Lock
  Screen`). The runner refused its first foreground capture. A later PID- and
  hit-test-verified activation click was intercepted by the lock screen; no
  in-app action was accepted. The diagnostic window capture is retained at
  `native/rust-live-final-20261102-101724/captures/rust-dark-welcome-launch-diagnostic.png`
  (SHA-256 `8f8f9e4d35df6f80b2319e2d12fa29daa012cc1fb8f6f16a130e60fd3f027d08`;
  1296×879 outer window). It is black and excluded from accepted evidence.
  Its background UI Automation tree contained 38 descendants and 27 named
  buttons, but no focused element; this is an accessibility diagnostic, not an
  accepted focus-order or visible-focus check.
- **Rust screenshot path typo.** One diagnostic call supplied a path relative to
  the project root instead of the run directory. GDI+ created no file; the
  corrected diagnostic path is the retained black frame above.
- **Initial Python QSettings harness.** On Windows,
  `QSettings(organization, application)` continued using the registry despite
  the attempted default-format/path configuration. Those initial frames are
  retained for diagnosis but excluded from accepted comparison metadata. The
  accepted reference run uses the corrected adapter-level `IniFormat` harness
  at `native/python-live-baseline-20261102-094708/`.
- **Python run `094527`.** Its capture metadata requested Dark, but its main
  window used the registry's Light theme. It is excluded. The accidentally
  changed `theme_mode` registry value was restored to `dark`; the three
  registry values and empty recent list were then checked before Rust migration.

The native Rust matrix remains pending until the desktop is unlocked. No
application behavior, registry value, source file, or Git index was changed by
the rejected Rust input attempt. `native/rb08-ui-driver.ps1` now detects
`LockApp`/`LogonUI` and fails closed before sending input or accepting a capture;
its guard was verified against the current lock-screen foreground.
