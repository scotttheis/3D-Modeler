"""Independently reopen the RB-03 native export walkthrough outputs."""

from __future__ import annotations

import sys
from pathlib import Path

import numpy as np


EXPECTED_BOUNDS = ([-6.0, 0.0, -15.0], [6.0, 20.0, 15.0])
EXPECTED_ITEMS = [
    {
        "name": "First part",
        "extents": [10.0, 20.0, 30.0],
        "position": [1.0, 10.0, 0.0],
        "color": [0.2, 0.4, 0.8, 128.0 / 255.0],
    },
    {
        "name": "Second part",
        "extents": [2.0, 3.0, 4.0],
        "position": [-5.0, 6.0, -3.0],
        "color": [1.0, 0.0, 0.0, 1.0],
    },
]


def assert_close(actual, expected, label: str, tolerance: float = 1e-4) -> None:
    assert np.allclose(actual, expected, atol=tolerance, rtol=0.0), (
        f"{label}: expected {expected}, got {actual}"
    )


def mesh_bounds(mesh):
    actual = np.asarray(mesh.bounds, dtype=np.float64)
    assert actual.shape == (2, 3), f"unexpected bounds shape {actual.shape}"
    return actual[0], actual[1]


def verify_merged_mesh(
    load_mesh,
    path: Path,
    extension: str,
    expected_bounds=EXPECTED_BOUNDS,
    expected_colors=None,
) -> None:
    mesh = load_mesh(path)
    assert len(mesh.vertices) > 0 and len(mesh.faces) > 0
    actual = mesh_bounds(mesh)
    assert_close(actual[0], expected_bounds[0], f"{extension} lower bounds")
    assert_close(actual[1], expected_bounds[1], f"{extension} upper bounds")

    if extension in {"obj", "ply"}:
        colors = np.asarray(mesh.visual.vertex_colors)
        assert colors.shape == (len(mesh.vertices), 4), (
            f"{extension} did not expose one RGBA color per vertex: {colors.shape}"
        )
        expected = expected_colors or {
            (51, 102, 204, 255 if extension == "obj" else 128),
            (255, 0, 0, 255),
        }
        observed = {tuple(int(value) for value in color) for color in colors}
        assert observed == expected, f"{extension} colors: expected {expected}, got {observed}"


def verify_scene_file(
    load_import_items,
    path: Path,
    extension: str,
    expected_items=EXPECTED_ITEMS,
    expected_bounds=EXPECTED_BOUNDS,
) -> None:
    items = load_import_items(path, unit="mm")
    assert len(items) == len(expected_items), (
        f"{extension} expected {len(expected_items)} objects, got {len(items)}"
    )
    by_name = {item.name: item for item in items}
    assert set(by_name) == {item["name"] for item in expected_items}, (
        f"{extension} names: {sorted(by_name)}"
    )
    world_vertices = []
    for expected in expected_items:
        item = by_name[expected["name"]]
        assert_close(item.mesh.extents, expected["extents"], f"{extension} extents")
        transform = np.asarray(item.transform, dtype=np.float64)
        assert_close(transform[:3, 3], expected["position"], f"{extension} position")
        assert item.color is not None, f"{extension} lost its material color"
        assert_close(item.color, expected["color"], f"{extension} color", 0.01)
        world_mesh = item.mesh.copy()
        world_mesh.apply_transform(transform)
        world_vertices.append(np.asarray(world_mesh.vertices, dtype=np.float64))
    joined = np.concatenate(world_vertices, axis=0)
    assert_close(joined.min(axis=0), expected_bounds[0], f"{extension} world minimum")
    assert_close(joined.max(axis=0), expected_bounds[1], f"{extension} world maximum")


def verify_wrl(path: Path) -> None:
    # Reuse the independent standard-library VRML 2.0 subset parser while
    # supplying this walkthrough's different scene contract.
    from verify_wrl_subset import (
        array_values,
        bounds,
        close_values,
        extract_transform_blocks,
        field_values,
        transformed_point,
    )

    text = path.read_text(encoding="utf-8")
    assert text.startswith("#VRML V2.0 utf8\n"), "missing VRML 2.0 header"
    blocks = extract_transform_blocks(text)
    assert len(blocks) == 2, f"WRL expected two scene instances, found {len(blocks)}"
    world_points = []
    for (def_name, name, block), expected in zip(blocks, EXPECTED_ITEMS):
        assert name == expected["name"], f"WRL name: expected {expected['name']}, got {name}"
        assert def_name == name.replace(" ", "_"), f"WRL DEF name mismatch: {def_name}"
        translation = field_values(block, "translation", 3)
        rotation = field_values(block, "rotation", 4)
        scale = field_values(block, "scale", 3)
        color = field_values(block, "diffuseColor", 3)
        transparency = field_values(block, "transparency", 1)[0]
        assert close_values(translation, expected["position"])
        assert close_values(rotation, [0.0, 0.0, 1.0, 0.0])
        assert close_values(scale, [1.0, 1.0, 1.0])
        assert close_values(color, expected["color"][:3])
        assert abs(transparency - (1.0 - expected["color"][3])) <= 0.01
        values = array_values(block, "point")
        assert len(values) == 24, f"{name} expected eight local vertices"
        local = [tuple(values[i : i + 3]) for i in range(0, len(values), 3)]
        local_bounds = bounds(local)
        half = [value / 2.0 for value in expected["extents"]]
        assert close_values(local_bounds[0], [-value for value in half])
        assert close_values(local_bounds[1], half)
        indices = [int(value) for value in array_values(block, "coordIndex")]
        assert indices.count(-1) == 12, f"{name} expected twelve indexed triangles"
        world_points.extend(
            transformed_point(point, translation, rotation, scale) for point in local
        )
    actual = bounds(world_points)
    assert close_values(actual[0], EXPECTED_BOUNDS[0])
    assert close_values(actual[1], EXPECTED_BOUNDS[1])


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
        filename = "all-visible.stl" if extension == "stl" else f"no-selection.{extension}"
        verify_merged_mesh(load_mesh, export_directory / filename, extension)
    for extension in ("3mf", "gltf", "glb"):
        verify_scene_file(
            load_import_items,
            export_directory / f"no-selection.{extension}",
            extension,
        )
    verify_wrl(export_directory / "no-selection.wrl")
    verify_merged_mesh(
        load_mesh,
        export_directory / "one-selected.obj",
        "obj",
        expected_bounds=([-4.0, 0.0, -15.0], [6.0, 20.0, 15.0]),
        expected_colors={(51, 102, 204, 255)},
    )
    verify_merged_mesh(load_mesh, export_directory / "multi-selected.ply", "ply")
    verify_scene_file(
        load_import_items,
        export_directory / "mixed-visible.3mf",
        "3mf",
        expected_items=[EXPECTED_ITEMS[1]],
        expected_bounds=([-6.0, 4.5, -5.0], [-4.0, 7.5, -1.0]),
    )
    for filename in ("hidden-selected.obj", "no-visible.stl"):
        assert not (export_directory / filename).exists(), (
            f"failed native selection export unexpectedly created {filename}"
        )
    print(
        "Native export readers pass: seven formats plus one/multiple-selected and "
        "mixed-visible outputs; hidden-selected/no-visible failures created no files; "
        "transformed bounds, object names, dimensions, colors, and WRL indexed geometry."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
