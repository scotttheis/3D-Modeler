#![cfg(windows)]

use std::time::Instant;

use modeler3d_core::{
    Mesh, RepairEngine, RepairResult, RepairRouteControl, RepairVerdict, inspect_mesh, make_box,
    make_cylinder, make_sphere, repair_mesh_routed,
};
use windows::Graphics::Printing3D::{Printing3DMesh, Printing3DModel};

fn available() -> bool {
    let installed = std::env::var_os("SystemRoot").is_some_and(|root| {
        std::path::PathBuf::from(root)
            .join("System32/Windows.Graphics.Printing.3D.dll")
            .is_file()
    });
    if !installed || Printing3DModel::new().is_err() || Printing3DMesh::new().is_err() {
        eprintln!("Skipping Windows repair: public WinRT activation is unavailable on this host");
        return false;
    }
    true
}

fn routed(mesh: &Mesh, allow_windows: bool) -> RepairResult {
    repair_mesh_routed(
        mesh,
        RepairRouteControl {
            allow_windows,
            cancelled: &|| false,
            progress: None,
        },
    )
    .expect("repair")
}

fn volume(mesh: &Mesh) -> f64 {
    mesh.faces
        .iter()
        .map(|face| {
            let [a, b, c] = face.map(|index| mesh.vertices[index as usize].map(f64::from));
            (a[0] * (b[1] * c[2] - b[2] * c[1])
                + a[1] * (b[2] * c[0] - b[0] * c[2])
                + a[2] * (b[0] * c[1] - b[1] * c[0]))
                / 6.0
        })
        .sum()
}

fn max_move_mm(input: &Mesh, output: &Mesh) -> f64 {
    let extent = input.extents().into_iter().fold(0.0_f32, f32::max);
    let step = (f64::from(extent) / (input.vertex_count() as f64).cbrt()).max(0.01);
    let cell = |point: [f32; 3]| point.map(|v| (f64::from(v) / step).floor() as i64);
    let mut grid = std::collections::HashMap::<[i64; 3], Vec<[f32; 3]>>::new();
    for &point in &input.vertices {
        grid.entry(cell(point)).or_default().push(point);
    }
    output
        .vertices
        .iter()
        .map(|&point| {
            let key = cell(point);
            let squared = |other: &[f32; 3]| {
                (0..3)
                    .map(|i| {
                        let delta = f64::from(point[i]) - f64::from(other[i]);
                        delta * delta
                    })
                    .sum::<f64>()
            };
            let mut nearest = f64::INFINITY;
            for x in -1..=1 {
                for y in -1..=1 {
                    for z in -1..=1 {
                        if let Some(points) = grid.get(&[key[0] + x, key[1] + y, key[2] + z]) {
                            for other in points {
                                nearest = nearest.min(squared(other));
                            }
                        }
                    }
                }
            }
            // Search other occupied cells only when their bounding box can beat
            // the local candidate; this also handles new hole-fill points.
            for (key, points) in &grid {
                let lower = (0..3)
                    .map(|i| {
                        let lo = key[i] as f64 * step;
                        let hi = lo + step;
                        let p = f64::from(point[i]);
                        let distance = (lo - p).max(p - hi).max(0.0);
                        distance * distance
                    })
                    .sum::<f64>();
                if lower < nearest {
                    for other in points {
                        nearest = nearest.min(squared(other));
                    }
                }
            }
            nearest.sqrt()
        })
        .fold(0.0, f64::max)
}

fn assert_never_worse(result: &RepairResult, original: &Mesh) {
    if result.report.can_apply {
        assert_eq!(result.report.verdict, RepairVerdict::Improved);
        result
            .mesh
            .validate_non_empty()
            .expect("valid repaired geometry");
    } else {
        assert!(result.report.refused());
        // Adapters keep the original mesh on refusal.
        original.validate_non_empty().expect("unchanged original");
    }
}

#[test]
fn single_triangle_holes_use_windows_and_close() {
    if !available() {
        return;
    }
    let mut sphere = make_sphere(15.0, 3).expect("sphere");
    for index in [701, 401, 101, 1] {
        sphere.faces.remove(index);
    }
    let mut progress_calls = 0;
    let mut progress = |_, _| {
        progress_calls += 1;
    };
    let result = repair_mesh_routed(
        &sphere,
        RepairRouteControl {
            allow_windows: true,
            cancelled: &|| false,
            progress: Some(&mut progress),
        },
    )
    .expect("repair");
    assert_eq!(
        result.report.engine,
        RepairEngine::Windows,
        "{:?}",
        result.report
    );
    assert!(result.report.can_apply);
    assert_eq!(result.report.after.boundary_edges, 0);
    assert_eq!(result.report.stages.len(), 1);
    assert_eq!(result.report.stages[0].name, "Windows repair engine");
    assert!(progress_calls > 0);
}

#[test]
fn flipped_sphere_is_fixed_with_volume_preserved() {
    if !available() {
        return;
    }
    let mut sphere = make_sphere(15.0, 3).expect("sphere");
    let original_volume = volume(&sphere);
    for face in sphere.faces.iter_mut().step_by(5) {
        face.swap(1, 2);
    }
    let result = routed(&sphere, true);
    assert_eq!(
        result.report.engine,
        RepairEngine::Windows,
        "{:?}",
        result.report.engine_note
    );
    assert!(result.report.can_apply);
    assert!(result.report.after.valid);
    assert!((volume(&result.mesh) / original_volume - 1.0).abs() < 0.01);
}

#[test]
fn duplicate_face_and_sealed_sliver_never_get_worse() {
    if !available() {
        return;
    }
    let mut mesh = make_box([10.0; 3]).expect("box");
    let [a, b, c] = mesh.faces.remove(0);
    let pa = mesh.vertices[a as usize];
    let pb = mesh.vertices[b as usize];
    let pc = mesh.vertices[c as usize];
    let point =
        std::array::from_fn(|axis| (pa[axis] + pb[axis]) * 0.5 + (pc[axis] - pa[axis]) * 1.0e-6);
    let extra = mesh.vertex_count() as u32;
    mesh.vertices.push(point);
    mesh.faces
        .extend([[a, b, extra], [b, c, extra], [c, a, extra]]);
    mesh.faces.push(mesh.faces[0]);
    let result = routed(&mesh, true);
    assert_never_worse(&result, &mesh);
    if result.report.engine != RepairEngine::Windows {
        assert!(result.report.refused(), "{:?}", result.report);
        assert!(result.report.engine_note.is_some());
    }
}

#[test]
fn hollow_model_uses_native_and_keeps_its_cavity() {
    if !available() {
        return;
    }
    let mut outer = make_box([20.0; 3]).expect("outer");
    outer.faces.pop();
    let mut inner = make_box([10.0; 3]).expect("inner");
    for face in &mut inner.faces {
        face.swap(1, 2);
    }
    let mesh = Mesh::concatenate(&[outer, inner]).expect("hollow model");
    assert_eq!(inspect_mesh(&mesh).expect("health").cavity_shells, 1);
    let result = routed(&mesh, true);
    assert_eq!(result.report.engine, RepairEngine::Native);
    assert!(
        result
            .report
            .engine_note
            .as_deref()
            .is_some_and(|note| note.contains("hollow inside"))
    );
    assert!(result.report.can_apply, "{:?}", result.report);
    assert_eq!(result.report.after.boundary_edges, 0);
    let kept = &result.mesh;
    assert_eq!(inspect_mesh(kept).expect("kept cavity").cavity_shells, 1);
}

#[test]
fn jittered_unwelded_cylinder_never_collapses() {
    if !available() {
        return;
    }
    let cylinder = make_cylinder(15.0, 20.0, 64).expect("cylinder");
    let mut vertices = Vec::new();
    let mut faces = Vec::new();
    let mut random = 7_u32;
    for face in &cylinder.faces {
        let offset = vertices.len() as u32;
        for index in face {
            let mut vertex = cylinder.vertices[*index as usize];
            for coordinate in &mut vertex {
                random = random.wrapping_mul(1664525).wrapping_add(1013904223);
                *coordinate += (random as f64 / u32::MAX as f64 * 0.02 - 0.01) as f32;
            }
            vertices.push(vertex);
        }
        faces.push([offset, offset + 1, offset + 2]);
    }
    let mesh = Mesh::new(vertices, faces).expect("soup");
    let result = routed(&mesh, true);
    assert_never_worse(&result, &mesh);
    let applied = if result.report.can_apply {
        &result.mesh
    } else {
        &mesh
    };
    for (after, before) in applied.extents().into_iter().zip(mesh.extents()) {
        assert!(after >= before - 0.2, "{:?}", result.report);
    }
    assert!(
        volume(applied).abs() > volume(&cylinder) * 0.9,
        "{:?}",
        result.report
    );
    if result.report.engine == RepairEngine::Windows {
        // Gaps wider than the engine's weld radius legitimately stay as
        // separate closed pieces; the repair must still seal and merge.
        assert_eq!(result.report.after.boundary_edges, 0);
        assert!(result.report.score_after.shells < result.report.score_before.shells);
    } else {
        assert!(result.report.engine_note.is_some());
    }
}

#[test]
fn disabled_windows_and_native_override_use_native() {
    // Spawn an isolated child instead of mutating process-wide environment
    // variables while Rust's test runner is using other threads.
    if std::env::var("MODELER3D_REPAIR_ENGINE").as_deref() != Ok("native") {
        let status = std::process::Command::new(std::env::current_exe().expect("test executable"))
            .args([
                "--exact",
                "disabled_windows_and_native_override_use_native",
                "--nocapture",
            ])
            .env("MODELER3D_REPAIR_ENGINE", "native")
            .status()
            .expect("native override child");
        assert!(status.success());
        return;
    }
    let mut mesh = make_box([10.0; 3]).expect("box");
    mesh.faces.pop();
    for allow in [false, true] {
        let result = routed(&mesh, allow);
        assert_eq!(result.report.engine, RepairEngine::Native);
        assert!(result.report.engine_note.is_none());
    }
}

#[test]
fn cancelling_the_windows_wait_returns_no_candidate() {
    if !available() {
        return;
    }
    let mut mesh = make_sphere(15.0, 3).expect("sphere");
    mesh.faces.pop();
    let cancelled = std::cell::Cell::new(false);
    let mut progress = |_, _| cancelled.set(true);
    let result = repair_mesh_routed(
        &mesh,
        RepairRouteControl {
            allow_windows: true,
            cancelled: &|| cancelled.get(),
            progress: Some(&mut progress),
        },
    );
    assert!(
        matches!(result, Err(modeler3d_core::CoreError::MeshOperation(message)) if message == "Repair cancelled")
    );
}

#[test]
#[ignore = "set MODELER3D_REPAIR_FIXTURES to the generated fixture directory"]
fn repair_fixture_dir() -> Result<(), Box<dyn std::error::Error>> {
    if !available() {
        return Ok(());
    }
    let directory = std::path::PathBuf::from(
        std::env::var_os("MODELER3D_REPAIR_FIXTURES")
            .ok_or("MODELER3D_REPAIR_FIXTURES is not set")?,
    );
    let mut paths = std::fs::read_dir(directory)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?;
    paths.sort();
    for path in paths
        .iter()
        .filter(|path| path.extension().is_some_and(|extension| extension == "v"))
    {
        let mesh = read_fixture(path)?;
        let start = Instant::now();
        let result = routed(&mesh, true);
        println!(
            "{} engine={} verdict={:?} note={:?} score={:?}/{:?} volume={:.6}/{:.6} seconds={:.3} open_edges={} non_manifold_edges={} shells={} max_move_mm={:.6} faces={} hash={:016x}",
            path.file_stem().unwrap_or_default().to_string_lossy(),
            result.report.engine.as_str(),
            result.report.verdict,
            result.report.engine_note,
            result.report.score_before,
            result.report.score_after,
            volume(&mesh),
            volume(&result.mesh),
            start.elapsed().as_secs_f64(),
            result.report.score_after.open_edges,
            result.report.score_after.non_manifold_edges,
            result.report.score_after.shells,
            max_move_mm(&mesh, &result.mesh),
            result.mesh.face_count(),
            mesh_hash(&result.mesh)
        );
    }
    Ok(())
}

fn read_fixture(path: &std::path::Path) -> Result<Mesh, Box<dyn std::error::Error>> {
    let vertex_bytes = std::fs::read(path)?;
    let face_bytes = std::fs::read(path.with_extension("t"))?;
    if vertex_bytes.len() % 24 != 0 || face_bytes.len() % 12 != 0 {
        return Err(format!("Invalid fixture record length: {}", path.display()).into());
    }
    let vertices = vertex_bytes
        .as_chunks::<24>()
        .0
        .iter()
        .map(|record| {
            let mut point = [0.0; 3];
            for (coordinate, bytes) in point.iter_mut().zip(record.as_chunks::<8>().0.iter()) {
                *coordinate = f64::from_le_bytes(*bytes) as f32;
            }
            point
        })
        .collect();
    let faces = face_bytes
        .as_chunks::<12>()
        .0
        .iter()
        .map(|record| {
            let mut face = [0; 3];
            for (index, bytes) in face.iter_mut().zip(record.as_chunks::<4>().0.iter()) {
                *index = u32::from_le_bytes(*bytes);
            }
            face
        })
        .collect();
    Ok(Mesh::new(vertices, faces)?)
}

#[test]
#[ignore = "Release health timing; set MODELER3D_REPAIR_FIXTURES"]
fn inspect_mesh_release_timings() -> Result<(), Box<dyn std::error::Error>> {
    let directory = std::path::PathBuf::from(
        std::env::var_os("MODELER3D_REPAIR_FIXTURES").ok_or("missing fixture directory")?,
    );
    let book = read_fixture(&directory.join("f1_book.v"))?;
    // An exact 1M-face latitude sphere avoids rounding up the control size.
    let sphere = latitude_sphere(1000, 501, 15.0);
    assert_eq!(sphere.face_count(), 1_000_000);
    for (name, mesh) in [("book", &book), ("sphere_1M", &sphere)] {
        let mut times = Vec::new();
        for _ in 0..5 {
            let start = Instant::now();
            let health = std::hint::black_box(inspect_mesh(std::hint::black_box(mesh))?);
            times.push(start.elapsed().as_secs_f64());
            println!(
                "inspect {name} faces={} seconds={:.6} intersections={:?}",
                mesh.face_count(),
                times[times.len() - 1],
                health.self_intersections
            );
        }
        times.sort_by(f64::total_cmp);
        println!("inspect {name} median_seconds={:.6}", times[2]);
    }
    Ok(())
}

fn latitude_sphere(nu: u32, nv: u32, radius: f64) -> Mesh {
    let mut vertices = vec![[0.0, 0.0, radius as f32]];
    for i in 1..nv {
        let theta = std::f64::consts::PI * f64::from(i) / f64::from(nv);
        for j in 0..nu {
            let phi = std::f64::consts::TAU * f64::from(j) / f64::from(nu);
            vertices.push([
                (radius * theta.sin() * phi.cos()) as f32,
                (radius * theta.sin() * phi.sin()) as f32,
                (radius * theta.cos()) as f32,
            ]);
        }
    }
    let south = vertices.len() as u32;
    vertices.push([0.0, 0.0, -radius as f32]);
    let idx = |i, j| 1 + (i - 1) * nu + j % nu;
    let mut faces = Vec::new();
    for j in 0..nu {
        faces.push([0, idx(1, j), idx(1, j + 1)]);
    }
    for i in 1..nv - 1 {
        for j in 0..nu {
            let [a, b, c, d] = [idx(i, j), idx(i, j + 1), idx(i + 1, j), idx(i + 1, j + 1)];
            faces.extend([[a, c, d], [a, d, b]]);
        }
    }
    for j in 0..nu {
        faces.push([south, idx(nv - 1, j + 1), idx(nv - 1, j)]);
    }
    Mesh::new(vertices, faces).expect("latitude sphere")
}

/// FNV-1a over vertex bits and face indices: equal output, equal hash.
fn mesh_hash(mesh: &Mesh) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    let mut feed = |value: u32| {
        for byte in value.to_le_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0100_0000_01b3);
        }
    };
    for vertex in &mesh.vertices {
        for coordinate in vertex {
            feed(coordinate.to_bits());
        }
    }
    for face in &mesh.faces {
        for index in face {
            feed(*index);
        }
    }
    hash
}

// Code-generated versions of genfx.py f2-f10. The random damage uses a
// local seeded generator so these tests need neither Python nor fixture files.
fn fixture_box(lo: [f32; 3], hi: [f32; 3]) -> Mesh {
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
    .expect("fixture box")
}

fn fixture_cylinder(top: bool) -> Mesh {
    let n = 64_u32;
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
        faces.extend([[i, j, n + j], [i, n + j, n + i], [2 * n, j, i]]);
        if top {
            faces.push([2 * n + 1, n + i, n + j]);
        }
    }
    Mesh::new(vertices, faces).expect("fixture cylinder")
}

fn parity_fixtures() -> Vec<(&'static str, Mesh)> {
    let mut state = 7_u64;
    let mut random = || {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        (state >> 32) as f64 / f64::from(u32::MAX)
    };
    let sphere = latitude_sphere(64, 32, 15.0);
    let mut holey = sphere.clone();
    holey.faces = holey
        .faces
        .into_iter()
        .enumerate()
        .filter_map(|(id, face)| (id % 33 != 7).then_some(face))
        .collect();
    let mut flipped = sphere.clone();
    for face in &mut flipped.faces {
        if random() < 0.2 {
            face.swap(1, 2);
        }
    }
    let cylinder = fixture_cylinder(true);
    let mut vertices = Vec::new();
    let mut faces = Vec::new();
    for face in &cylinder.faces {
        let offset = vertices.len() as u32;
        let mut corners = face.map(|v| {
            cylinder.vertices[v as usize].map(|x| x + (random() * 0.0006 - 0.0003) as f32)
        });
        let positive_side = corners.iter().map(|p| p[1]).sum::<f32>() > 0.0;
        for corner in &mut corners {
            if positive_side && corner[1].atan2(corner[0]).abs() <= 0.001 {
                corner[1] += 0.01;
            }
        }
        vertices.extend(corners);
        faces.push([offset, offset + 1, offset + 2]);
    }
    let soup = Mesh::new(vertices, faces).expect("soup cylinder");
    let overlap = Mesh::concatenate(&[
        fixture_box([0.0; 3], [20.0; 3]),
        fixture_box([10.0; 3], [30.0; 3]),
    ])
    .expect("overlap");
    let mut non_manifold = Mesh::concatenate(&[
        fixture_box([0.0; 3], [10.0; 3]),
        fixture_box([10.0, 10.0, 0.0], [20.0, 20.0, 10.0]),
    ])
    .expect("edge touching");
    for face in &mut non_manifold.faces {
        for vertex in face {
            if *vertex == 8 {
                *vertex = 2;
            } else if *vertex == 12 {
                *vertex = 6;
            }
        }
    }
    non_manifold.vertices.push([5.0, 5.0, 15.0]);
    non_manifold.faces.push([4, 5, 16]);
    let mut duplicates = fixture_box([0.0; 3], [10.0; 3]);
    duplicates.faces.extend_from_within(..2);
    let mut inward = sphere;
    for face in &mut inward.faces {
        face.swap(1, 2);
    }
    let mut sliver = fixture_box([0.0; 3], [10.0; 3]);
    sliver.vertices.push([5.0, 0.00005, 10.0]);
    sliver.faces.retain(|face| *face != [4, 5, 6]);
    sliver.faces.extend([[4, 8, 6], [8, 5, 6], [4, 5, 8]]);
    vec![
        ("f2", fixture_cylinder(false)),
        ("f3", holey),
        ("f4", flipped),
        ("f5", soup),
        ("f6", overlap),
        ("f7", non_manifold),
        ("f8", duplicates),
        ("f9", inward),
        ("f10", sliver),
    ]
}

#[test]
fn both_engines_pass_the_same_generated_f2_through_f10() {
    if !available() {
        return;
    }
    let mut shell_mismatches = Vec::new();
    for (name, mesh) in parity_fixtures() {
        let windows = routed(&mesh, true);
        let native = routed(&mesh, false);
        if name != "f10" {
            assert_eq!(
                windows.report.engine,
                RepairEngine::Windows,
                "{name}: {:?}",
                windows.report
            );
            assert!(windows.report.can_apply, "{name}: {:?}", windows.report);
            assert!(native.report.can_apply, "{name}: {:?}", native.report);
        }
        for result in [&windows, &native] {
            result.mesh.validate_non_empty().expect("valid result");
            let health = &result.report.after;
            assert_eq!(health.boundary_edges, 0, "{name}: {health:?}");
            assert_eq!(health.non_manifold_edges, 0, "{name}: {health:?}");
            assert_eq!(health.flipped_edges, 0, "{name}: {health:?}");
            assert_eq!(health.self_intersections, Some(0), "{name}: {health:?}");
        }
        // f7 is two cubes touching along one edge. Windows joins them into
        // one part; native keeps two parts by design (R4 decision: parts that
        // only touch are not merged). Everything else must match.
        if name != "f7" && windows.report.after.body_count != native.report.after.body_count {
            shell_mismatches.push((
                name,
                windows.report.after.body_count,
                native.report.after.body_count,
            ));
        }
        let (wv, nv) = (volume(&windows.mesh), volume(&native.mesh));
        assert!((wv / nv - 1.0).abs() < 0.01, "{name}: {wv}/{nv}");
        println!(
            "parity {name} windows={:?} native={:?} shells={}/{} volume={wv:.6}/{nv:.6} self_intersections={:?}/{:?}",
            windows.report.verdict,
            native.report.verdict,
            windows.report.after.body_count,
            native.report.after.body_count,
            windows.report.after.self_intersections,
            native.report.after.self_intersections
        );
    }
    assert!(
        shell_mismatches.is_empty(),
        "Windows/native shell counts differ: {shell_mismatches:?}"
    );
}
