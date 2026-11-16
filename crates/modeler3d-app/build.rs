//! Build script: embeds the Windows icon resource (OF-29).
//!
//! Only Windows targets get a resource; other platforms take their icon from
//! the window (`artwork::load_native_icon`) and the package launchers.

fn main() {
    println!("cargo::rerun-if-changed=resources/modeler3d.rc");
    println!("cargo::rerun-if-changed=assets/original/Modeler3D.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    if let Err(error) =
        embed_resource::compile("resources/modeler3d.rc", embed_resource::NONE).manifest_required()
    {
        println!("cargo::error=could not embed the Windows icon resource: {error}");
    }
}
