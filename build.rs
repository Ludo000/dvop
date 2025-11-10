use std::env;
use std::fs;
use std::path::Path;

fn main() {
    // Tell Cargo to rerun this build script if resources change
    println!("cargo:rerun-if-changed=dvop.svg");
    println!("cargo:rerun-if-changed=resources");

    // Compile GResources
    glib_build_tools::compile_resources(
        &["resources"],
        "resources/resources.gresource.xml",
        "resources.gresource",
    );

    // Get the output directory where the binary will be placed
    let out_dir = env::var("OUT_DIR").unwrap();
    let profile = env::var("PROFILE").unwrap();

    // Determine the target directory based on profile
    let target_dir = Path::new(&out_dir)
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .expect("Could not find target directory");

    let binary_dir = target_dir.join(&profile);

    // Copy the logo to the binary directory
    let src = Path::new("dvop.svg");
    let dest = binary_dir.join("dvop.svg");

    if src.exists() {
        if let Err(e) = fs::copy(src, &dest) {
            eprintln!("Warning: Failed to copy dvop.svg: {}", e);
        } else {
            println!("cargo:warning=Copied dvop.svg to {}", dest.display());
        }
    }

    // For cargo install, we need to embed the icon data
    // So the application can extract it if needed
    println!("cargo:rustc-env=DVOP_ICON_PATH=dvop.svg");
}
