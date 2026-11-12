# RB-03 format and unit results

Rust automated coverage, native walkthroughs, and independent output readers are complete for the supported file paths. The Python-reference UI was exercised during import, lifecycle, recent, and shared export-dialog flows; the old export UI was used for 3MF, not replayed independently for all seven output formats.

## Unitless import units

| Choice | Scale to millimetres | Automated coverage | Visible native result |
|---|---:|---|---|
| `um` | 0.001 | Factor, reference order, and preference aliases covered. | Selector appears in the Rust six-unit chooser; Python has the corresponding unit setting. |
| `mm` | 1 | Default and factor covered. | Unitless box displays 2 × 3 × 4 mm in both apps. |
| `cm` | 10 | Factor covered; declared-centimetre 3MF remains authoritative. | Rust's import flow displays the 3MF parts; declared-unit behavior is covered by fixtures/tests. |
| `in` | 25.4 | Factor covered. | Unitless box displays 50.8 × 76.2 × 101.6 mm in both apps after inch import. |
| `ft` | 304.8 | Factor covered. | Selector/scale covered by tests. |
| `m` | 1000 | Factor covered; glTF/GLB use declared metres. | Selector and authoritative-unit flow covered. |

STL/OBJ/PLY apply the explicit or saved ambiguous-import unit exactly once. 3MF uses its declared unit; glTF/GLB use metres without applying the ambiguous-unit override. The old Python UI was switched from default mm to inches for the unitless import capture, then imported authoritative-unit formats to show their own unit behavior.

## Import and export formats

| Format | Import result | Export result | Independent verification | Python-reference UI |
|---|---|---|---|---|
| STL | Unitless scale; geometry and Y=0 placement covered. | World-space geometry; STL color loss preserved. | Rust reopen and geometry/bounds checks. | Imported in mm/in; old Export Mesh picker captured with 3MF selected. |
| OBJ | Names, geometry, transforms, and vertex colors covered. | World-space geometry with per-vertex RGB; alpha is not represented. | Rust reopen and Python reference reader. | Added to the saved project with PLY; shared old export picker captured. |
| PLY | Names, geometry, transforms, and vertex colors covered. | World-space geometry with per-vertex RGBA. | Rust reopen and Python reference reader. | Added to the saved project with OBJ; shared old export picker captured. |
| 3MF | Declared centimetres; two names, colors, transforms, placement, and framing covered. | Separate named objects, transforms, colors, and supported metadata. | Rust reopen and Python reference reader. | Two-part import and successful native no-selection 3MF export captured. |
| glTF | Declared metres; two names, colors, transforms, placement, and framing covered. | Separate nodes, metre transforms, and material colors. | Rust reopen and Python reference reader. | Two-part import captured; the shared export picker was not replayed with glTF selected. |
| GLB | Same authoritative metre and multi-object coverage as glTF. | Separate nodes, metre transforms, and material colors. | Rust reopen and Python reference reader. | Two-part import captured; the shared export picker was not replayed with GLB selected. |
| WRL | Import remains unsupported and reports an error while preserving state. | Export only; names, transforms, colors, indexed geometry, and world bounds covered. | Independent standard-library VRML 2.0 subset reader. | Unsupported recent import error captured; old Export Mesh picker was not replayed with WRL selected. |

The independent Rust native export run is recorded in `logs/native-export-verification.log`. `verify_python_reference_exports.py` reads STL/OBJ/PLY/3MF/glTF/GLB output independently; `verify_wrl_subset.py` checks the actual WRL output with a standard-library VRML 2.0 subset reader.

## Export selection and cancellation

| Native scene state | Result |
|---|---|
| No selection | Export all visible objects. |
| One selected | Export selected visible object(s). |
| Multiple selected | Export selected visible object(s). |
| Hidden selected with no visible selected object | Return the reference-style no-exportable-object error; no output file is created. |
| Mixed visible/hidden scene | Export only eligible visible objects. |
| No visible objects | Return an error; no output file is created. |
| Overwrite cancelled | Preserve the existing destination byte-for-byte. The Python capture also recorded unchanged SHA-256 `10fe49926dd3581c55b4215f53b540a3c636f4b171a51ff7bd922da3441a6c63`. |
| Import/export picker cancelled | Preserve scene and state; do not commit a recent entry or output path. |

The Python GUI's actual successful export was `target/rb03-python-no-selection.3mf` (1,262 bytes). The separate Rust outputs live under ignored `target/rb03-native-export-20261031-1519`; the checked-in evidence is the verifier scripts and logs, not generated target files.

## Recent previews

Rust renders bounded mesh previews for STL/OBJ/PLY/3MF/glTF/GLB and visible `.3dp` scenes, loads supported image previews, and uses a file-type icon when the source is missing, corrupt, unsupported, hidden-only, or undecodable. The visible cards were inspected after restart. Python's recent cards use wireframe thumbnails; Rust's previews are small shaded images. The side-by-side images record this style difference without calling it a visual pass.
