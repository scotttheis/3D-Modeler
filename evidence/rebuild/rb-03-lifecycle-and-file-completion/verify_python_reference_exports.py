"""Reopen RB-03 exports with the read-only Python Builder3D reference."""

from __future__ import annotations

import sys
from pathlib import Path

import numpy as np


EXPECTED_BOUNDS = ([-4.5, 3.5, -1.5], [11.0, 22.0, 33.0])
EXPECTED_ITEMS = [
    {
        "name": "Blue block",
        "extents": [2.0, 4.0, 6.0],
        "position": [10.0, 20.0, 30.0],
        "color": [0.2, 0.4, 0.6, 0.8],
    },
    {
        "name": "Red block",
        "extents": [1.0, 3.0, 5.0],
        "position": [-4.0, 5.0, 1.0],
        "color": [0.9, 0.2, 0.1, 1.0],
    },
]


def assert_close(actual, expected, label: str, tolerance: float = 1e-4) -> None:
    assert np.allclose(actual, expected, atol=tolerance, rtol=0.0), (
        f"{label}: expected {expected}, got {actual}"
    )


def mesh_bounds(mesh) -> tuple[np.ndarray, np.ndarray]:
    bounds = np.asarray(mesh.bounds, dtype=np.float64)
    assert bounds.shape == (2, 3), f"unexpected bounds shape {bounds.shape}"
    return bounds[0], bounds[1]


def verify_mesh_export(load_mesh, path: Path, extension: str) -> None:
    mesh = load_mesh(path)
    assert len(mesh.vertices) > 0 and len(mesh.faces) > 0
    actual_bounds = mesh_bounds(mesh)
    assert_close(actual_bounds[0], EXPECTED_BOUNDS[0], f"{extension} lower bounds")
    assert_close(actual_bounds[1], EXPECTED_BOUNDS[1], f"{extension} upper bounds")

    if extension in {"obj", "ply"}:
        vertex_colors = np.asarray(mesh.visual.vertex_colors)
        assert vertex_colors.shape == (len(mesh.vertices), 4), (
            f"{extension} did not expose per-vertex RGBA colors: {vertex_colors.shape}"
        )
        observed = {tuple(int(channel) for channel in color) for color in vertex_colors}
        expected = {
            (51, 102, 153, 255 if extension == "obj" else 204),
            (230, 51, 26, 255),
        }
        assert observed == expected, f"{extension} colors: expected {expected}, got {observed}"


def verify_scene_export(load_import_items, path: Path, extension: str) -> None:
    items = load_import_items(path, unit="mm")
    assert len(items) == 2, f"{extension} expected two scene items, got {len(items)}"
    items_by_name = {item.name: item for item in items}
    assert set(items_by_name) == {expected["name"] for expected in EXPECTED_ITEMS}, (
        f"{extension} names: {sorted(items_by_name)}"
    )
    all_world_vertices = []

    for expected in EXPECTED_ITEMS:
        item = items_by_name[expected["name"]]
        assert_close(item.mesh.extents, expected["extents"], f"{extension} {item.name} extents")
        transform = np.asarray(item.transform, dtype=np.float64)
        assert_close(transform[:3, 3], expected["position"], f"{extension} {item.name} position")
        assert item.color is not None, f"{extension} {item.name} lost its material color"
        assert_close(item.color, expected["color"], f"{extension} {item.name} color", 0.01)

        world_mesh = item.mesh.copy()
        world_mesh.apply_transform(transform)
        all_world_vertices.append(np.asarray(world_mesh.vertices, dtype=np.float64))

    joined = np.concatenate(all_world_vertices, axis=0)
    assert_close(joined.min(axis=0), EXPECTED_BOUNDS[0], f"{extension} world lower bounds")
    assert_close(joined.max(axis=0), EXPECTED_BOUNDS[1], f"{extension} world upper bounds")


def main() -> int:
    if len(sys.argv) != 3:
        print(
            f"usage: {Path(sys.argv[0]).name} <python-reference-root> <export-directory>",
            file=sys.stderr,
        )
        return 2

    reference_root = Path(sys.argv[1]).resolve()
    export_directory = Path(sys.argv[2]).resolve()
    sys.path.insert(0, str(reference_root))
    from src.mesh_ops import load_import_items, load_mesh

    for extension in ("stl", "obj", "ply"):
        verify_mesh_export(load_mesh, export_directory / f"scene.{extension}", extension)
    for extension in ("3mf", "gltf", "glb"):
        verify_scene_export(
            load_import_items,
            export_directory / f"scene.{extension}",
            extension,
        )

    print(
        "Python reference reader pass: STL/OBJ/PLY world geometry; OBJ/PLY colors; "
        "3MF/glTF/GLB names, local geometry, transforms, RGBA, and world bounds verified."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
