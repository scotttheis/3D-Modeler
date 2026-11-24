use std::collections::{BTreeMap, HashMap};

use modeler3d_core::{
    Mesh, RepairResult, RepairVerdict, inspect_mesh, make_sphere, repair_mesh, repair_summary,
};

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

fn r6_closed(result: &RepairResult, shells: i32) {
    result.mesh.validate_non_empty().expect("validated result");
    let health = inspect_mesh(&result.mesh).expect("health");
    assert_eq!(health.boundary_edges, 0, "{health:?}");
    assert_eq!(health.non_manifold_edges, 0, "{health:?}");
    assert_eq!(health.flipped_edges, 0, "{health:?}");
    assert_eq!(health.self_intersections, Some(0), "{health:?}");
    assert_eq!(health.body_count, shells, "{health:?}");
}

#[test]
fn r6_f6_overlapping_boxes_union_and_are_deterministic() {
    // genfx.py f6: a 10 mm overlap on all axes, intersection 1,000 mm3.
    let input = Mesh::concatenate(&[box_at([0.0; 3], [20.0; 3]), box_at([10.0; 3], [30.0; 3])])
        .expect("f6");
    assert!((volume(&input) - 16_000.0).abs() < 1.0e-8);
    assert!(
        inspect_mesh(&input)
            .expect("before")
            .self_intersections
            .is_some_and(|n| n > 0)
    );
    let result = repair(&input);
    assert_eq!(result.report.verdict, RepairVerdict::Improved);
    assert!(result.report.can_apply);
    r6_closed(&result, 1);
    assert!((volume(&result.mesh) - 15_000.0).abs() <= 0.5);
    assert_eq!(result, repair_mesh(&input).expect("repeat"));
}

#[test]
fn r6_overlapping_spheres_union_even_when_one_is_inside_out() {
    let outer = make_sphere(15.0, 4).expect("sphere");
    let mut other = outer.clone();
    for point in &mut other.vertices {
        point[0] += 15.0;
    }
    for face in &mut other.faces {
        face.swap(1, 2);
    }
    let input = Mesh::concatenate(&[outer, other]).expect("spheres");
    let result = repair(&input);
    assert_eq!(result.report.verdict, RepairVerdict::Improved);
    r6_closed(&result, 1);
    let r = 15.0_f64;
    let d = 15.0;
    let analytic = 2.0 * 4.0 / 3.0 * std::f64::consts::PI * r.powi(3)
        - std::f64::consts::PI * (4.0 * r + d) * (2.0 * r - d).powi(2) / 12.0;
    assert!((volume(&result.mesh) / analytic - 1.0).abs() < 0.01);
}

#[test]
fn r6_nested_outward_spheres_keep_only_the_outer_solid() {
    let outer = make_sphere(15.0, 3).expect("outer");
    let outer_volume = volume(&outer);
    let inner = make_sphere(5.0, 3).expect("wrong-way inner");
    let result = repair(&Mesh::concatenate(&[outer, inner]).expect("nested spheres"));
    assert_eq!(result.report.verdict, RepairVerdict::Improved);
    r6_closed(&result, 1);
    assert!((volume(&result.mesh) / outer_volume - 1.0).abs() < 0.01);
}

#[test]
fn r6_holey_hollow_ball_keeps_its_correct_cavity() {
    let mut outer = make_sphere(15.0, 3).expect("outer");
    let mut inner = make_sphere(5.0, 3).expect("inner");
    let expected_volume = volume(&outer) - volume(&inner);
    outer.faces.pop();
    for face in &mut inner.faces {
        face.swap(1, 2);
    }
    let input = Mesh::concatenate(&[outer, inner]).expect("hollow");
    let result = repair(&input);
    assert_eq!(result.report.verdict, RepairVerdict::Improved);
    assert!(result.report.can_apply);
    r6_closed(&result, 2);
    assert_eq!(result.report.after.cavity_shells, 1);
    assert!((volume(&result.mesh) / expected_volume - 1.0).abs() < 0.01);
    let cavity_vertices: Vec<_> = result
        .mesh
        .faces
        .iter()
        .filter(|face| {
            face.iter().all(|&v| {
                dot(
                    result.mesh.vertices[v as usize].map(f64::from),
                    result.mesh.vertices[v as usize].map(f64::from),
                ) < 26.0
            })
        })
        .map(|face| face.map(|v| result.mesh.vertices[v as usize]))
        .collect();
    assert_eq!(cavity_vertices.len(), 1280);
    assert!(
        cavity_vertices
            .iter()
            .map(|&[a, b, c]| dot(a.map(f64::from), cross(b.map(f64::from), c.map(f64::from))))
            .sum::<f64>()
            < 0.0
    );
}

#[test]
fn r6_wrong_way_hollow_ball_is_merged_into_a_solid() {
    let mut outer = make_sphere(15.0, 3).expect("outer");
    let expected = volume(&outer);
    outer.faces.pop();
    let inner = make_sphere(5.0, 3).expect("outward inner");
    let result = repair(&Mesh::concatenate(&[outer, inner]).expect("wrong cavity"));
    assert!(result.report.can_apply);
    r6_closed(&result, 1);
    assert_eq!(result.report.after.cavity_shells, 0);
    assert!((volume(&result.mesh) / expected - 1.0).abs() < 0.01);
}

#[test]
fn r6_cavity_survives_until_a_nontrivial_outer_hole_has_closed() {
    let mut outer = make_sphere(15.0, 1).expect("outer");
    outer.faces.retain(|face| !face.contains(&12));
    let mut inner = make_sphere(5.0, 1).expect("inner");
    for face in &mut inner.faces {
        face.swap(1, 2);
    }
    let input = Mesh::concatenate(&[outer, inner]).expect("hollow with six-edge hole");
    assert_eq!(inspect_mesh(&input).expect("before").boundary_edges, 6);
    let result = repair(&input);
    assert!(result.report.can_apply, "{:?}", result.report);
    r6_closed(&result, 2);
    assert_eq!(result.report.after.cavity_shells, 1);
}

#[test]
fn r6_touching_parts_and_a_small_gap_stay_bit_identical() {
    for lo in [
        [10.0, 0.0, 0.0],
        [10.0, 10.0, 0.0],
        [10.0; 3],
        [10.05, 0.0, 0.0],
    ] {
        let input =
            Mesh::concatenate(&[cube(), box_at(lo, lo.map(|x| x + 10.0))]).expect("two parts");
        assert_eq!(
            inspect_mesh(&input).expect("before").self_intersections,
            Some(0)
        );
        let result = repair(&input);
        r6_closed(&result, 2);
        assert_eq!(result.mesh, input);
        assert_eq!(result.report.verdict, RepairVerdict::NoBetter);
        assert!(!result.report.can_apply);
    }
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
    // R5 caps the two five-edge loops separately, without zipping the gap.
    sealed(&result, 2);
    assert_eq!(result.mesh.face_count(), input.face_count() + 2 * (5 - 2));
    assert_eq!(result.mesh.vertex_count(), input.vertex_count());
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

fn open_cylinder(n: u32) -> Mesh {
    let mut vertices = Vec::new();
    for z in [0.0, 20.0] {
        for i in 0..n {
            let angle = std::f64::consts::TAU * f64::from(i) / f64::from(n);
            vertices.push([(10.0 * angle.cos()) as f32, (10.0 * angle.sin()) as f32, z]);
        }
    }
    vertices.push([0.0; 3]);
    let mut faces = Vec::new();
    for i in 0..n {
        let j = (i + 1) % n;
        faces.extend([[i, j, n + j], [i, n + j, n + i], [2 * n, j, i]]);
    }
    Mesh::new(vertices, faces).expect("open cylinder")
}

fn check_fill(input: &Mesh, result: &RepairResult) {
    result
        .mesh
        .validate_non_empty()
        .expect("Mesh::validate via public check");
    assert!(
        result.report.errors.is_empty(),
        "{:?}",
        result.report.errors
    );
    let health = inspect_mesh(&result.mesh).expect("filled mesh health");
    assert_eq!(health.boundary_edges, 0);
    assert_eq!(health.non_manifold_edges, 0);
    assert_eq!(health.degenerate_faces, 0);
    assert_eq!(health.flipped_edges, 0);
    assert_eq!(
        result.report.verdict,
        RepairVerdict::Improved,
        "{:?}",
        result.report
    );
    assert!(result.report.can_apply);
    check_bound(input, &result.mesh, false);
}

fn top_triangles(mesh: &Mesh) -> Vec<[u32; 3]> {
    mesh.faces
        .iter()
        .copied()
        .filter(|f| f.iter().all(|&v| mesh.vertices[v as usize][2] == 20.0))
        .collect()
}

#[test]
fn r5_cylinders_fill_with_n_minus_two_flat_triangles_and_no_new_points() {
    for n in [3, 4, 10, 64, 300, 301, 500] {
        let input = open_cylinder(n);
        let start = std::time::Instant::now();
        let result = repair_mesh(&input).expect("cylinder repair");
        let elapsed = start.elapsed();
        check_fill(&input, &result);
        sealed(&result, 1);
        assert_eq!(top_triangles(&result.mesh).len(), n as usize - 2);
        assert_eq!(
            result.mesh.face_count(),
            input.face_count() + n as usize - 2
        );
        assert_eq!(result.mesh.vertex_count(), input.vertex_count());
        // Compare to the same closed polygonal cylinder (also meaningful at n=3).
        let polygon_volume =
            0.5 * f64::from(n) * 100.0 * (std::f64::consts::TAU / f64::from(n)).sin() * 20.0;
        assert!((volume(&result.mesh) / polygon_volume - 1.0).abs() < 0.01);
        if n == 64 || n == 500 {
            assert!((volume(&result.mesh) / (std::f64::consts::PI * 2000.0) - 1.0).abs() < 0.01);
        }
        if n == 500 && !cfg!(debug_assertions) {
            println!("r5 open500 Release seconds={:.6}", elapsed.as_secs_f64());
            assert!(elapsed < std::time::Duration::from_secs(1), "{elapsed:?}");
        }
        let trivial = &result.report.stages[1];
        assert_eq!(
            trivial.after_triangles - trivial.before_triangles,
            if n <= 4 { n as usize - 2 } else { 0 }
        );
    }
}

#[test]
fn r5_holey_sphere_seals_without_moving_points_off_the_input_surface() {
    let closed = make_sphere(15.0, 3).expect("sphere");
    let mut input = closed.clone();
    // About 3%, with no dependence on a platform RNG or mesh hashes.
    input.faces = input
        .faces
        .into_iter()
        .enumerate()
        .filter_map(|(i, f)| (i % 33 != 7).then_some(f))
        .collect();
    let result = repair_mesh(&input).expect("holey sphere");
    check_fill(&input, &result);
    sealed(&result, 1);
    assert!((volume(&result.mesh) / volume(&closed) - 1.0).abs() < 0.01);
    assert!(max_move(&input, &result.mesh, 0.002) < 0.002);
}

fn collinear_top_box() -> Mesh {
    let mut input = cube();
    input
        .faces
        .retain(|face| *face != [4, 5, 6] && *face != [4, 6, 7]);
    for (a, b) in [(4, 5), (5, 6), (6, 7), (7, 4)] {
        let middle = input.vertices.len() as u32;
        input.vertices.push(std::array::from_fn(|axis| {
            (input.vertices[a][axis] + input.vertices[b][axis]) / 2.0
        }));
        let id = input
            .faces
            .iter()
            .position(|face| face.contains(&(a as u32)) && face.contains(&(b as u32)))
            .expect("side face");
        let face = input.faces.remove(id);
        let edge = (0..3)
            .find(|&i| {
                [face[i], face[(i + 1) % 3]].contains(&(a as u32))
                    && [face[i], face[(i + 1) % 3]].contains(&(b as u32))
            })
            .expect("directed edge");
        let (a, b, c) = (face[edge], face[(edge + 1) % 3], face[(edge + 2) % 3]);
        input.faces.extend([[a, middle, c], [middle, b, c]]);
    }
    input
}

#[test]
fn r5_eight_point_flat_hole_keeps_collinear_side_points_without_zero_area_faces() {
    let input = collinear_top_box();
    assert_eq!(open_edges(&input), 8);
    let result = repair_mesh(&input).expect("collinear rim");
    check_fill(&input, &result);
    assert_eq!(result.mesh.face_count(), input.face_count() + 6);
    assert!((volume(&result.mesh) - 1000.0).abs() < 1e-5);
}

fn nonplanar_cup() -> Mesh {
    let mut vertices = vec![
        [10.0, 0.0, 12.0],
        [8.66, 5.0, 10.0],
        [5.0, 8.66, 14.0],
        [0.0, 10.0, 11.0],
        [-5.0, 8.66, 13.0],
        [-8.66, 5.0, 10.0],
        [-10.0, 0.0, 12.0],
        [-8.66, -5.0, 14.0],
        [-5.0, -8.66, 11.0],
        [0.0, -10.0, 13.0],
        [5.0, -8.66, 10.0],
        [8.66, -5.0, 14.0],
    ];
    vertices.push([0.0; 3]);
    Mesh::new(vertices, (0..12).map(|i| [12, (i + 1) % 12, i]).collect()).expect("cup")
}

fn triangle_area(points: [[f64; 3]; 3]) -> f64 {
    let normal = cross(sub(points[1], points[0]), sub(points[2], points[0]));
    dot(normal, normal).sqrt() * 0.5
}

// Exhaustive recursive search, without the production DP's table or splitting
// code. Twelve points have only 16,796 triangulations.
fn brute_area(points: &[[f64; 3]]) -> f64 {
    if points.len() < 3 {
        return 0.0;
    }
    (1..points.len() - 1)
        .map(|m| {
            let area = triangle_area([points[0], points[m], points[points.len() - 1]]);
            if area == 0.0 {
                return f64::INFINITY;
            }
            area + brute_area(&points[..=m]) + brute_area(&points[m..])
        })
        .fold(f64::INFINITY, f64::min)
}

#[test]
fn r5_nonplanar_cup_matches_the_exhaustive_minimum_area_fill() {
    let input = nonplanar_cup();
    let result = repair_mesh(&input).expect("cup repair");
    check_fill(&input, &result);
    let rim: Vec<_> = input.vertices[..12]
        .iter()
        .map(|p| {
            result
                .mesh
                .vertices
                .iter()
                .copied()
                .min_by(|a, b| {
                    let da = sub(a.map(f64::from), p.map(f64::from));
                    let db = sub(b.map(f64::from), p.map(f64::from));
                    dot(da, da).total_cmp(&dot(db, db))
                })
                .expect("rim survives")
                .map(f64::from)
        })
        .collect();
    let cap: Vec<_> = result
        .mesh
        .faces
        .iter()
        .filter(|f| f.iter().all(|&v| result.mesh.vertices[v as usize][2] > 5.0))
        .collect();
    assert_eq!(cap.len(), 10);
    let actual: f64 = cap
        .iter()
        .map(|face| triangle_area(face.map(|v| result.mesh.vertices[v as usize].map(f64::from))))
        .sum();
    let minimum = brute_area(&rim);
    assert!(
        (actual - minimum).abs() < minimum * 1e-12,
        "{actual} != {minimum}"
    );
}

#[test]
fn r5_fill_avoids_the_smaller_diagonal_already_used_by_the_mesh() {
    let input = Mesh::new(
        vec![
            [0.0, -1.0, 0.0],
            [0.0, 1.0, 0.0],
            [-10.0, 0.0, 10.0],
            [10.0, 0.0, 10.0],
        ],
        vec![[0, 2, 1], [0, 1, 3]],
    )
    .expect("two triangles sharing a forbidden diagonal");
    let result = repair_mesh(&input).expect("constrained cap");
    check_fill(&input, &result);
    assert_eq!(result.mesh.face_count(), 4);
    assert_eq!(result.mesh.vertex_count(), 4);
    assert!((volume(&result.mesh) - 200.0 / 3.0).abs() < 1e-5);
}

#[test]
fn r5_fill_preserves_winding_for_outward_and_inside_out_holes() {
    for mut input in [open_cylinder(64), nonplanar_cup()] {
        for inward in [false, true] {
            if inward {
                for face in &mut input.faces {
                    face.swap(1, 2);
                }
            }
            let result = repair_mesh(&input).expect("oriented fill");
            check_fill(&input, &result);
            assert_eq!(result.report.after.inside_out_shells, 0);
            assert!(volume(&result.mesh) > 0.0);
        }
    }
}

#[test]
fn r5_hole_fills_are_deterministic_on_both_algorithm_paths() {
    for input in [
        open_cylinder(64),
        open_cylinder(500),
        collinear_top_box(),
        nonplanar_cup(),
    ] {
        let first = repair_mesh(&input).expect("first repair");
        let second = repair_mesh(&input).expect("second repair");
        check_fill(&input, &first);
        check_fill(&input, &second);
        assert_eq!(first, second);
    }
}

#[test]
fn r5_simple_profile_uses_the_same_no_new_point_fill() {
    for n in [4, 10, 500] {
        let input = open_cylinder(n);
        let result =
            modeler3d_core::repair_mesh_with_profile(&input, "simple").expect("simple fill");
        check_fill(&input, &result);
        assert_eq!(top_triangles(&result.mesh).len(), n as usize - 2);
        assert_eq!(result.mesh.vertex_count(), input.vertex_count());
    }
}

#[test]
fn r5_hole_fill_resource_failures_keep_the_stage_input_and_report_the_error() {
    use modeler3d_core::{MeshOperationLimits, RepairProfile, repair_mesh_with_options};
    let input = open_cylinder(64);
    for limits in [
        MeshOperationLimits {
            max_work_units: 1,
            ..MeshOperationLimits::default()
        },
        MeshOperationLimits {
            max_faces: input.face_count(),
            ..MeshOperationLimits::default()
        },
        MeshOperationLimits {
            max_memory_bytes: 32_000,
            ..MeshOperationLimits::default()
        },
    ] {
        let result = repair_mesh_with_options(&input, RepairProfile::Extended, limits)
            .expect("best effort report");
        let stage = result
            .report
            .stages
            .iter()
            .find(|s| s.name == "CloseAllHoles")
            .expect("fill stage");
        assert!(stage.error.is_some());
        assert_eq!(stage.before_triangles, stage.after_triangles);
        assert_eq!(result.report.after.boundary_edges, 64);
        result
            .mesh
            .validate_non_empty()
            .expect("unchanged valid structure");
    }
}

#[test]
fn r5_an_isolated_triangle_is_refused_unchanged_not_capped_with_its_back() {
    let input = Mesh::new(
        vec![[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        vec![[0, 1, 2]],
    )
    .expect("triangle");
    let original = input.clone();
    let result = repair_mesh(&input).expect("repair");
    assert_eq!(result.report.verdict, RepairVerdict::NoBetter);
    assert!(result.report.refused());
    assert!(!result.report.can_apply);
    assert_eq!(result.mesh.face_count(), 1);
    assert_eq!(input, original);
}

/// OF-74 cases: a sphere or box with holes, flipped faces, a duplicate face,
/// a vertex pushed through the surface and sometimes a crossing box. The
/// generator is deterministic; each listed seed was refused as a whole
/// before OF-74 because one stage made a defect worse.
fn of74_damaged(seed: u64) -> Mesh {
    struct Random(u64);
    impl Random {
        fn next(&mut self) -> f64 {
            self.0 = self
                .0
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (self.0 >> 11) as f64 / ((1_u64 << 53) as f64)
        }
        fn pick(&mut self, n: usize) -> usize {
            (self.next() * n as f64) as usize % n
        }
    }
    let mut r = Random(seed);
    let mut mesh = if r.next() < 0.5 {
        make_sphere(15.0, 2).expect("sphere")
    } else {
        modeler3d_core::make_box([20.0; 3]).expect("box")
    };
    for _ in 0..r.pick(4) {
        let index = r.pick(mesh.faces.len());
        mesh.faces.remove(index);
    }
    for _ in 0..r.pick(4) {
        let index = r.pick(mesh.faces.len());
        mesh.faces[index].swap(1, 2);
    }
    if r.next() < 0.5 {
        let face = mesh.faces[r.pick(mesh.faces.len())];
        mesh.faces.push(face);
    }
    if r.next() < 0.5 {
        let index = r.pick(mesh.vertices.len());
        let v = mesh.vertices[index];
        mesh.vertices[index] = [-v[0] * 0.6, -v[1] * 0.6 + 2.0, -v[2] * 0.6];
    }
    if r.next() < 0.4 {
        let mut other = modeler3d_core::make_box([6.0; 3]).expect("small box");
        let offset: [f32; 3] = std::array::from_fn(|_| (r.next() * 20.0 - 10.0) as f32);
        for vertex in &mut other.vertices {
            for axis in 0..3 {
                vertex[axis] += offset[axis];
            }
        }
        if r.next() < 0.5 {
            other.faces.remove(0);
        }
        mesh = Mesh::concatenate(&[mesh, other]).expect("two parts");
    }
    mesh
}

#[test]
fn of74_keeps_the_fixes_that_help_and_leaves_the_rest() {
    for seed in [19, 251, 617, 1628, 1774, 2602] {
        let input = of74_damaged(seed);
        let original = input.clone();
        let result = repair_mesh(&input).expect("repair");
        let report = &result.report;
        assert_eq!(input, original);
        assert!(report.can_apply, "seed {seed}: {report:?}");
        assert_eq!(report.verdict, RepairVerdict::Improved, "seed {seed}");
        assert!(!report.refused(), "seed {seed}");
        // Never worse: every defect count is the same or lower.
        let before = report.score_before;
        let after = report.score_after;
        for (old, new) in [
            (before.open_edges, after.open_edges),
            (before.non_manifold_edges, after.non_manifold_edges),
            (before.flipped_edges, after.flipped_edges),
            (before.degenerate_faces, after.degenerate_faces),
            (before.duplicate_faces, after.duplicate_faces),
            (before.inside_out_shells, after.inside_out_shells),
            (before.tiny_shells, after.tiny_shells),
            (before.shells, after.shells),
            (before.self_intersections, after.self_intersections),
        ] {
            assert!(new <= old, "seed {seed}: {before:?} -> {after:?}");
        }
        // The health report describes the mesh that is applied.
        assert_eq!(inspect_mesh(&result.mesh).expect("health"), report.after);
        // At least one stage was dropped, and the report says so.
        assert!(
            report.stages.iter().any(|stage| stage
                .error
                .as_deref()
                .is_some_and(|e| e.starts_with("skipped"))),
            "seed {seed}"
        );
        assert!(
            repair_summary(report).starts_with("Repaired"),
            "seed {seed}"
        );
    }
}

#[test]
fn of74_summary_says_what_was_fixed_and_what_could_not_be() {
    // Box with a shrunk but still open hole.
    let result = repair_mesh(&of74_damaged(19)).expect("repair");
    assert_eq!(
        repair_summary(&result.report),
        "Repaired: made the holes smaller. Couldn't fix: 1 hole."
    );
    let result = repair_mesh(&of74_damaged(2602)).expect("repair");
    assert_eq!(
        repair_summary(&result.report),
        "Repaired: turned flipped faces the right way, merged 2 overlapping parts into 1 \
         and removed 1 duplicate face. Couldn't fix: 1 hole."
    );
    // A fully fixed model has no "Couldn't fix" part.
    let result = repair_mesh(&of74_damaged(1774)).expect("repair");
    assert!(result.report.after.valid);
    assert!(!repair_summary(&result.report).contains("Couldn't fix"));
}

#[test]
fn of74_a_model_that_was_already_improved_is_repaired_exactly_as_before() {
    // The whole pipeline still runs first; the step-by-step pass only runs
    // when it would be refused, so no stage is reported as skipped.
    let input = open_cylinder(64);
    let result = repair_mesh(&input).expect("repair");
    assert!(result.report.can_apply);
    assert!(
        result
            .report
            .stages
            .iter()
            .all(|stage| stage.error.is_none())
    );
    assert!(!repair_summary(&result.report).contains("Couldn't fix"));
}
