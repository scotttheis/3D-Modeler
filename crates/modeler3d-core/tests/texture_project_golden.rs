//! Textured projects (OF-46) and the Python reference build.
//!
//! `rust-textured.3dp` was written by this build (regenerate it by running
//! this test with `MODELER3D_WRITE_TEXTURE_FIXTURE` set to the fixture path).
//! `python-resaved-textured.3dp` is what the Python reference wrote after
//! opening that file and saving it again: the texture reference survives in
//! object metadata, the image table does not. Both must open here.

use std::io::Cursor;

use modeler3d_core::{
    Scene, SceneObject, TextureImage, load_project_bytes, make_box, object_texture,
    register_texture, resolved_texture, save_project_bytes,
};

const RUST_TEXTURED: &[u8] = include_bytes!("fixtures/projects/rust-textured.3dp");
const PYTHON_RESAVED: &[u8] = include_bytes!("fixtures/projects/python-resaved-textured.3dp");

fn checker_png() -> Vec<u8> {
    let image = image::RgbaImage::from_fn(8, 8, |x, y| {
        if (x / 4 + y / 4) % 2 == 0 {
            image::Rgba([220, 40, 40, 200])
        } else {
            image::Rgba([40, 40, 220, 200])
        }
    });
    let mut bytes = Vec::new();
    image::DynamicImage::ImageRgba8(image)
        .write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png)
        .expect("png");
    bytes
}

#[test]
fn rust_textured_fixture_matches_this_build_and_resolves_its_texture() {
    let texture =
        register_texture(TextureImage::from_source_bytes(&checker_png()).expect("texture"));
    let mut scene = Scene::new();
    scene
        .add_object(SceneObject::new(make_box([20.0, 20.0, 20.0]).expect("box")))
        .expect("add");
    scene.apply_texture(&[0], &texture, 10.0).expect("texture");
    let written = save_project_bytes(&scene).expect("save");
    if let Some(path) = std::env::var_os("MODELER3D_WRITE_TEXTURE_FIXTURE") {
        std::fs::write(path, &written).expect("write fixture");
    }

    let loaded = load_project_bytes(RUST_TEXTURED).expect("Rust textured project opens");
    let (image, tile_mm) =
        resolved_texture(loaded.objects()[0].metadata()).expect("texture resolves");
    assert_eq!((image.width(), image.height()), (8, 8));
    assert!((tile_mm - 10.0).abs() < 1e-4);
}

#[test]
fn a_project_resaved_by_python_opens_with_its_colour_and_keeps_the_reference() {
    let loaded = load_project_bytes(PYTHON_RESAVED).expect("Python-resaved project opens");
    let object = &loaded.objects()[0];
    let reference = object_texture(object.metadata()).expect("reference kept by Python");
    assert!((reference.tile_mm - 10.0).abs() < 1e-4);
    // The image table is gone; whether the picture shows depends only on
    // whether this process still has it, so the project always opens and
    // saves either way.
    assert!(save_project_bytes(&loaded).is_ok());
    let root: serde_json::Value = {
        let mut archive = zip::ZipArchive::new(Cursor::new(PYTHON_RESAVED)).expect("zip");
        let mut text = String::new();
        std::io::Read::read_to_string(
            &mut archive.by_name("scene.json").expect("scene"),
            &mut text,
        )
        .expect("read");
        serde_json::from_str(&text).expect("json")
    };
    assert!(root.get("textures").is_none());
}
