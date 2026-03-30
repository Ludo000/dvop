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
    stdin_data: Option<&str>,
) -> Result<String, String> {
    if !script_path.exists() {
        return Err(format!("Script not found: {}", script_path.display()));
    }

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
        }
    }

    // Wait with timeout using a thread
    let (tx, rx) = std::sync::mpsc::channel();
    let thread_handle = {
        std::thread::spawn(move || {
            let result = child.wait_with_output();
            let _ = tx.send(result);
        })
    };

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
            drop(thread_handle);
            Err("Script timed out (5s)".to_string())
        }
    }
}

/// Run a script and parse its JSON stdout into a typed result.
pub fn run_script_json<T: serde::de::DeserializeOwned>(
    script_path: &Path,
    args: &[&str],
) -> Result<T, String> {
    let output = run_script(script_path, args, None)?;
    serde_json::from_str(&output)
        .map_err(|e| format!("Failed to parse script JSON output: {}", e))
}

/// Run a script without waiting for output (fire-and-forget).
/// The script runs in a background thread with a timeout guard.
pub fn run_script_fire_and_forget(script_path: &Path, args: &[&str]) {
    if !script_path.exists() {
        return;
    }

    let script_path = script_path.to_path_buf();
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();

    std::thread::spawn(move || {
        let mut cmd = Command::new("bash");
        cmd.arg(&script_path);
        for arg in &args {
            cmd.arg(arg);
        }
        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::piped());

        match cmd.spawn() {
            Ok(child) => {
                // Wait with timeout
                let (tx, rx) = std::sync::mpsc::channel();
                std::thread::spawn(move || {
                    let result = child.wait_with_output();
                    let _ = tx.send(result);
                });
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
