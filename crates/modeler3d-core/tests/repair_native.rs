use std::collections::{BTreeMap, HashMap};

use modeler3d_core::{Mesh, RepairResult, RepairVerdict, inspect_mesh, make_sphere, repair_mesh};

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

fn cube() -> Mesh {
    box_at([0.0; 3], [10.0; 3])
}

fn sliver(height: f32) -> Mesh {
    let mut mesh = cube();
    mesh.vertices.push([5.0, height, 10.0]);
    mesh.faces.retain(|face| *face != [4, 5, 6]);
    mesh.faces.extend([[4, 8, 6], [8, 5, 6], [4, 5, 8]]);
    mesh
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|i| a[i] - b[i])
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a.into_iter().zip(b).map(|(a, b)| a * b).sum()
}
fn volume(mesh: &Mesh) -> f64 {
    mesh.faces
        .iter()
        .map(|face| {
            let [a, b, c] = face.map(|v| mesh.vertices[v as usize].map(f64::from));
            dot(a, cross(b, c)) / 6.0
        })
        .sum()
}
fn open_edges(mesh: &Mesh) -> usize {
    let mut edges = BTreeMap::new();
    for face in &mesh.faces {
        for i in 0..3 {
            let (a, b) = (face[i], face[(i + 1) % 3]);
            *edges.entry((a.min(b), a.max(b))).or_insert(0) += 1;
        }
    }
    edges.values().filter(|&&count| count == 1).count()
}
fn step(mesh: &Mesh) -> f64 {
    let (lo, hi) = mesh.bounds().expect("bounds");
    0.002_f64.min(
        (0..3)
            .map(|i| f64::from(hi[i]) - f64::from(lo[i]))
            .fold(0.0, f64::max)
            / 2500.0,
    )
}
fn min_height(mesh: &Mesh, top_only: bool) -> f64 {
    mesh.faces
        .iter()
        .filter_map(|face| {
            let [a, b, c] = face.map(|v| mesh.vertices[v as usize].map(f64::from));
            if top_only && [a, b, c].iter().any(|p| (p[2] - 10.0).abs() > 1e-5) {
                return None;
            }
            let ab = sub(b, a);
            let ac = sub(c, a);
            let bc = sub(c, b);
            let normal = cross(ab, ac);
            Some(dot(normal, normal).sqrt() / dot(ab, ab).max(dot(ac, ac)).max(dot(bc, bc)).sqrt())
        })
        .fold(f64::INFINITY, f64::min)
}

// Local grid lookup covers every allowed move. For a larger move, examine
// the remaining occupied cells whose boxes can contain a nearer point.
fn max_move(input: &Mesh, output: &Mesh, limit: f64) -> f64 {
    let cell = |p: [f32; 3]| p.map(|v| (f64::from(v) / limit).floor() as i64);
    let mut buckets = HashMap::<[i64; 3], Vec<[f64; 3]>>::new();
    for &p in &input.vertices {
        buckets.entry(cell(p)).or_default().push(p.map(f64::from));
    }
    output
        .vertices
        .iter()
        .map(|&p| {
            let key = cell(p);
            let mut nearest = f64::INFINITY;
            for x in -1..=1 {
                for y in -1..=1 {
                    for z in -1..=1 {
                        if let Some(points) = buckets.get(&[key[0] + x, key[1] + y, key[2] + z]) {
                            for &other in points {
                                let delta = sub(p.map(f64::from), other);
                                nearest = nearest.min(dot(delta, delta).sqrt());
                            }
                        }
                    }
                }
            }
            if nearest > limit {
                for (key, points) in &buckets {
                    let lower = (0..3)
                        .map(|i| {
                            let lo = key[i] as f64 * limit;
                            let hi = lo + limit;
                            let p = f64::from(p[i]);
                            let distance = (lo - p).max(p - hi).max(0.0);
                            distance * distance
                        })
                        .sum::<f64>();
                    if lower < nearest * nearest {
                        for &other in points {
                            let delta = sub(p.map(f64::from), other);
                            nearest = nearest.min(dot(delta, delta).sqrt());
                        }
                    }
                }
            }
            nearest
        })
        .fold(0.0, f64::max)
}
fn check_bound(input: &Mesh, output: &Mesh, weld: bool) {
    let step = step(input);
    let bound = step * (3.0_f64.sqrt() / 2.0 + if weld { 5.0 } else { 0.0 }) + 1e-5;
    let moved = max_move(input, output, bound);
    assert!(moved <= bound, "max move {moved} exceeds {bound}");
}
fn repair(input: &Mesh) -> RepairResult {
    let result = repair_mesh(input).expect("native repair");
    // The public check delegates to Mesh::validate (crate-private).
    result.mesh.validate_non_empty().expect("valid candidate");
    assert!(
        result.report.errors.is_empty(),
        "{:?}",
        result.report.errors
    );
    println!(
        "faces={} open_edges={} non_manifold_edges={} shells={} volume={:.6} verdict={:?}",
        result.mesh.face_count(),
        open_edges(&result.mesh),
        result.report.score_after.non_manifold_edges,
        result.report.score_after.shells,
        volume(&result.mesh),
        result.report.verdict
    );
    println!("planar_cleanup={:?}", result.report.planar_cleanup);
    result
}
fn sealed(result: &RepairResult, shells: usize) {
    assert_eq!(open_edges(&result.mesh), 0);
    assert_eq!(result.report.score_after.non_manifold_edges, 0);
    assert_eq!(result.report.score_after.shells, shells);
}

#[test]
fn f10_sub_grid_sliver_collapses_without_opening_the_box() {
    let input = sliver(0.00005);
    let result = repair(&input);
    sealed(&result, 1);
    assert!(min_height(&result.mesh, false) >= 0.5 * step(&input));
    assert!((volume(&result.mesh) - 1000.0).abs() <= 5.0);
    check_bound(&input, &result.mesh, false);
}

#[test]
fn nonzero_lattice_sliver_is_kept() {
    let input = sliver(0.01);
    let result = repair(&input);
    sealed(&result, 1);
    let top_count = |mesh: &Mesh| {
        mesh.faces
            .iter()
            .filter(|face| {
                face.iter()
                    .all(|&v| (mesh.vertices[v as usize][2] - 10.0).abs() < 1e-5)
            })
            .count()
    };
    assert_eq!(top_count(&input), top_count(&result.mesh));
    assert!((min_height(&result.mesh, true) - 0.01).abs() <= step(&input));
    check_bound(&input, &result.mesh, false);
}

#[test]
fn needle_corner_fan_merges_and_stays_sealed() {
    let mut input = cube();
    // Distance 0.001 from the corner, with neither axis on a half-cell tie.
    let offset = 0.001_f32 / 2.0_f32.sqrt();
    input.vertices.push([offset, offset, 10.0]);
    input.faces.retain(|face| *face != [4, 5, 6]);
    input.faces.extend([[4, 8, 6], [8, 5, 6], [4, 5, 8]]);
    let result = repair(&input);
    sealed(&result, 1);
    assert!(!result.mesh.vertices.contains(&[offset, offset, 10.0]));
    assert_eq!(result.mesh.vertex_count(), 8);
    check_bound(&input, &result.mesh, false);
}

#[test]
fn book_like_distinct_collinear_corners_collapse_without_a_hole() {
    let input = sliver(0.0);
    assert_eq!(inspect_mesh(&input).expect("health").degenerate_faces, 1);
    let result = repair(&input);
    sealed(&result, 1);
    assert_eq!(result.report.score_after.degenerate_faces, 0);
    assert_eq!(result.report.verdict, RepairVerdict::Improved);
    assert!(result.report.can_apply);
    check_bound(&input, &result.mesh, false);
}

fn cracked_sphere(gap: f32) -> Mesh {
    let mut sphere = make_sphere(10.0, 2).expect("sphere");
    let mut copies = BTreeMap::new();
    for face in &mut sphere.faces {
        // The five neighbours of an original icosphere corner form a
        // complete edge ring. Detach that cap and shift only its ring copy.
        if !face.contains(&0) {
            continue;
        }
        for v in face {
            if *v != 0 {
                let copy = *copies.entry(*v).or_insert_with(|| {
                    let mut point = sphere.vertices[*v as usize];
                    point[1] += gap;
                    let next = sphere.vertices.len() as u32;
                    sphere.vertices.push(point);
                    next
                });
                *v = copy;
            }
        }
    }
    assert_eq!(copies.len(), 5);
    assert_eq!(open_edges(&sphere), 10);
    sphere
}

#[test]
fn crack_within_five_steps_welds() {
    let input = cracked_sphere(0.008);
    let result = repair(&input);
    sealed(&result, 1);
    check_bound(&input, &result.mesh, true);
}

#[test]
fn crack_wider_than_radius_is_not_zipped() {
    let input = cracked_sphere(0.03);
    let result = repair(&input);
    assert!(open_edges(&result.mesh) > 0);
    let stitch = &result.report.stages[0];
    assert_eq!(stitch.before_vertices, stitch.after_vertices);
    check_bound(&input, &result.mesh, true);
}

#[test]
fn closed_cubes_five_microns_apart_do_not_weld() {
    let input = Mesh::concatenate(&[cube(), box_at([10.005, 0.0, 0.0], [20.005, 10.0, 10.0])])
        .expect("two cubes");
    let result = repair(&input);
    sealed(&result, 2);
    check_bound(&input, &result.mesh, false);
}

#[test]
fn t_junction_point_four_microns_off_edge_is_absorbed() {
    let mut input = sliver(0.004);
    input.faces.retain(|face| *face != [4, 5, 8]);
    assert_eq!(open_edges(&input), 3);
    let result = repair(&input);
    sealed(&result, 1);
    let stitch = &result.report.stages[0];
    assert_eq!(stitch.after_triangles, 14);
    check_bound(&input, &result.mesh, true);
}

fn f7() -> Mesh {
    let mut input =
        Mesh::concatenate(&[cube(), box_at([10.0, 10.0, 0.0], [20.0, 20.0, 10.0])]).expect("cubes");
    for face in &mut input.faces {
        for v in face {
            if *v == 8 {
                *v = 2;
            } else if *v == 12 {
                *v = 6;
            }
        }
    }
    input.vertices.push([5.0, 5.0, 15.0]);
    input.faces.push([4, 5, 16]);
    input
}

#[test]
fn f7_touching_cubes_split_and_the_planar_fin_disappears() {
    let input = f7();
    let result = repair(&input);
    sealed(&result, 2);
    assert!((volume(&result.mesh) - 2000.0).abs() <= 1.0);
    assert!(!result.mesh.vertices.contains(&[5.0, 5.0, 15.0]));
    assert_eq!(result.report.verdict, RepairVerdict::Improved);
    check_bound(&input, &result.mesh, true);
}

#[test]
fn f8_same_winding_duplicates_are_removed() {
    let mut input = cube();
    input.faces.extend([input.faces[0], input.faces[1]]);
    let result = repair(&input);
    sealed(&result, 1);
    assert_eq!(result.mesh.face_count(), 12);
    assert_eq!(result.report.score_after.duplicate_faces, 0);
    assert_eq!(result.report.verdict, RepairVerdict::Improved);
}

fn soup_cylinder() -> Mesh {
    let n = 64;
    let mut vertices = Vec::new();
    for z in [0.0, 20.0] {
        for i in 0..n {
            let a = std::f64::consts::TAU * f64::from(i) / f64::from(n);
            vertices.push([(10.0 * a.cos()) as f32, (10.0 * a.sin()) as f32, z]);
        }
    }
    vertices.extend([[0.0; 3], [0.0, 0.0, 20.0]]);
    let mut faces = Vec::new();
    for i in 0..n {
        let j = (i + 1) % n;
        faces.extend([
            [i, j, n + j],
            [i, n + j, n + i],
            [2 * n, j, i],
            [2 * n + 1, n + i, n + j],
        ]);
    }
    let mut seed = 7_u64;
    let mut soup = Vec::new();
    for face in &faces {
        let positive = face.iter().map(|&v| vertices[v as usize][1]).sum::<f32>() > 0.0;
        for &v in face {
            let mut point = vertices[v as usize];
            for coordinate in &mut point {
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                *coordinate += ((seed >> 32) as f64 / f64::from(u32::MAX) * 0.0006 - 0.0003) as f32;
            }
            if positive && point[1].atan2(point[0]).abs() <= 0.001 {
                point[1] += 0.01;
            }
            soup.push(point);
        }
    }
    Mesh::new(
        soup,
        (0..faces.len() as u32)
            .map(|i| [3 * i, 3 * i + 1, 3 * i + 2])
            .collect(),
    )
    .expect("soup")
}

#[test]
fn f5_jittered_unwelded_cylinder_crack_is_repaired() {
    let input = soup_cylinder();
    let result = repair(&input);
    sealed(&result, 1);
    assert_eq!(result.report.verdict, RepairVerdict::Improved);
    check_bound(&input, &result.mesh, true);
}

#[test]
fn native_repair_is_deterministic() {
    for input in [
        sliver(0.00005),
        sliver(0.01),
        cracked_sphere(0.008),
        f7(),
        soup_cylinder(),
    ] {
        let first = repair(&input);
        let second = repair(&input);
        assert_eq!(first, second);
    }
}
