//! # Dvop Build Script (`build.rs`)
//!
//! This file is a **Cargo build script** — it runs automatically before the main compilation
//! step whenever you run `cargo build`. Cargo build scripts are used for tasks that need to
//! happen at compile time, such as:
//!
//! 1. **Compiling GTK resources** (`.ui` files, icons, etc.) into a binary GResource bundle
//!    that gets embedded into the final executable. This means the app doesn't need external
//!    resource files at runtime.
//! 2. **Installing rust-analyzer** via `rustup` if it's not already present — this is needed
//!    by the Rust Diagnostics extension (see FEATURES.md: Feature #41).
//! 3. **Copying the application logo** (`dvop.svg`) to the build output directory so it's
//!    available alongside the compiled binary.
//!
//! Build scripts communicate with Cargo through `println!("cargo:...")` directives.
//! For example, `cargo:rerun-if-changed=resources` tells Cargo to re-run this script
//! only if files in the `resources/` directory change.

use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    // ── Step 1: Ensure rust-analyzer is installed ─────────────────────────────
    // rust-analyzer is the Language Server Protocol (LSP) server for Rust.
    // It provides real-time diagnostics (errors, warnings) as you type.
    // See FEATURES.md: Feature #41 — Rust Diagnostics Extension
    // See FEATURES.md: Feature #47 — LSP Integration (Rust Analyzer)
    let status = Command::new("rustup")
        .args(["component", "list", "--installed"])
        .output();
    
    if let Ok(output) = status {
        let installed = String::from_utf8_lossy(&output.stdout);
        if !installed.contains("rust-analyzer") {
            let _install_status = Command::new("rustup")
                .args(["component", "add", "rust-analyzer"])
                .status();
        }
    }

    // ── Step 2: Tell Cargo when to re-run this build script ──────────────────
    // `cargo:rerun-if-changed=<path>` tells Cargo to only re-run build.rs if these
    // files/directories have changed. Without this, build.rs would run on every build.
    println!("cargo:rerun-if-changed=dvop.svg");
    println!("cargo:rerun-if-changed=resources");

    // ── Step 3: Compile GTK GResources ───────────────────────────────────────
    // GResources are GTK's way of bundling UI definition files (.ui), icons, and other
    // assets into the compiled binary. The XML manifest (`resources.gresource.xml`)
    // lists all files to include. After compilation, the resources are available at
    // runtime via `gio::resources_lookup_data()` without needing external files.
    // See FEATURES.md: Feature #110 — GTK4 Template-Based UI
    glib_build_tools::compile_resources(
        &["resources"],
        "resources/resources.gresource.xml",
        "resources.gresource",
    );

    // ── Step 4: Copy the application logo to the build output directory ──────
    // The `OUT_DIR` environment variable is set by Cargo and points to a build-specific
    // output directory (e.g. `target/debug/build/dvop-<hash>/out`). We navigate up
    // from there to find the profile directory (e.g. `target/debug/`) where the
    // compiled binary lives, then copy the logo next to it.
    let out_dir = env::var("OUT_DIR").unwrap();
    let profile = env::var("PROFILE").unwrap(); // "debug" or "release"

    // Navigate from OUT_DIR up to the target directory:
    //   OUT_DIR = target/debug/build/dvop-<hash>/out
    //   parent x3 = target/debug/build/ -> target/debug/ -> target/
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

    // ── Step 5: Set environment variable for icon path ───────────────────────
    // `cargo:rustc-env=KEY=VALUE` makes an environment variable available at compile time
    // via `env!("DVOP_ICON_PATH")` in Rust code. The application can use this to locate
    // the icon even when installed via `cargo install`.
    println!("cargo:rustc-env=DVOP_ICON_PATH=dvop.svg");
}
