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
    let kept = if result.report.can_apply {
        assert_eq!(result.report.after.boundary_edges, 0);
        &result.mesh
    } else {
        assert!(result.report.refused());
        &mesh
    };
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
        let mesh = Mesh::new(vertices, faces)?;
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
