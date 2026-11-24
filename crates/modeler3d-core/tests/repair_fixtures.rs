//! Portable, generated equivalents of genfx.py f2-f10 and genfx_open500.py.
use modeler3d_core::{
    Mesh, REPAIR_REFUSED_MESSAGE, RepairResult, RepairVerdict, inspect_mesh, repair_mesh,
    repair_summary,
};

struct Random(u64);
impl Random {
    fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1);
        (self.0 >> 11) as f64 / ((1_u64 << 53) as f64)
    }
}

fn cylinder(n: u32, top: bool) -> Mesh {
    let mut vertices = Vec::new();
    for z in [0.0, 20.0] {
        for i in 0..n {
            let angle = std::f64::consts::TAU * f64::from(i) / f64::from(n);
            vertices.push([(10.0 * angle.cos()) as f32, (10.0 * angle.sin()) as f32, z]);
        }
    }
    vertices.push([0.0; 3]);
    if top {
        vertices.push([0.0, 0.0, 20.0]);
    }
    let mut faces = Vec::new();
    for i in 0..n {
        let j = (i + 1) % n;
        faces.extend([[i, j, n + j], [i, n + j, n + i], [2 * n, j, i]]);
        if top {
            faces.push([2 * n + 1, n + i, n + j]);
        }
    }
    Mesh::new(vertices, faces).expect("cylinder")
}

fn sphere() -> Mesh {
    let (nu, nv) = (64_u32, 32_u32);
    let mut vertices = vec![[0.0, 0.0, 15.0]];
    for i in 1..nv {
        let theta = std::f64::consts::PI * f64::from(i) / f64::from(nv);
        for j in 0..nu {
            let phi = std::f64::consts::TAU * f64::from(j) / f64::from(nu);
            vertices.push([
                (15.0 * theta.sin() * phi.cos()) as f32,
                (15.0 * theta.sin() * phi.sin()) as f32,
                (15.0 * theta.cos()) as f32,
            ]);
        }
    }
    let south = vertices.len() as u32;
    vertices.push([0.0, 0.0, -15.0]);
    let index = |i, j| 1 + (i - 1) * nu + j % nu;
    let mut faces = Vec::new();
    for j in 0..nu {
        faces.push([0, index(1, j), index(1, j + 1)]);
    }
    for i in 1..nv - 1 {
        for j in 0..nu {
            let (a, b, c, d) = (
                index(i, j),
                index(i, j + 1),
                index(i + 1, j),
                index(i + 1, j + 1),
            );
            faces.extend([[a, c, d], [a, d, b]]);
        }
    }
    for j in 0..nu {
        faces.push([south, index(nv - 1, j + 1), index(nv - 1, j)]);
    }
    Mesh::new(vertices, faces).expect("sphere")
}

fn box_at(lo: [f32; 3], hi: [f32; 3]) -> Mesh {
    let [x, y, z] = lo;
    let [xx, yy, zz] = hi;
    Mesh::new(
        vec![
            [x, y, z],
            [xx, y, z],
            [xx, yy, z],
            [x, yy, z],
            [x, y, zz],
            [xx, y, zz],
            [xx, yy, zz],
            [x, yy, zz],
        ],
        vec![
            [0, 2, 1],
            [0, 3, 2],
            [4, 5, 6],
            [4, 6, 7],
            [0, 1, 5],
            [0, 5, 4],
            [1, 2, 6],
            [1, 6, 5],
            [2, 3, 7],
            [2, 7, 6],
            [3, 0, 4],
            [3, 4, 7],
        ],
    )
    .expect("box")
}

fn volume(mesh: &Mesh) -> f64 {
    mesh.faces
        .iter()
        .map(|face| {
            let [a, b, c] = face.map(|i| mesh.vertices[i as usize].map(f64::from));
            (a[0] * (b[1] * c[2] - b[2] * c[1])
                + a[1] * (b[2] * c[0] - b[0] * c[2])
                + a[2] * (b[0] * c[1] - b[1] * c[0]))
                / 6.0
        })
        .sum()
}

fn repaired(mesh: &Mesh) -> RepairResult {
    let result = repair_mesh(mesh).expect("native repair");
    assert_eq!(
        result.report.verdict,
        RepairVerdict::Improved,
        "{:?}",
        result.report
    );
    assert!(result.report.can_apply);
    assert_ne!(repair_summary(&result.report), REPAIR_REFUSED_MESSAGE);
    result
}

fn valid_closed(result: &RepairResult) {
    let health = inspect_mesh(&result.mesh).expect("health");
    assert!(health.valid, "{health:?}");
    assert_eq!(health.boundary_edges, 0);
}

#[test]
fn f2_open_cylinder() {
    let mesh = cylinder(64, false);
    assert_eq!(inspect_mesh(&mesh).expect("health").boundary_edges, 64);
    let result = repaired(&mesh);
    valid_closed(&result);
    assert!(result.mesh.vertex_count() <= mesh.vertex_count());
}

#[test]
fn f3_holey_sphere() {
    let mut mesh = sphere();
    let mut rng = Random(42);
    mesh.faces.retain(|_| rng.next() > 0.03);
    let result = repaired(&mesh);
    valid_closed(&result);
    assert_eq!(result.mesh.vertex_count(), mesh.vertex_count());
}

#[test]
fn f4_flipped_sphere() {
    let mut mesh = sphere();
    let mut rng = Random(7);
    for face in &mut mesh.faces {
        if rng.next() < 0.2 {
            face.swap(1, 2);
        }
    }
    let result = repaired(&mesh);
    valid_closed(&result);
    assert_eq!(result.report.score_after.flipped_edges, 0);
    assert!(volume(&result.mesh) > 0.0);
}

#[test]
fn f5_soup_cracked_cylinder() {
    let source = cylinder(64, true);
    let mut rng = Random(7);
    let mut vertices = Vec::new();
    let mut faces = Vec::new();
    for face in source.faces {
        let mut points = face.map(|i| source.vertices[i as usize]);
        for point in &mut points {
            for value in point {
                *value += (rng.next() * 0.0006 - 0.0003) as f32;
            }
        }
        let positive_side = points.iter().map(|p| p[1]).sum::<f32>() > 0.0;
        for point in &mut points {
            if positive_side && point[1].atan2(point[0]).abs() <= 0.001 {
                point[1] += 0.01;
            }
        }
        let base = vertices.len() as u32;
        vertices.extend(points);
        faces.push([base, base + 1, base + 2]);
    }
    let result = repaired(&Mesh::new(vertices, faces).expect("soup"));
    valid_closed(&result);
    assert_eq!(result.report.score_after.shells, 1);
}

#[test]
fn f6_overlapping_boxes() {
    let mesh = Mesh::concatenate(&[box_at([0.0; 3], [20.0; 3]), box_at([10.0; 3], [30.0; 3])])
        .expect("overlapping boxes");
    let result = repaired(&mesh);
    valid_closed(&result);
    assert_eq!(result.report.score_after.shells, 1);
    assert!((volume(&result.mesh) - 15000.0).abs() <= 0.01);
}

#[test]
fn f7_nonmanifold_edge_and_fin() {
    let mut mesh = Mesh::concatenate(&[
        box_at([0.0; 3], [10.0; 3]),
        box_at([10.0, 10.0, 0.0], [20.0, 20.0, 10.0]),
    ])
    .expect("edge-touching boxes");
    // Weld the two endpoints of the shared vertical edge.
    for face in &mut mesh.faces {
        for index in face {
            let point = mesh.vertices[*index as usize];
            if point == [10.0, 10.0, 0.0] || point == [10.0, 10.0, 10.0] {
                *index = mesh
                    .vertices
                    .iter()
                    .position(|p| *p == point)
                    .expect("endpoint") as u32;
            }
        }
    }
    let fin = mesh.vertices.len() as u32;
    mesh.vertices.push([5.0, 5.0, 15.0]);
    let endpoint = |point| {
        mesh.vertices
            .iter()
            .position(|p| *p == point)
            .expect("fin endpoint") as u32
    };
    mesh.faces
        .push([endpoint([0.0, 0.0, 10.0]), endpoint([10.0, 0.0, 10.0]), fin]);
    let result = repaired(&mesh);
    assert_eq!(result.report.score_after.non_manifold_edges, 0);
    assert_eq!(result.report.score_after.shells, 2);
    assert_eq!(
        result.mesh.face_count(),
        24,
        "fin removed, both cubes retained"
    );
    assert!(!result.mesh.vertices.contains(&[5.0, 5.0, 15.0]));
    assert!((volume(&result.mesh) - 2000.0).abs() <= 0.01);
}

#[test]
fn f8_duplicate_faces() {
    let mut mesh = box_at([0.0; 3], [10.0; 3]);
    mesh.faces.extend_from_within(..2);
    let result = repaired(&mesh);
    valid_closed(&result);
    assert_eq!(result.mesh.face_count(), 12);
}

#[test]
fn f9_inside_out_sphere() {
    let mut mesh = sphere();
    for face in &mut mesh.faces {
        face.swap(1, 2);
    }
    let result = repaired(&mesh);
    valid_closed(&result);
    assert!(volume(&result.mesh) > 0.0);
}

#[test]
fn f10_sealed_sliver() {
    let mut mesh = box_at([0.0; 3], [10.0; 3]);
    let index = |point| {
        mesh.vertices
            .iter()
            .position(|p| *p == point)
            .expect("top corner") as u32
    };
    let (a, b, c) = (
        index([0.0, 0.0, 10.0]),
        index([10.0, 0.0, 10.0]),
        index([10.0; 3]),
    );
    assert!(mesh.faces.contains(&[a, b, c]));
    mesh.faces.retain(|f| *f != [a, b, c]);
    let tip = mesh.vertices.len() as u32;
    mesh.vertices.push([5.0, 0.00005, 10.0]);
    mesh.faces.extend([[a, tip, c], [tip, b, c], [a, b, tip]]);
    assert!(inspect_mesh(&mesh).expect("health").valid);
    let result = repair_mesh(&mesh).expect("native repair");
    assert_ne!(result.report.verdict, RepairVerdict::Worse);
    assert_eq!(
        inspect_mesh(&result.mesh).expect("health").boundary_edges,
        0
    );
    assert_ne!(repair_summary(&result.report), REPAIR_REFUSED_MESSAGE);
}

#[test]
fn open500() {
    let mesh = cylinder(500, false);
    assert_eq!(inspect_mesh(&mesh).expect("health").boundary_edges, 500);
    valid_closed(&repaired(&mesh));
}
