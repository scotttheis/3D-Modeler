# R6-04 theme-scoped artwork

## Delivered behavior

- Added 20 compact transparent SVG icon assets for each of the light and dark
  themes, including the documented `ExRay.svg` and `Subtract.svg` artwork.
- Added a compile-time `ThemeArtwork` catalog keyed by semantic `IconKey`
  values. The catalog validates bounded size, self-contained SVG structure,
  and rejects external/raster dependencies before the application starts.
- Ribbon buttons now paint vector icons through the theme catalog; the SVG's
  theme-scoped declared tint is used for the visible icon, while the shell
  palette supplies the surrounding state colors.
- The Rust tree contains 40 artwork files totaling 4,650 bytes (2,335 light,
  2,315 dark), keeping the installed-size gate practical instead of copying
  the oversized Python raster directories.

## Verification

- `cargo fmt --all -- --check` — passed.
- `cargo test -p modeler3d-app` — 13 passed, including both-theme catalog
  validation and scoped-artwork tests.
- `cargo run -q -p modeler3d-app --bin modeler3d-app -- --smoke` — passed.
- Python reference icon/theme/ribbon selectors — 119 passed, 32 deselected.

The next ordered task is R6-05 launchers.
