from __future__ import annotations

import hashlib
import struct
from pathlib import Path


def generate_closed_cube(
    subdivisions: int,
    output_name: str = "slow-job-closed-cube.ply",
) -> Path:
    output = Path(__file__).parent / "fixtures" / output_name
    output.parent.mkdir(parents=True, exist_ok=True)

    vertices: list[tuple[int, int, int]] = []
    vertex_indices: dict[tuple[int, int, int], int] = {}
    faces: list[tuple[int, int, int]] = []

    def vertex_for(point: tuple[int, int, int]) -> int:
        existing = vertex_indices.get(point)
        if existing is not None:
            return existing
        index = len(vertices)
        vertices.append(point)
        vertex_indices[point] = index
        return index

    # The ordered in-plane axes give each cube side an outward winding.
    sides = (
        (0, subdivisions, 1, 2),
        (0, 0, 2, 1),
        (1, subdivisions, 2, 0),
        (1, 0, 0, 2),
        (2, subdivisions, 0, 1),
        (2, 0, 1, 0),
    )
    for fixed_axis, fixed_value, u_axis, v_axis in sides:
        for v in range(subdivisions):
            for u in range(subdivisions):
                def point_at(u_value: int, v_value: int) -> tuple[int, int, int]:
                    point = [0, 0, 0]
                    point[fixed_axis] = fixed_value
                    point[u_axis] = u_value
                    point[v_axis] = v_value
                    return tuple(point)

                first = vertex_for(point_at(u, v))
                second = vertex_for(point_at(u + 1, v))
                third = vertex_for(point_at(u + 1, v + 1))
                fourth = vertex_for(point_at(u, v + 1))
                faces.extend(((first, second, third), (first, third, fourth)))

    header = (
        "ply\n"
        "format binary_little_endian 1.0\n"
        f"element vertex {len(vertices)}\n"
        "property float x\n"
        "property float y\n"
        "property float z\n"
        f"element face {len(faces)}\n"
        "property list uchar uint vertex_indices\n"
        "end_header\n"
    ).encode("ascii")
    with output.open("wb") as stream:
        stream.write(header)
        for point in vertices:
            stream.write(
                struct.pack(
                    "<fff",
                    *(20.0 * value / subdivisions - 10.0 for value in point),
                )
            )
        for face in faces:
            stream.write(struct.pack("<BIII", 3, *face))

    digest = hashlib.sha256(output.read_bytes()).hexdigest()
    print(
        f"path={output}\nvertices={len(vertices)}\nfaces={len(faces)}\n"
        f"bytes={output.stat().st_size}\nsha256={digest}"
    )
    return output


if __name__ == "__main__":
    generate_closed_cube(289)
