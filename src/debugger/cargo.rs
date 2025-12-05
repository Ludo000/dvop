use std::path::Path;
use std::process::Command;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct CargoMetadata {
    packages: Vec<Package>,
    workspace_members: Vec<String>,
}

#[derive(Deserialize, Debug)]
struct Package {
    name: String,
    id: String,
    targets: Vec<Target>,
    manifest_path: String,
}

#[derive(Deserialize, Debug)]
struct Target {
    name: String,
    kind: Vec<String>,
    src_path: String,
}

#[derive(Debug, Clone)]
pub struct DebugTarget {
    pub name: String,
    pub bin_path: String, // Path to the source file or binary name
    pub kind: String,
}

pub fn detect_cargo_target(workspace_root: &Path) -> Option<DebugTarget> {
    let output = Command::new("cargo")
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .arg("--no-deps")
        .current_dir(workspace_root)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let metadata: CargoMetadata = serde_json::from_slice(&output.stdout).ok()?;

    // Find the primary package (workspace root or first member)
    // For simplicity, let's look for a package that has a 'bin' target
    
    for package in metadata.packages {
        // Check if this package is in the workspace members
        if metadata.workspace_members.contains(&package.id) {
            for target in package.targets {
                if target.kind.contains(&"bin".to_string()) {
                    return Some(DebugTarget {
                        name: target.name,
                        bin_path: target.src_path,
                        kind: "bin".to_string(),
                    });
                }
            }
        }
    }

    None
}
