//! Exact pre-settle scenes corresponding to the Python transform oracle.
use modeler3d_core::{Scene, SceneObject, Transform, make_box, save_project};
use std::{error::Error, path::Path};
fn main() -> Result<(), Box<dyn Error>> {
    let cases: serde_json::Value = serde_json::from_str(include_str!(
        "../../../evidence/rebuild/rb-04-scene-and-precise-transforms/settle-oracle.json"
    ))?;
    for case in cases.as_array().ok_or("cases")? {
        let transforms = case["before"].as_array().ok_or("transforms")?;
        let mut scene = Scene::new();
        for (index, rows) in transforms.iter().enumerate() {
            scene.add_object(SceneObject::with_details(
                make_box([20., 30., 40.])?,
                None,
                Some(format!("box {index}")),
                None,
                None,
            )?)?;
            let rows: [[f64; 4]; 4] = serde_json::from_value(rows.clone())?;
            scene.set_transform(index, Transform::from_f64_rows(rows)?)?;
        }
        let path = format!(
            "evidence/rebuild/rb-04-scene-and-precise-transforms/settle-{}-before.3dp",
            transforms.len()
        );
        save_project(&scene, Path::new(&path))?;
    }
    Ok(())
}
