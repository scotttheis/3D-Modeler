# R2a file-I/O evidence

## Scope

This slice implements the first ordered R2 formats from the Python reference:
STL, OBJ, and PLY. The Python tree at L:/Builder3D was read and executed only
as the temporary behavioral reference; it was not modified.

The Rust boundary is in builder3d-core. It provides bounded path and byte
loading, binary/ASCII STL import with binary STL export, OBJ import/export with
optional per-vertex RGB(A), and ASCII/binary PLY import with binary
little-endian export. File colors remain in MeshAsset instead of changing the
authoritative Mesh type.

## Python golden artifacts

The controls were generated with the Python venv using make_box((2, 3, 4)):

| Artifact | Bytes | SHA-256 |
|---|---:|---|
| Python binary STL | 684 | 09ecb0fe943709ccdc45156b8ee4947f17f48b6cbc0931782ea6df86d1efc37d |
| Python ASCII STL | 1397 | f55301ca87cc6a654673dedd0e5265ff1437362e2a8d078c11d1f1ca628aa240 |
| Python OBJ | 425 | 5098149bf5a780127c50efa5df5aaf1b4887208c01ffc37df81d69546bf604d6 |
| Python binary PLY | 464 | d5d07488c74fa494582e6d1541ccc72835adf481da9b9159a182902abf56958f |
| Python ASCII PLY | 569 | f2ad05bf600f02bc96b8de719ca9789f7dee92610a504406c6b343c40c896afe |

Rust tests compare the binary STL, OBJ, and binary PLY output byte-for-byte
against the captured Python artifacts. ASCII STL is compared by ordered
triangle coordinates because its import path may rebuild shared vertex
indices. The hand-authored colored OBJ and PLY fixtures compare the reference
display colors, including OBJ alpha defaulting to 1 and PLY alpha 204/255.

## Verification

- cargo fmt --all -- --check
- cargo test --workspace
- cargo clippy --all-targets -- -D warnings
- cargo deny check
- Python reference: pytest tests/test_mesh_ops.py -k SaveLoad -q — 16 passed,
  67 deselected

The Rust box primitive's one mismatched top-face diagonal was corrected so its
STL/OBJ/PLY triangle order matches the Python exporter golden controls.
