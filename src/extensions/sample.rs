//! # Sample Extensions — Bundled Archive Stubs
//!
//! Dvop ships a few sample extension archives (`.tar.gz`) in the
//! `extensions/` source directory that users can install via the
//! "Install from file" button in the Extensions panel.
//!
//! This module is intentionally minimal — the archives are static
//! files, not generated at runtime.
//!
//! See FEATURES.md: Feature #87 — Extension System

/// Sample extension archives are shipped as static .tar.gz files
/// in the `extensions/` folder at the project root.
/// Users can import them via the "Install from file" button in the Extensions panel.
///
/// Available samples:
/// - extensions/word-count.tar.gz
/// - extensions/hello-world.tar.gz
pub fn ensure_sample_archives() {
    // No-op: sample archives are pre-built in the source tree (extensions/*.tar.gz).
}
