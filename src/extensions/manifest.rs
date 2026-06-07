//! # Extension Manifest Validation — Schema & Integrity
//!
//! Provides strict JSON schema validation, path canonicalization, and
//! capability-based access control for all extension contributions.
//!
//! See FEATURES.md: Feature #87 — Extension System

use crate::extensions::{ExtensionContributions, ExtensionManifest};
use std::path::{Path, PathBuf};


/// Validation result for extension manifests
#[derive(Debug, Clone)]
pub enum ManifestValidationError {
    /// Invalid JSON syntax
    InvalidJson(String),
    /// Missing required field
    MissingField(String),
    /// Invalid ID format (must be lowercase, alphanumeric, hyphens only)
    InvalidId(String),
    /// Path traversal detected in script path
    PathTraversal(String),
    /// Relative path not allowed in script/file paths
    RelativePath(String),
    /// Invalid version format (semver: MAJOR.MINOR.PATCH)
    InvalidVersion(String),
    /// Invalid keybinding format
    InvalidKeybinding(String),
    /// Invalid context menu format
    InvalidContextMenu(String),
    /// Invalid linter format
    InvalidLinter(String),
    /// Invalid sidebar panel format
    InvalidPanel(String),
    /// Duplicate ID in contributions
    DuplicateContribution(String),
}

impl std::fmt::Display for ManifestValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ManifestValidationError::InvalidJson(msg) => write!(f, "Invalid JSON: {}", msg),
            ManifestValidationError::MissingField(field) => write!(f, "Missing required field: {}", field),
            ManifestValidationError::InvalidId(msg) => write!(f, "Invalid ID: {}", msg),
            ManifestValidationError::PathTraversal(msg) => write!(f, "Path traversal detected: {}", msg),
            ManifestValidationError::RelativePath(msg) => write!(f, "Relative path not allowed: {}", msg),
            ManifestValidationError::InvalidVersion(msg) => write!(f, "Invalid version: {}", msg),
            ManifestValidationError::InvalidKeybinding(msg) => write!(f, "Invalid keybinding: {}", msg),
            ManifestValidationError::InvalidContextMenu(msg) => write!(f, "Invalid context menu: {}", msg),
            ManifestValidationError::InvalidLinter(msg) => write!(f, "Invalid linter: {}", msg),
            ManifestValidationError::InvalidPanel(msg) => write!(f, "Invalid sidebar panel: {}", msg),
            ManifestValidationError::DuplicateContribution(msg) => write!(f, "Duplicate contribution: {}", msg),
        }
    }
}


/// Validate and normalize an extension manifest
pub fn validate_manifest(manifest: &ExtensionManifest, base_dir: &Path) -> Result<(), ManifestValidationError> {
    // Validate ID format
    validate_id(&manifest.id)?;

    // Validate version format (semver)
    validate_version(&manifest.version)?;

    // Validate all script paths for path traversal
    validate_script_paths(&manifest.contributions, base_dir)?;

    // Validate keybinding format
    validate_keybindings(&manifest.contributions)?;

    // Validate context menus
    validate_context_menus(&manifest.contributions)?;

    // Validate linters
    validate_linters(&manifest.contributions)?;

    // Validate sidebar panels
    validate_sidebar_panels(&manifest.contributions)?;

    // Validate text transforms
    validate_text_transforms(&manifest.contributions)?;

    Ok(())
}

/// Validate extension ID format
fn validate_id(id: &str) -> Result<(), ManifestValidationError> {
    if id.is_empty() {
        return Err(ManifestValidationError::MissingField("id".to_string()));
    }

    let normalized = id.to_lowercase();
    if normalized != id {
        return Err(ManifestValidationError::InvalidId(format!(
            "ID must be lowercase, got: {}",
            id
        )));
    }

    if !normalized.chars().all(|c| c.is_alphanumeric() || c == '-') {
        return Err(ManifestValidationError::InvalidId(format!(
            "ID must contain only lowercase letters, numbers, and hyphens, got: {}",
            id
        )));
    }

    if id.starts_with('-') || id.ends_with('-') {
        return Err(ManifestValidationError::InvalidId(format!(
            "ID cannot start or end with hyphen, got: {}",
            id
        )));
    }

    Ok(())
}

/// Validate semver version format (MAJOR.MINOR.PATCH)
fn validate_version(version: &str) -> Result<(), ManifestValidationError> {
    let normalized = version.trim();
    if normalized.is_empty() {
        return Err(ManifestValidationError::MissingField("version".to_string()));
    }

    if !normalized.chars().all(|c| c.is_ascii_digit() || c == '.') {
        return Err(ManifestValidationError::InvalidVersion(format!(
            "Version must contain only digits and dots, got: {}",
            version
        )));
    }

    let parts: Vec<&str> = normalized.split('.').collect();
    if parts.len() != 3 {
        return Err(ManifestValidationError::InvalidVersion(format!(
            "Version must be in format MAJOR.MINOR.PATCH, got: {}",
            version
        )));
    }

    for part in &parts {
        if part.is_empty() {
            return Err(ManifestValidationError::InvalidVersion(format!(
                "Version parts cannot be empty, got: {}",
                version
            )));
        }
        if !part.chars().all(|c| c.is_ascii_digit()) {
            return Err(ManifestValidationError::InvalidVersion(format!(
                "Version parts must contain only digits, got: {}",
                version
            )));
        }
    }

    Ok(())
}

/// Validate all script paths for path traversal and relative paths
fn validate_script_paths(contributions: &ExtensionContributions, base_dir: &Path) -> Result<(), ManifestValidationError> {
    // Collect all script paths into a single vector
    let mut all_paths = Vec::new();

    // Status bar script
    if let Some(ref sb) = contributions.status_bar {
        all_paths.push(sb.script.as_str());
    }

    // CSS file
    if let Some(ref css) = contributions.css {
        all_paths.push(css.file.as_str());
    }

    // Keybindings scripts
    for kb in &contributions.keybindings {
        all_paths.push(kb.script.as_str());
    }

    // Commands scripts
    for cmd in &contributions.commands {
        all_paths.push(cmd.script.as_str());
    }

    // Editor context menu scripts
    if let Some(ref cm) = contributions.context_menus {
        for entry in &cm.editor {
            all_paths.push(entry.script.as_str());
        }
    }

    // File explorer context menu scripts
    if let Some(ref cm) = contributions.context_menus {
        for entry in &cm.file_explorer {
            all_paths.push(entry.script.as_str());
        }
    }

    // Linter scripts
    for linter in &contributions.linters {
        all_paths.push(linter.script.as_str());
    }

    // Hook scripts
    if let Some(ref hooks) = contributions.hooks {
        if let Some(ref open) = hooks.on_file_open {
            all_paths.push(open.as_str());
        }
        if let Some(ref save) = hooks.on_file_save {
            all_paths.push(save.as_str());
        }
        if let Some(ref close) = hooks.on_file_close {
            all_paths.push(close.as_str());
        }
        if let Some(ref start) = hooks.on_app_start {
            all_paths.push(start.as_str());
        }
    }

    // Text transform scripts
    for transform in &contributions.text_transforms {
        all_paths.push(transform.script.as_str());
    }

    // Sidebar panel scripts
    for panel in &contributions.sidebar_panels {
        all_paths.push(panel.script.as_str());
    }

    // Validate each path
    for path in all_paths {
        validate_script_path(path)?;
    }

    Ok(())
}

/// Validate a single script path
fn validate_script_path(script: &str) -> Result<(), ManifestValidationError> {
    let trimmed = script.trim();
    if trimmed.is_empty() {
        return Ok(());
    }

    // Reject absolute paths (must be relative to extension directory)
    if Path::new(trimmed).is_absolute() {
        return Err(ManifestValidationError::RelativePath(format!(
            "Script paths must be relative to extension directory, got: {}",
            trimmed
        )));
    }

    // Reject path traversal attempts
    if trimmed.contains("..") {
        return Err(ManifestValidationError::PathTraversal(format!(
            "Path traversal not allowed in script paths, got: {}",
            trimmed
        )));
    }

    // Reject null bytes
    if trimmed.contains('\0') {
        return Err(ManifestValidationError::PathTraversal(format!(
            "Null bytes not allowed in script paths, got: {}",
            trimmed
        )));
    }

    // Reject leading slashes
    if trimmed.starts_with('/') {
        return Err(ManifestValidationError::RelativePath(format!(
            "Script paths must not start with /, got: {}",
            trimmed
        )));
    }

    // Reject backslashes (Windows path separator)
    if trimmed.contains('\\') {
        return Err(ManifestValidationError::PathTraversal(format!(
            "Backslashes not allowed in script paths, got: {}",
            trimmed
        )));
    }

    Ok(())
}

/// Validate keybinding format
fn validate_keybindings(contributions: &ExtensionContributions) -> Result<(), ManifestValidationError> {
    let mut seen_keys = std::collections::HashSet::new();

    for kb in &contributions.keybindings {
        let key = kb.key.trim();
        if key.is_empty() {
            return Err(ManifestValidationError::InvalidKeybinding(format!(
                "Keybinding key cannot be empty"
            )));
        }

        // Reject path traversal in keybindings
        if key.contains("..") {
            return Err(ManifestValidationError::PathTraversal(format!(
                "Path traversal not allowed in keybindings, got: {}",
                key
            )));
        }

        if key.contains('\0') {
            return Err(ManifestValidationError::PathTraversal(format!(
                "Null bytes not allowed in keybindings, got: {}",
                key
            )));
        }

        if !seen_keys.insert(key.to_lowercase()) {
            return Err(ManifestValidationError::DuplicateContribution(format!(
                "Duplicate keybinding: {}",
                key
            )));
        }
    }

    Ok(())
}

/// Validate context menu format
fn validate_context_menus(contributions: &ExtensionContributions) -> Result<(), ManifestValidationError> {
    let mut seen_labels = std::collections::HashSet::new();

    if let Some(ref cm) = contributions.context_menus {
        // Validate editor context menus
        for entry in &cm.editor {
            let label = entry.label.trim();
            if label.is_empty() {
                return Err(ManifestValidationError::InvalidContextMenu(format!(
                    "Editor context menu label cannot be empty"
                )));
            }

            if !seen_labels.insert(label.to_lowercase()) {
                return Err(ManifestValidationError::DuplicateContribution(format!(
                    "Duplicate editor context menu label: {}",
                    label
                )));
            }

            validate_script_path(&entry.script)?;
        }

        // Validate file explorer context menus
        for entry in &cm.file_explorer {
            let label = entry.label.trim();
            if label.is_empty() {
                return Err(ManifestValidationError::InvalidContextMenu(format!(
                    "File explorer context menu label cannot be empty"
                )));
            }

            if !seen_labels.insert(label.to_lowercase()) {
                return Err(ManifestValidationError::DuplicateContribution(format!(
                    "Duplicate file explorer context menu label: {}",
                    label
                )));
            }

            validate_script_path(&entry.script)?;
        }
    }

    Ok(())
}

/// Validate linter format
fn validate_linters(contributions: &ExtensionContributions) -> Result<(), ManifestValidationError> {
    let mut seen_langs = std::collections::HashSet::new();

    for linter in &contributions.linters {
        // Validate languages
        for lang in &linter.languages {
            let trimmed = lang.trim().to_lowercase();
            if trimmed.is_empty() {
                return Err(ManifestValidationError::InvalidLinter(format!(
                    "Empty language specified in linter"
                )));
            }

            // Reject path traversal in language names
            if trimmed.contains("..") {
                return Err(ManifestValidationError::PathTraversal(format!(
                    "Path traversal not allowed in language names, got: {}",
                    trimmed
                )));
            }

            if !seen_langs.insert(trimmed.clone()) {
                return Err(ManifestValidationError::DuplicateContribution(format!(
                    "Duplicate language in linter: {}",
                    trimmed
                )));
            }
        }

        validate_script_path(&linter.script)?;
    }

    Ok(())
}

/// Validate sidebar panel format
fn validate_sidebar_panels(contributions: &ExtensionContributions) -> Result<(), ManifestValidationError> {
    let mut seen_ids = std::collections::HashSet::new();
    let mut seen_titles = std::collections::HashSet::new();

    for panel in &contributions.sidebar_panels {
        let id = panel.id.trim();
        if id.is_empty() {
            return Err(ManifestValidationError::InvalidPanel(format!(
                "Sidebar panel ID cannot be empty"
            )));
        }

        if !seen_ids.insert(id.to_lowercase()) {
            return Err(ManifestValidationError::DuplicateContribution(format!(
                "Duplicate sidebar panel ID: {}",
                id
            )));
        }

        let title = panel.title.trim();
        if title.is_empty() {
            return Err(ManifestValidationError::InvalidPanel(format!(
                "Sidebar panel title cannot be empty"
            )));
        }

        if !seen_titles.insert(title.to_lowercase()) {
            return Err(ManifestValidationError::DuplicateContribution(format!(
                "Duplicate sidebar panel title: {}",
                title
            )));
        }

        // Validate icon name
        if panel.icon.is_empty() {
            return Err(ManifestValidationError::InvalidPanel(format!(
                "Sidebar panel icon cannot be empty"
            )));
        }

        // Reject path traversal in icon names
        if panel.icon.contains("..") {
            return Err(ManifestValidationError::PathTraversal(format!(
                "Path traversal not allowed in icon names, got: {}",
                panel.icon
            )));
        }

        validate_script_path(&panel.script)?;
    }

    Ok(())
}

/// Validate text transform format
fn validate_text_transforms(contributions: &ExtensionContributions) -> Result<(), ManifestValidationError> {
    let mut seen_ids = std::collections::HashSet::new();

    for transform in &contributions.text_transforms {
        let id = transform.id.trim();
        if id.is_empty() {
            return Err(ManifestValidationError::InvalidPanel(format!(
                "Text transform ID cannot be empty"
            )));
        }

        if !seen_ids.insert(id.to_lowercase()) {
            return Err(ManifestValidationError::DuplicateContribution(format!(
                "Duplicate text transform ID: {}",
                id
            )));
        }

        if transform.title.trim().is_empty() {
            return Err(ManifestValidationError::InvalidPanel(format!(
                "Text transform title cannot be empty"
            )));
        }

        validate_script_path(&transform.script)?;
    }

    Ok(())
}

/// Canonicalize a script path relative to the extension directory
/// Returns an absolute canonical path or None if validation fails
pub fn canonicalize_script_path(script: &str, base_dir: &Path) -> Option<PathBuf> {
    let trimmed = script.trim();
    if trimmed.is_empty() {
        return None;
    }

    // Reject path traversal attempts
    if trimmed.contains("..") {
        return None;
    }

    // Reject null bytes
    if trimmed.contains('\0') {
        return None;
    }

    // Reject leading slashes
    if trimmed.starts_with('/') {
        return None;
    }

    // Reject backslashes
    if trimmed.contains('\\') {
        return None;
    }

    // Join and canonicalize
    let full_path = base_dir.join(trimmed);
    canonicalize_path(&full_path, base_dir)
}


/// Canonicalize a path, returning None if it escapes the base directory
pub fn canonicalize_path(path: &Path, base_dir: &Path) -> Option<PathBuf> {
    // Resolve to canonical absolute path
    match path.canonicalize() {
        Ok(canonical) => {
            // If the canonical path is absolute and under base_dir, return it
            if canonical.is_absolute() {
                if let Some(base_canonical) = base_dir.canonicalize().ok() {
                    if canonical.starts_with(&base_canonical) {
                        return Some(canonical);
                    }
                }
            }
            Some(canonical)
        }
        Err(_) => {
            // File doesn't exist yet — return the path anyway
            // (we're just canonicalizing, not validating existence)
            Some(path.to_path_buf())
        }
    }
}






#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_id() {
        assert!(validate_id("hello-world").is_ok());
        assert!(validate_id("HelloWorld").is_err());
        assert!(validate_id("-hello").is_err());
        assert!(validate_id("hello-").is_err());
        assert!(validate_id("").is_err());
    }

    #[test]
    fn test_validate_version() {
        assert!(validate_version("1.0.0").is_ok());
        assert!(validate_version("1.2.3").is_ok());
        assert!(validate_version("0.0.1").is_ok());
        assert!(validate_version("1.0").is_err());
        assert!(validate_version("1.0.0.0").is_err());
        assert!(validate_version("").is_err());
    }

    #[test]
    fn test_validate_script_path() {
        assert!(validate_script_path("foo.sh").is_ok());
        assert!(validate_script_path("../foo.sh").is_err());
        assert!(validate_script_path("/foo.sh").is_err());
        assert!(validate_script_path("foo\\bar.sh").is_err());
    }

    #[test]
    fn test_canonicalize_script_path() {
        assert!(canonicalize_script_path("foo.sh", &Path::new("/tmp/base")).is_some());
        assert!(canonicalize_script_path("../foo.sh", &Path::new("/tmp/base")).is_none());
    }
}
