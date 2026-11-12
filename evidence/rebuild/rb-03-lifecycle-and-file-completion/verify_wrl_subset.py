"""Independently read the VRML 2.0 subset emitted by the RB-03 writer.

This standard-library reader checks the exporter output as a VRML document:
DEF/Transform/Shape/Material/IndexedFaceSet structure, local vertices and
indices, material RGBA, transformed world bounds, and object names. It is an
independent subset reader, not a general-purpose VRML implementation.
"""

from __future__ import annotations

import json
import math
import re
import sys
from pathlib import Path


NUMBER = re.compile(r"[-+]?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][-+]?\d+)?")
TRANSFORM = re.compile(r"(?m)^DEF\s+([A-Za-z_][A-Za-z0-9_]*)\s+Transform\s*\{")


def extract_transform_blocks(text: str) -> list[tuple[str, str, str]]:
    blocks: list[tuple[str, str, str]] = []
    for match in TRANSFORM.finditer(text):
        open_brace = text.find("{", match.start(), match.end())
        depth = 0
        in_string = False
        escaped = False
        in_comment = False
        close_brace = None
        index = open_brace
        while index < len(text):
            character = text[index]
            if in_comment:
                if character == "\n":
                    in_comment = False
            elif in_string:
                if escaped:
                    escaped = False
                elif character == "\\":
                    escaped = True
                elif character == '"':
                    in_string = False
            elif character == "#":
                in_comment = True
            elif character == '"':
                in_string = True
            elif character == "{":
                depth += 1
            elif character == "}":
                depth -= 1
                if depth == 0:
                    close_brace = index + 1
                    break
            index += 1
        assert close_brace is not None, f"unterminated Transform DEF {match.group(1)}"

        preceding_lines = text[: match.start()].rstrip().splitlines()
        assert preceding_lines, "Transform is missing its object-name comment"
        name_comment = preceding_lines[-1]
        name_match = re.fullmatch(r'# Object name: ("(?:[^"\\]|\\.)*")', name_comment)
        assert name_match is not None, f"missing VRML object name before {match.group(1)}"
        name = json.loads(name_match.group(1))
        blocks.append((match.group(1), name, text[open_brace:close_brace]))
    return blocks


def field_values(block: str, field: str, count: int) -> list[float]:
    match = re.search(rf"(?m)^\s*{re.escape(field)}\s+([^\r\n]+)$", block)
    assert match is not None, f"missing {field} field"
    values = [float(value) for value in NUMBER.findall(match.group(1))]
    assert len(values) == count, f"{field} expected {count} values, found {values}"
    return values


def array_values(block: str, field: str) -> list[float]:
    match = re.search(rf"\b{re.escape(field)}\s*\[(.*?)\]", block, re.DOTALL)
    assert match is not None, f"missing {field} array"
    return [float(value) for value in NUMBER.findall(match.group(1))]


def transformed_point(
    point: tuple[float, float, float],
    translation: list[float],
    rotation: list[float],
    scale: list[float],
) -> tuple[float, float, float]:
    scaled = tuple(point[axis] * scale[axis] for axis in range(3))
    x_axis, y_axis, z_axis, angle = rotation
    axis_length = math.sqrt(x_axis * x_axis + y_axis * y_axis + z_axis * z_axis)
    if abs(angle) <= 1e-12:
        rotated = scaled
    else:
        assert axis_length > 0, "non-zero rotation requires an axis"
        axis = (x_axis / axis_length, y_axis / axis_length, z_axis / axis_length)
        cosine = math.cos(angle)
        sine = math.sin(angle)
        dot = sum(axis[index] * scaled[index] for index in range(3))
        cross = (
            axis[1] * scaled[2] - axis[2] * scaled[1],
            axis[2] * scaled[0] - axis[0] * scaled[2],
            axis[0] * scaled[1] - axis[1] * scaled[0],
        )
        rotated = tuple(
            scaled[index] * cosine
            + cross[index] * sine
            + axis[index] * dot * (1.0 - cosine)
            for index in range(3)
        )
    return tuple(rotated[index] + translation[index] for index in range(3))


def bounds(points: list[tuple[float, float, float]]) -> tuple[list[float], list[float]]:
    assert points, "VRML object has no vertices"
    minimum = [min(point[axis] for point in points) for axis in range(3)]
    maximum = [max(point[axis] for point in points) for axis in range(3)]
    return minimum, maximum


def close_values(actual: list[float] | tuple[float, ...], expected: list[float]) -> bool:
    return len(actual) == len(expected) and all(
        abs(left - right) <= 1e-6 for left, right in zip(actual, expected)
    )


def verify(path: Path) -> None:
    text = path.read_text(encoding="utf-8")
    assert text.startswith("#VRML V2.0 utf8\n"), "missing VRML 2.0 header"
    blocks = extract_transform_blocks(text)
    assert len(blocks) == 2, f"expected two scene instances, found {len(blocks)}"

    expected = [
        {
            "name": "Blue block",
            "translation": [10.0, 20.0, 30.0],
            "extents": [2.0, 4.0, 6.0],
            "rgb": [0.2, 0.4, 0.6],
            "transparency": 0.2,
        },
        {
            "name": "Red block",
            "translation": [-4.0, 5.0, 1.0],
            "extents": [1.0, 3.0, 5.0],
            "rgb": [0.9, 0.2, 0.1],
            "transparency": 0.0,
        },
    ]
    world_points: list[tuple[float, float, float]] = []

    for (def_name, name, block), contract in zip(blocks, expected):
        assert name == contract["name"], f"unexpected object name {name!r}"
        assert def_name == name.replace(" ", "_"), f"unexpected DEF name {def_name!r}"
        translation = field_values(block, "translation", 3)
        rotation = field_values(block, "rotation", 4)
        scale = field_values(block, "scale", 3)
        rgb = field_values(block, "diffuseColor", 3)
        transparency = field_values(block, "transparency", 1)[0]
        assert close_values(translation, contract["translation"]), translation
        assert close_values(rotation, [0.0, 0.0, 1.0, 0.0]), rotation
        assert close_values(scale, [1.0, 1.0, 1.0]), scale
        assert close_values(rgb, contract["rgb"]), rgb
        assert abs(transparency - contract["transparency"]) <= 1e-6

        local_values = array_values(block, "point")
        assert len(local_values) == 8 * 3, f"{name} expected eight vertices"
        local_points = [tuple(local_values[index : index + 3]) for index in range(0, len(local_values), 3)]
        local_bounds = bounds(local_points)
        half_extents = [value / 2.0 for value in contract["extents"]]
        assert close_values(local_bounds[0], [-value for value in half_extents])
        assert close_values(local_bounds[1], half_extents)

        face_values = [int(value) for value in array_values(block, "coordIndex")]
        faces: list[list[int]] = []
        face: list[int] = []
        for index in face_values:
            if index == -1:
                assert len(face) == 3, f"{name} contains a non-triangle face"
                faces.append(face)
                face = []
            else:
                assert 0 <= index < len(local_points), f"{name} index {index} is out of range"
                face.append(index)
        assert not face and len(faces) == 12, f"{name} expected twelve indexed triangles"

        object_world = [
            transformed_point(point, translation, rotation, scale)
            for point in local_points
        ]
        object_bounds = bounds(object_world)
        expected_world_bounds = (
            [contract["translation"][axis] - half_extents[axis] for axis in range(3)],
            [contract["translation"][axis] + half_extents[axis] for axis in range(3)],
        )
        assert close_values(object_bounds[0], expected_world_bounds[0])
        assert close_values(object_bounds[1], expected_world_bounds[1])
        world_points.extend(object_world)

    overall_bounds = bounds(world_points)
    assert close_values(overall_bounds[0], [-4.5, 3.5, -1.5])
    assert close_values(overall_bounds[1], [11.0, 22.0, 33.0])


def main() -> int:
    if len(sys.argv) != 2:
        print(f"usage: {Path(sys.argv[0]).name} <exported-scene.wrl>", file=sys.stderr)
        return 2
    path = Path(sys.argv[1])
    verify(path)
    print(
        "WRL independent subset-reader pass: 2 named objects, 24 triangles, "
        "transforms, RGBA material values, and world bounds verified."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
