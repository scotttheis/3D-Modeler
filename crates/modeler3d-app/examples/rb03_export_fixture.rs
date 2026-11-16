//! Generate the deterministic multi-object scene used for RB-03 output checks.

use std::{env, error::Error, fs, path::PathBuf};

use modeler3d_app::export_scene;
use modeler3d_core::{Scene, SceneObject, Transform, make_box};

fn main() -> Result<(), Box<dyn Error>> {
    let output_directory = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/rb03-export-fixture"));
    fs::create_dir_all(&output_directory)?;

    let mut scene = Scene::new();
    scene.add_object(SceneObject::with_details(
        make_box([2.0, 4.0, 6.0])?,
        None,
        Some(String::from("Blue block")),
        Some([0.2, 0.4, 0.6, 0.8]),
        None,
    )?)?;
    scene.add_object(SceneObject::with_details(
        make_box([1.0, 3.0, 5.0])?,
        None,
        Some(String::from("Red block")),
        Some([0.9, 0.2, 0.1, 1.0]),
        None,
    )?)?;
    scene.set_transform(0, Transform::identity().translated([10.0, 20.0, 30.0])?)?;
    scene.set_transform(1, Transform::identity().translated([-4.0, 5.0, 1.0])?)?;
    scene.clear_selection();

    for extension in ["stl", "obj", "ply", "3mf", "gltf", "glb", "wrl"] {
        let path = output_directory.join(format!("scene.{extension}"));
        let report = export_scene(&scene, &path)?;
        println!(
            "{extension}: {} objects -> {}",
            report.objects_exported,
            path.display()
        );
    }

    Ok(())
}
