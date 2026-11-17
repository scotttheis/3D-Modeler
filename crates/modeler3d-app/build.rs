//! Build script: embeds the Windows icon resource (OF-29).
//!
//! Only Windows targets get a resource; other platforms take their icon from
//! the window (`artwork::load_native_icon`) and the package launchers.

fn main() {
    println!("cargo::rerun-if-env-changed=MODELER3D_RELEASE_VERSION");
    println!("cargo::rerun-if-changed=../../packaging/update-feed.json");
    let version = std::env::var("MODELER3D_RELEASE_VERSION").unwrap_or_else(|_| "0.1.0".into());
    if semver::Version::parse(&version).is_err() {
        panic!("MODELER3D_RELEASE_VERSION must be a semantic version");
    }
    println!("cargo::rustc-env=MODELER3D_RELEASE_VERSION={version}");
    let feed: serde_json::Value =
        serde_json::from_str(include_str!("../../packaging/update-feed.json"))
            .unwrap_or_else(|error| panic!("invalid update-feed.json: {error}"));
    let repo = feed["repository"].as_str().unwrap_or("");
    if !feed["repository"].is_null()
        && (repo.is_empty()
            || repo.split('/').count() != 2
            || repo.split('/').any(|part| {
                part.is_empty()
                    || part == "."
                    || part == ".."
                    || !part
                        .bytes()
                        .all(|c| c.is_ascii_alphanumeric() || b"-_.".contains(&c))
            }))
    {
        panic!("update-feed.json repository must be null or owner/repository");
    }
    println!("cargo::rustc-env=MODELER3D_UPDATE_REPOSITORY={repo}");
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
