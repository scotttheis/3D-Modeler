//! Broken files must be refused with an error, never crash the app.
//!
//! The release build uses `panic = "abort"`, so a panic in any loader closes
//! 3D Modeler and loses unsaved work. Every loader that reads a file a person
//! picks gets valid bytes cut short, bytes changed, and size fields set huge.
//! The mutations are deterministic so a failure always reproduces.

use std::panic::{AssertUnwindSafe, catch_unwind};

use modeler3d_core::{
    MeshFormat, export_mesh, load_3mf_bytes, load_gltf_bytes, load_mesh_asset_bytes,
    load_project_bytes, make_box, save_project_bytes,
};

const FIXTURES: &[(&str, MeshFormat, &[u8])] = &[
    (
        "python_box.stl",
        MeshFormat::Stl,
        include_bytes!("fixtures/formats/python_box.stl"),
    ),
    (
        "python_box.obj",
        MeshFormat::Obj,
        include_bytes!("fixtures/formats/python_box.obj"),
    ),
    (
        "python_box.ply",
        MeshFormat::Ply,
        include_bytes!("fixtures/formats/python_box.ply"),
    ),
    (
        "colored_triangle.obj",
        MeshFormat::Obj,
        include_bytes!("fixtures/formats/colored_triangle.obj"),
    ),
    (
        "colored_triangle.ply",
        MeshFormat::Ply,
        include_bytes!("fixtures/formats/colored_triangle.ply"),
    ),
    (
        "python_multi_object_cm.3mf",
        MeshFormat::ThreeMf,
        include_bytes!("fixtures/imports/python_multi_object_cm.3mf"),
    ),
    (
        "python_multi_object.glb",
        MeshFormat::Glb,
        include_bytes!("fixtures/imports/python_multi_object.glb"),
    ),
    (
        "python_reference.wrl",
        MeshFormat::Wrl,
        include_bytes!("fixtures/wrl/python_reference.wrl"),
    ),
];

const PROJECTS: &[(&str, &[u8])] = &[
    (
        "python-current.3dp",
        include_bytes!("fixtures/projects/python-current.3dp"),
    ),
    (
        "python-legacy.3dp",
        include_bytes!("fixtures/projects/python-legacy.3dp"),
    ),
    (
        "rust-textured.3dp",
        include_bytes!("fixtures/projects/rust-textured.3dp"),
    ),
];

/// Small deterministic generator (no extra dependency).
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }

    fn below(&mut self, bound: usize) -> usize {
        if bound == 0 {
            0
        } else {
            (self.next() as usize) % bound
        }
    }
}

/// Returns cut-short copies and copies with changed bytes.
fn mutations(seed: &[u8], salt: u64) -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    let step = (seed.len() / 97).max(1);
    let mut cut = 0;
    while cut < seed.len() {
        out.push((format!("cut at {cut}"), seed[..cut].to_vec()));
        cut += step;
    }
    let mut rng = Lcg(0x5EED ^ salt);
    for round in 0..300 {
        let mut bytes = seed.to_vec();
        if bytes.is_empty() {
            break;
        }
        let at = rng.below(bytes.len());
        let label = match round % 4 {
            0 => {
                bytes[at] = rng.next() as u8;
                format!("byte {at} changed")
            }
            1 => {
                for byte in bytes.iter_mut().skip(at).take(4) {
                    *byte = 0xFF;
                }
                format!("4 bytes at {at} set to FF")
            }
            2 => {
                for (offset, byte) in [0xFF, 0xFF, 0xFF, 0x7F].iter().enumerate() {
                    if let Some(slot) = bytes.get_mut(at + offset) {
                        *slot = *byte;
                    }
                }
                format!("i32::MAX written at {at}")
            }
            _ => {
                let digits = b"99999999999";
                let end = (at + digits.len()).min(bytes.len());
                bytes.splice(at..end, digits.iter().copied());
                format!("huge number written at {at}")
            }
        };
        out.push((label, bytes));
    }
    out
}

fn survives(what: &str, failures: &mut Vec<String>, run: impl FnOnce()) {
    if catch_unwind(AssertUnwindSafe(run)).is_err() {
        failures.push(what.to_owned());
    }
}

fn exported_seeds() -> Vec<(String, MeshFormat, Vec<u8>)> {
    let Ok(mesh) = make_box([2.0, 3.0, 4.0]) else {
        return Vec::new();
    };
    [
        MeshFormat::Stl,
        MeshFormat::Obj,
        MeshFormat::Ply,
        MeshFormat::ThreeMf,
        MeshFormat::Gltf,
        MeshFormat::Glb,
        MeshFormat::Wrl,
    ]
    .into_iter()
    .filter_map(|format| {
        export_mesh(&mesh, format)
            .ok()
            .map(|bytes| (format!("exported {format:?}"), format, bytes))
    })
    .collect()
}

#[test]
fn broken_model_files_are_refused_without_crashing() {
    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let mut seeds: Vec<(String, MeshFormat, Vec<u8>)> = FIXTURES
        .iter()
        .map(|(name, format, bytes)| ((*name).to_owned(), *format, bytes.to_vec()))
        .collect();
    seeds.extend(exported_seeds());
    let mut failures = Vec::new();
    let mut tried = 0_usize;
    for (index, (name, format, seed)) in seeds.iter().enumerate() {
        for (label, bytes) in mutations(seed, index as u64) {
            tried += 1;
            let what = format!("{name} ({format:?}): {label}");
            survives(&what, &mut failures, || {
                let _ = load_mesh_asset_bytes(*format, &bytes);
            });
            match format {
                MeshFormat::ThreeMf => {
                    survives(&format!("{what} [load_3mf_bytes]"), &mut failures, || {
                        let _ = load_3mf_bytes(&bytes);
                    })
                }
                MeshFormat::Gltf | MeshFormat::Glb => {
                    survives(&format!("{what} [load_gltf_bytes]"), &mut failures, || {
                        let _ = load_gltf_bytes(&bytes);
                    });
                }
                _ => {}
            }
        }
    }
    std::panic::set_hook(previous_hook);
    assert!(tried > 3_000, "only {tried} broken files were tried");
    assert!(
        failures.is_empty(),
        "{} of {tried} broken files crashed a loader:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn broken_project_files_are_refused_without_crashing() {
    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let mut seeds: Vec<(String, Vec<u8>)> = PROJECTS
        .iter()
        .map(|(name, bytes)| ((*name).to_owned(), bytes.to_vec()))
        .collect();
    if let Ok(scene) = load_project_bytes(PROJECTS[0].1)
        && let Ok(bytes) = save_project_bytes(&scene)
    {
        seeds.push(("re-saved python-current.3dp".to_owned(), bytes));
    }
    let mut failures = Vec::new();
    let mut tried = 0_usize;
    for (index, (name, seed)) in seeds.iter().enumerate() {
        for (label, bytes) in mutations(seed, 1_000 + index as u64) {
            tried += 1;
            survives(&format!("{name}: {label}"), &mut failures, || {
                let _ = load_project_bytes(&bytes);
            });
        }
    }
    std::panic::set_hook(previous_hook);
    assert!(tried > 1_000, "only {tried} broken projects were tried");
    assert!(
        failures.is_empty(),
        "{} of {tried} broken projects crashed the loader:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
