//! # Script Runner — Shell Script Execution with Timeout
//!
//! All script-based extension actions (status bar, linters, transforms, hooks)
//! are executed through this module. Scripts receive arguments via CLI args
//! and optionally receive text on stdin; their stdout is captured and returned.
//!
//! A 5-second timeout prevents runaway scripts from blocking the UI.
//!
//! ## Functions
//!
//! - `run_script()` — run synchronously, capture stdout.
//! - `run_script_json()` — run + parse JSON output into a typed `T`.
//! - `run_script_fire_and_forget()` — spawn asynchronously, no output capture.
//!
//! See FEATURES.md: Feature #96 — Extension Script Execution

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

/// Default timeout for extension scripts
const SCRIPT_TIMEOUT: Duration = Duration::from_secs(5);

/// Run a bash script and capture its stdout.
///
/// - `script_path`: absolute path to the script
/// - `args`: arguments passed to the script ($1, $2, ...)
/// - `stdin_data`: optional data piped to the script's stdin
///
/// Returns the trimmed stdout on success.
pub fn run_script(
    script_path: &Path,
    args: &[&str],
    // Option<T> is an enum that represents an optional value: either Some(T) or None.
    stdin_data: Option<&str>,
// Result<T, E> is an enum used for returning and propagating errors: either Ok(T) or Err(E).
) -> Result<String, String> {
    // `stdin_data` feeds transforms / stdin-driven scripts; `None` keeps stdin closed so hooks that do not read stdin exit cleanly.
    if !script_path.exists() {
        return Err(format!("Script not found: {}", script_path.display()));
    }

    // Run via `bash /path/to/script` so extension authors need not chmod +x or rely on a shebang line.
    let mut cmd = Command::new("bash");
    cmd.arg(script_path);
    for arg in args {
        cmd.arg(arg);
    }

    if stdin_data.is_some() {
        cmd.stdin(Stdio::piped());
    } else {
        cmd.stdin(Stdio::null());
    }
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| format!("Failed to spawn script: {}", e))?;

    if let Some(data) = stdin_data {
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(data.as_bytes());
            // stdin is dropped here, closing the pipe
            // Dropping `stdin` closes the pipe so the script sees EOF and can finish reading.
        }
    }

    // Wait with timeout using a thread
    // `wait_with_output` blocks indefinitely — run it on a helper thread and block the caller with a timeout.
    // Sender pushes one `Result` when child exits; receiver blocks until then or until recv_timeout fires.
    let (tx, rx) = std::sync::mpsc::channel();
    let thread_handle = {
        // The "move" keyword forces the closure to take ownership of the variables it uses.
        std::thread::spawn(move || {
            let result = child.wait_with_output();
            let _ = tx.send(result);
        })
    };

    // match statements evaluate different cases and MUST be exhaustive (cover all possibilities).
    match rx.recv_timeout(SCRIPT_TIMEOUT) {
        Ok(Ok(output)) => {
            if output.status.success() {
                Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(format!("Script exited with {}: {}", output.status, stderr.trim()))
            }
        }
        Ok(Err(e)) => Err(format!("Script I/O error: {}", e)),
        Err(_) => {
            // Timeout — try to clean up the thread (it will eventually finish)
            // `recv_timeout` fired — child may still be running; we abandon waiting (UI stays responsive).
            drop(thread_handle);
            Err("Script timed out (5s)".to_string())
        }
    }
}

/// Run a script and parse its JSON stdout into a typed result.
pub fn run_script_json<T: serde::de::DeserializeOwned>(
    script_path: &Path,
    args: &[&str],
// Result<T, E> is an enum used for returning and propagating errors: either Ok(T) or Err(E).
) -> Result<T, String> {
    let output = run_script(script_path, args, None)?;
    // Caller chooses `T`; stdout must be a single JSON value with no extra text (structured linters/transforms share this contract).
    serde_json::from_str(&output)
        .map_err(|e| format!("Failed to parse script JSON output: {}", e))
}

/// Run a script without waiting for output (fire-and-forget).
/// The script runs in a background thread with a timeout guard.
pub fn run_script_fire_and_forget(script_path: &Path, args: &[&str]) {
    // Same `SCRIPT_TIMEOUT` as synchronous runs — a wedged hook cannot hang forever even though GTK never blocks here.
    if !script_path.exists() {
        return;
    }

    let script_path = script_path.to_path_buf();
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();

    // The "move" keyword forces the closure to take ownership of the variables it uses.
    std::thread::spawn(move || {
        let mut cmd = Command::new("bash");
        cmd.arg(&script_path);
        for arg in &args {
            cmd.arg(arg);
        }
        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::piped());
        // Fire-and-forget hooks only need stderr for failures — stdout is discarded so noisy scripts do not flood logs.

        // match statements evaluate different cases and MUST be exhaustive (cover all possibilities).
        match cmd.spawn() {
            Ok(child) => {
                // Wait with timeout
                let (tx, rx) = std::sync::mpsc::channel();
                // The "move" keyword forces the closure to take ownership of the variables it uses.
                std::thread::spawn(move || {
                    let result = child.wait_with_output();
                    let _ = tx.send(result);
                });
                // match statements evaluate different cases and MUST be exhaustive (cover all possibilities).
                match rx.recv_timeout(SCRIPT_TIMEOUT) {
                    Ok(Ok(output)) => {
                        if !output.status.success() {
                            let stderr = String::from_utf8_lossy(&output.stderr);
                            eprintln!(
                                "Hook script {:?} failed: {}",
                                script_path.file_name().unwrap_or_default(),
                                stderr.trim()
                            );
                        }
                    }
                    Ok(Err(e)) => {
                        eprintln!("Hook script I/O error: {}", e);
                    }
                    Err(_) => {
                        eprintln!(
                            "Hook script {:?} timed out",
                            script_path.file_name().unwrap_or_default()
                        );
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to spawn hook script: {}", e);
            }
        }
    });
}

#[cfg(test)]
#[path = "../../tests/unit/extensions/runner_tests.rs"]
mod tests;
