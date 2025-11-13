use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    // Ensure rust-analyzer is installed
    println!("cargo:warning=Checking for rust-analyzer...");
    let status = Command::new("rustup")
        .args(&["component", "list", "--installed"])
        .output();
    
    if let Ok(output) = status {
        let installed = String::from_utf8_lossy(&output.stdout);
        if !installed.contains("rust-analyzer") {
            println!("cargo:warning=Installing rust-analyzer component...");
            let install_status = Command::new("rustup")
                .args(&["component", "add", "rust-analyzer"])
                .status();
            
            match install_status {
                Ok(status) if status.success() => {
                    println!("cargo:warning=rust-analyzer installed successfully");
                }
                Ok(_) => {
                    println!("cargo:warning=Failed to install rust-analyzer");
                }
                Err(e) => {
                    println!("cargo:warning=Error installing rust-analyzer: {}", e);
                }
            }
        } else {
            println!("cargo:warning=rust-analyzer is already installed");
        }
    }

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
