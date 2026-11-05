//! Deterministic paired RB-04 scene, with the same dimensions as the Python run.
use builder3d_core::{Scene, SceneObject, Transform, make_box, save_project};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut scene = Scene::new();
    for (name, size) in [("Box A", [20., 30., 40.]), ("Box B", [10., 20., 30.])] {
        scene.add_object(SceneObject::with_details(
            make_box(size)?,
            None,
            Some(name.into()),
            None,
            None,
        )?)?;
    }
    scene.set_transform(1, Transform::identity().translated([35., 10., 0.])?)?;
    scene.set_selection([0]);
    save_project(
        &scene,
        std::path::Path::new(
            "evidence/rebuild/rb-04-scene-and-precise-transforms/paired-scene.3dp",
        ),
    )?;
    Ok(())
}
