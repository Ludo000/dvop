// Rust project detection for the debugger
// Detects Rust projects and binaries based on folder structure

use std::path::{Path, PathBuf};
use std::process::Command;

/// Information about a detected Rust project
#[derive(Debug, Clone)]
pub struct RustProject {
    pub root: PathBuf,
    pub cargo_toml: PathBuf,
    pub project_name: String,
    pub target_dir: PathBuf,
    pub binaries: Vec<RustBinary>,
    pub is_workspace: bool,
    pub workspace_members: Vec<String>,
}

/// Information about a Rust binary target
#[derive(Debug, Clone)]
pub struct RustBinary {
    pub name: String,
    pub path: PathBuf,
    pub binary_type: BinaryType,
    pub is_built: bool,
}

/// Type of Rust binary
#[derive(Debug, Clone, PartialEq)]
pub enum BinaryType {
    Binary,      // [[bin]] or src/main.rs
    Example,     // examples/
    Test,        // tests/
    Benchmark,   // benches/
}

impl std::fmt::Display for BinaryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BinaryType::Binary => write!(f, "Binary"),
            BinaryType::Example => write!(f, "Example"),
            BinaryType::Test => write!(f, "Test"),
            BinaryType::Benchmark => write!(f, "Benchmark"),
        }
    }
}

/// Detect if a path is within a Rust project
/// Returns the Rust project root if found
pub fn find_rust_project_root(path: &Path) -> Option<PathBuf> {
    let mut current = if path.is_file() {
        path.parent()?.to_path_buf()
    } else {
        path.to_path_buf()
    };

    loop {
        let cargo_toml = current.join("Cargo.toml");
        if cargo_toml.exists() {
            return Some(current);
        }

        // Move up to parent directory
        if let Some(parent) = current.parent() {
            current = parent.to_path_buf();
        } else {
            break;
        }
    }

    None
}

/// Check if a directory is a Rust project
pub fn is_rust_project(path: &Path) -> bool {
    find_rust_project_root(path).is_some()
}

/// Get detailed information about a Rust project
pub fn get_rust_project_info(project_root: &Path) -> Option<RustProject> {
    let cargo_toml_path = project_root.join("Cargo.toml");
    if !cargo_toml_path.exists() {
        return None;
    }

    // Parse Cargo.toml
    let cargo_content = std::fs::read_to_string(&cargo_toml_path).ok()?;
    let cargo_doc: toml::Value = cargo_content.parse().ok()?;

    // Get project name
    let project_name = cargo_doc
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .unwrap_or("unknown")
        .to_string();

    // Detect workspace
    let is_workspace = cargo_doc.get("workspace").is_some();
    let workspace_members = if is_workspace {
        cargo_doc
            .get("workspace")
            .and_then(|w| w.get("members"))
            .and_then(|m| m.as_array())
            .map(|members| {
                members
                    .iter()
                    .filter_map(|m| m.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    // Get target directory
    let target_dir = project_root.join("target");

    // Find binaries
    let binaries = find_project_binaries(project_root, &project_name, &target_dir);

    Some(RustProject {
        root: project_root.to_path_buf(),
        cargo_toml: cargo_toml_path,
        project_name,
        target_dir,
        binaries,
        is_workspace,
        workspace_members,
    })
}

/// Find all binary targets in a Rust project
fn find_project_binaries(project_root: &Path, project_name: &str, target_dir: &Path) -> Vec<RustBinary> {
    let mut binaries = Vec::new();

    // Check for main binary in debug directory
    let debug_dir = target_dir.join("debug");
    let release_dir = target_dir.join("release");

    // Main binary
    let main_binary_name = project_name.replace('-', "_");
    let debug_binary = debug_dir.join(&main_binary_name);
    let release_binary = release_dir.join(&main_binary_name);

    // Check debug binary
    if debug_binary.exists() {
        binaries.push(RustBinary {
            name: project_name.to_string(),
            path: debug_binary,
            binary_type: BinaryType::Binary,
            is_built: true,
        });
    } else {
        // Add placeholder for unbuilt binary
        binaries.push(RustBinary {
            name: project_name.to_string(),
            path: debug_dir.join(&main_binary_name),
            binary_type: BinaryType::Binary,
            is_built: false,
        });
    }

    // Check release binary
    if release_binary.exists() {
        binaries.push(RustBinary {
            name: format!("{} (release)", project_name),
            path: release_binary,
            binary_type: BinaryType::Binary,
            is_built: true,
        });
    }

    // Look for additional binary targets from Cargo.toml
    if let Ok(cargo_content) = std::fs::read_to_string(project_root.join("Cargo.toml")) {
        if let Ok(cargo_doc) = cargo_content.parse::<toml::Value>() {
            // Check for [[bin]] entries
            if let Some(bins) = cargo_doc.get("bin").and_then(|b| b.as_array()) {
                for bin in bins {
                    if let Some(name) = bin.get("name").and_then(|n| n.as_str()) {
                        if name != project_name {
                            let bin_name = name.replace('-', "_");
                            let debug_path = debug_dir.join(&bin_name);
                            
                            binaries.push(RustBinary {
                                name: name.to_string(),
                                path: debug_path.clone(),
                                binary_type: BinaryType::Binary,
                                is_built: debug_path.exists(),
                            });
                        }
                    }
                }
            }
        }
    }

    // Look for examples
    let examples_dir = project_root.join("examples");
    if examples_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&examples_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "rs") {
                    if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                        let example_binary = debug_dir.join("examples").join(name);
                        
                        binaries.push(RustBinary {
                            name: name.to_string(),
                            path: example_binary.clone(),
                            binary_type: BinaryType::Example,
                            is_built: example_binary.exists(),
                        });
                    }
                }
            }
        }
    }

    binaries
}

/// Build a Rust project in debug mode
pub fn build_project(project_root: &Path) -> Result<String, String> {
    let output = Command::new("cargo")
        .arg("build")
        .current_dir(project_root)
        .output()
        .map_err(|e| format!("Failed to run cargo build: {}", e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

/// Build a specific binary target
pub fn build_binary(project_root: &Path, binary_name: &str, release: bool) -> Result<String, String> {
    let mut cmd = Command::new("cargo");
    cmd.arg("build").arg("--bin").arg(binary_name);
    
    if release {
        cmd.arg("--release");
    }
    
    let output = cmd
        .current_dir(project_root)
        .output()
        .map_err(|e| format!("Failed to run cargo build: {}", e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

/// Build an example
pub fn build_example(project_root: &Path, example_name: &str) -> Result<String, String> {
    let output = Command::new("cargo")
        .arg("build")
        .arg("--example")
        .arg(example_name)
        .current_dir(project_root)
        .output()
        .map_err(|e| format!("Failed to run cargo build: {}", e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

/// Get the Rust toolchain version
pub fn get_rust_version() -> Option<String> {
    Command::new("rustc")
        .arg("--version")
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
            } else {
                None
            }
        })
}

/// Check if Cargo is available
pub fn is_cargo_available() -> bool {
    Command::new("cargo")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    fn create_test_rust_project() -> TempDir {
        let dir = TempDir::new().unwrap();
        
        // Create Cargo.toml
        let cargo_toml = r#"
[package]
name = "test-project"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "test-project"
path = "src/main.rs"
"#;
        fs::write(dir.path().join("Cargo.toml"), cargo_toml).unwrap();
        
        // Create src directory
        fs::create_dir(dir.path().join("src")).unwrap();
        fs::write(dir.path().join("src/main.rs"), "fn main() {}").unwrap();
        
        dir
    }

    fn create_nested_rust_project() -> TempDir {
        let dir = TempDir::new().unwrap();
        
        // Create nested project structure
        let nested = dir.path().join("subdir/nested");
        fs::create_dir_all(&nested).unwrap();
        
        // Create Cargo.toml in nested directory
        let cargo_toml = r#"
[package]
name = "nested-project"
version = "0.1.0"
edition = "2021"
"#;
        fs::write(nested.join("Cargo.toml"), cargo_toml).unwrap();
        
        // Create src directory
        fs::create_dir(nested.join("src")).unwrap();
        fs::write(nested.join("src/main.rs"), "fn main() {}").unwrap();
        
        dir
    }

    #[test]
    fn test_find_rust_project_root() {
        let project = create_test_rust_project();
        
        // Should find root from project directory
        let result = find_rust_project_root(project.path());
        assert!(result.is_some());
        assert_eq!(result.unwrap(), project.path());
        
        // Should find root from subdirectory
        let src_dir = project.path().join("src");
        let result = find_rust_project_root(&src_dir);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), project.path());
    }

    #[test]
    fn test_find_rust_project_root_from_file() {
        let project = create_test_rust_project();
        
        // Should find root from file path
        let main_rs = project.path().join("src/main.rs");
        let result = find_rust_project_root(&main_rs);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), project.path());
    }

    #[test]
    fn test_nested_project_detection() {
        let dir = create_nested_rust_project();
        
        // Create a deeply nested file
        let deep_path = dir.path().join("subdir/nested/src");
        fs::create_dir_all(&deep_path).unwrap();
        let file_path = deep_path.join("lib.rs");
        fs::write(&file_path, "// lib").unwrap();
        
        // Should find project root from deep nesting
        let result = find_rust_project_root(&file_path);
        assert!(result.is_some());
        assert!(result.unwrap().ends_with("nested"));
    }

    #[test]
    fn test_is_rust_project() {
        let project = create_test_rust_project();
        
        assert!(is_rust_project(project.path()));
        assert!(is_rust_project(&project.path().join("src")));
        
        // Non-rust directory
        let temp = TempDir::new().unwrap();
        assert!(!is_rust_project(temp.path()));
    }

    #[test]
    fn test_get_rust_project_info() {
        let project = create_test_rust_project();
        
        let info = get_rust_project_info(project.path());
        assert!(info.is_some());
        
        let info = info.unwrap();
        assert_eq!(info.project_name, "test-project");
        assert_eq!(info.cargo_toml, project.path().join("Cargo.toml"));
        assert!(!info.is_workspace);
        assert!(info.workspace_members.is_empty());
    }

    #[test]
    fn test_get_rust_project_info_with_workspace() {
        let dir = TempDir::new().unwrap();
        
        // Create workspace Cargo.toml
        let cargo_toml = r#"
[workspace]
members = ["crate1", "crate2"]

[package]
name = "workspace-root"
version = "0.1.0"
edition = "2021"
"#;
        fs::write(dir.path().join("Cargo.toml"), cargo_toml).unwrap();
        
        let info = get_rust_project_info(dir.path());
        assert!(info.is_some());
        
        let info = info.unwrap();
        assert!(info.is_workspace);
        assert_eq!(info.workspace_members.len(), 2);
        assert!(info.workspace_members.contains(&"crate1".to_string()));
        assert!(info.workspace_members.contains(&"crate2".to_string()));
    }

    #[test]
    fn test_binary_type_display() {
        assert_eq!(format!("{}", BinaryType::Binary), "Binary");
        assert_eq!(format!("{}", BinaryType::Example), "Example");
        assert_eq!(format!("{}", BinaryType::Test), "Test");
        assert_eq!(format!("{}", BinaryType::Benchmark), "Benchmark");
    }

    #[test]
    fn test_no_project_found() {
        let temp = TempDir::new().unwrap();
        
        assert!(find_rust_project_root(temp.path()).is_none());
        assert!(get_rust_project_info(temp.path()).is_none());
    }

    #[test]
    fn test_is_cargo_available() {
        // This may be true or false depending on the environment
        // Just test that it doesn't panic
        let _ = is_cargo_available();
    }

    #[test]
    fn test_get_rust_version() {
        // This may return Some or None depending on the environment
        // Just test that it doesn't panic
        let _ = get_rust_version();
    }
}
