// Rust debugger module
// Provides debugging capabilities for Rust applications

pub mod ui;

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio, ChildStdin};
use std::sync::{Arc, Mutex};

/// Debugger backend state
pub struct Debugger {
    process: Option<Child>,
    binary_path: Option<PathBuf>,
    breakpoints: Vec<Breakpoint>,
    stdin: Option<Arc<Mutex<ChildStdin>>>,
}

#[derive(Debug, Clone)]
pub struct Breakpoint {
    pub file: String,
    pub line: u32,
    pub enabled: bool,
}

#[derive(Debug, Clone)]
pub struct Variable {
    pub name: String,
    pub value: String,
    pub var_type: String,
}

#[derive(Debug, Clone)]
pub struct StackFrame {
    pub function: String,
    pub file: String,
    pub line: u32,
}

impl Debugger {
    pub fn new() -> Self {
        Debugger {
            process: None,
            binary_path: None,
            breakpoints: Vec::new(),
            stdin: None,
        }
    }

    pub fn add_breakpoint(&mut self, file: String, line: u32) {
        self.breakpoints.push(Breakpoint {
            file,
            line,
            enabled: true,
        });
    }

    pub fn remove_breakpoint(&mut self, file: &str, line: u32) {
        self.breakpoints.retain(|bp| bp.file != file || bp.line != line);
    }

    pub fn get_breakpoints(&self) -> &[Breakpoint] {
        &self.breakpoints
    }

    pub fn set_binary(&mut self, path: PathBuf) {
        self.binary_path = Some(path);
    }

    pub fn get_binary(&self) -> Option<&PathBuf> {
        self.binary_path.as_ref()
    }

    pub fn start(&mut self) -> Result<(), String> {
        let binary_path = self.binary_path.as_ref()
            .ok_or("No binary path set")?;

        if !binary_path.exists() {
            return Err(format!("Binary not found: {:?}", binary_path));
        }

        let mut child = start_debug_session(binary_path)
            .map_err(|e| format!("Failed to start debugger: {}", e))?;

        // Take stdin and wrap it for shared access
        let stdin = child.stdin.take().ok_or("Failed to get GDB stdin")?;
        let stdin_arc = Arc::new(Mutex::new(stdin));
        
        // Send breakpoints to GDB
        {
            use std::io::Write;
            let mut stdin_guard = stdin_arc.lock().unwrap();
            
            for breakpoint in &self.breakpoints {
                let cmd = format!("-break-insert {}:{}\n", breakpoint.file, breakpoint.line);
                println!("[DEBUG] Setting breakpoint: {}", cmd.trim());
                let _ = stdin_guard.write_all(cmd.as_bytes());
                let _ = stdin_guard.flush();
            }
            
            // Small delay to let breakpoints be processed
            drop(stdin_guard);
            std::thread::sleep(std::time::Duration::from_millis(100));
            let mut stdin_guard = stdin_arc.lock().unwrap();
            
            // Start execution
            println!("[DEBUG] Sending -exec-run command");
            let _ = stdin_guard.write_all(b"-exec-run\n");
            let _ = stdin_guard.flush();
            println!("[DEBUG] -exec-run command sent");
        }

        // Spawn a thread to read GDB stderr
        if let Some(stderr) = child.stderr.take() {
            std::thread::spawn(move || {
                use std::io::{BufRead, BufReader};
                let reader = BufReader::new(stderr);
                for line in reader.lines() {
                    if let Ok(text) = line {
                        println!("[GDB-ERR] {}", text);
                    }
                }
            });
        }

        // Spawn a thread to read GDB output
        if let Some(stdout) = child.stdout.take() {
            let stdin_for_reader = stdin_arc.clone();
            std::thread::spawn(move || {
                use std::io::{BufRead, BufReader, Write};
                let reader = BufReader::new(stdout);
                for line in reader.lines() {
                    match line {
                        Ok(text) => {
                            println!("[GDB-OUT] {}", text);
                            
                            // Check for target program output (starts with ~)
                            if text.starts_with("~\"") {
                                // Extract and decode the output
                                if let Some(output) = extract_program_output(&text) {
                                    ui::handle_debug_event(ui::DebugEvent::ProgramOutput { 
                                        text: output 
                                    });
                                }
                            }
                            // Check if this is a non-MI line (target program output)
                            // MI protocol lines start with: ^, *, =, ~, @, or &
                            // Anything else is likely the inferior's output
                            else if !text.is_empty() && 
                                    !text.starts_with('^') && 
                                    !text.starts_with('*') && 
                                    !text.starts_with('=') && 
                                    !text.starts_with('~') && 
                                    !text.starts_with('@') && 
                                    !text.starts_with('&') &&
                                    !text.starts_with("(gdb)") {
                                // This is likely program output - send it to the UI
                                ui::handle_debug_event(ui::DebugEvent::ProgramOutput { 
                                    text: format!("{}\n", text)
                                });
                            }
                            
                            // Parse GDB MI output
                            if text.starts_with("*stopped") {
                                if text.contains("reason=\"breakpoint-hit\"") {
                                    // Parse location
                                    let file = extract_mi_field(&text, "fullname=");
                                    let line = extract_mi_field(&text, "line=")
                                        .and_then(|s| s.parse().ok());
                                    
                                    // Send event to UI
                                    ui::handle_debug_event(ui::DebugEvent::Stopped {
                                        reason: "breakpoint-hit".to_string(),
                                        line,
                                        file,
                                    });
                                    
                                    // Request stack and variables
                                    if let Ok(mut stdin) = stdin_for_reader.lock() {
                                        let _ = stdin.write_all(b"-stack-list-frames\n");
                                        let _ = stdin.write_all(b"-stack-list-variables --simple-values\n");
                                        let _ = stdin.flush();
                                    }
                                } else if text.contains("reason=\"exited") {
                                    println!("[DEBUG] Program exited");
                                    ui::handle_debug_event(ui::DebugEvent::Exited);
                                } else if text.contains("reason=\"signal-received") {
                                    println!("[DEBUG] Program received signal (likely Ctrl+C)");
                                    ui::handle_debug_event(ui::DebugEvent::Exited);
                                }
                            } else if text.starts_with("*running") {
                                println!("[DEBUG] Program started running");
                                ui::handle_debug_event(ui::DebugEvent::Running);
                            } else if text.starts_with("^done") && text.contains("stack=") {
                                // Parse call stack
                                println!("[DEBUG] Parsing stack frames from: {}", &text[..100.min(text.len())]);
                                let frames = parse_stack_frames(&text);
                                println!("[DEBUG] Found {} stack frames", frames.len());
                                if !frames.is_empty() {
                                    ui::handle_debug_event(ui::DebugEvent::StackFrame { frames });
                                }
                            } else if text.starts_with("^done") && text.contains("variables=") {
                                // Parse variables
                                println!("[DEBUG] Parsing variables from: {}", &text[..100.min(text.len())]);
                                let vars = parse_variables(&text);
                                println!("[DEBUG] Found {} variables", vars.len());
                                if !vars.is_empty() {
                                    ui::handle_debug_event(ui::DebugEvent::Variables { vars });
                                }
                            }
                        }
                        Err(_) => break,
                    }
                }
            });
        }

        self.stdin = Some(stdin_arc);
        self.process = Some(child);
        Ok(())
    }    pub fn stop(&mut self) {
        self.stdin = None;
        if let Some(mut proc) = self.process.take() {
            let _ = proc.kill();
            let _ = proc.wait();
        }
    }

    pub fn is_running(&self) -> bool {
        self.process.is_some()
    }
    
    pub fn continue_execution(&self) -> Result<(), String> {
        if let Some(stdin_arc) = &self.stdin {
            use std::io::Write;
            let mut stdin = stdin_arc.lock().map_err(|e| format!("Lock error: {}", e))?;
            stdin.write_all(b"-exec-continue\n")
                .map_err(|e| format!("Failed to send continue command: {}", e))?;
            stdin.flush()
                .map_err(|e| format!("Failed to flush: {}", e))?;
            Ok(())
        } else {
            Err("No active debug session".to_string())
        }
    }
    
    pub fn step_over(&self) -> Result<(), String> {
        if let Some(stdin_arc) = &self.stdin {
            use std::io::Write;
            let mut stdin = stdin_arc.lock().map_err(|e| format!("Lock error: {}", e))?;
            stdin.write_all(b"-exec-next\n")
                .map_err(|e| format!("Failed to send step command: {}", e))?;
            stdin.flush()
                .map_err(|e| format!("Failed to flush: {}", e))?;
            Ok(())
        } else {
            Err("No active debug session".to_string())
        }
    }
    
    pub fn step_into(&self) -> Result<(), String> {
        if let Some(stdin_arc) = &self.stdin {
            use std::io::Write;
            let mut stdin = stdin_arc.lock().map_err(|e| format!("Lock error: {}", e))?;
            stdin.write_all(b"-exec-step\n")
                .map_err(|e| format!("Failed to send step into command: {}", e))?;
            stdin.flush()
                .map_err(|e| format!("Failed to flush: {}", e))?;
            Ok(())
        } else {
            Err("No active debug session".to_string())
        }
    }
}

/// Check if there are any Rust files in the given directory
pub fn has_rust_files(dir: &Path) -> bool {
    if !dir.is_dir() {
        return false;
    }

    // Check for Cargo.toml first (strong indicator of a Rust project)
    if dir.join("Cargo.toml").exists() {
        return true;
    }

    // Recursively check for .rs files
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            if let Ok(file_type) = entry.file_type() {
                let path = entry.path();
                
                if file_type.is_file() {
                    if let Some(extension) = path.extension() {
                        if extension == "rs" {
                            return true;
                        }
                    }
                } else if file_type.is_dir() {
                    // Skip common directories that won't contain source
                    let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if dir_name != "target" && dir_name != ".git" && !dir_name.starts_with('.') {
                        if has_rust_files(&path) {
                            return true;
                        }
                    }
                }
            }
        }
    }

    false
}

/// Start a debug session for a Rust binary
pub fn start_debug_session(binary_path: &Path) -> Result<Child, std::io::Error> {
    // For now, we'll use rust-gdb or lldb
    // Try rust-gdb first, fall back to gdb
    let debugger = if Command::new("rust-gdb").arg("--version").output().is_ok() {
        "rust-gdb"
    } else if Command::new("rust-lldb").arg("--version").output().is_ok() {
        "rust-lldb"
    } else if Command::new("gdb").arg("--version").output().is_ok() {
        "gdb"
    } else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "No debugger found (tried rust-gdb, rust-lldb, gdb)",
        ));
    };

    // Build a representation of the debugger command for the UI
    let config = format!("{} --interpreter=mi {}", debugger, binary_path.display());
    // Notify the UI about the debugger configuration (for debugging purposes)
    ui::handle_debug_event(ui::DebugEvent::GdbConfig { config: config.clone() });

    Command::new(debugger)
        .arg("--interpreter=mi")
        .arg(binary_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
}

/// Find Rust binary to debug in the current project
pub fn find_rust_binary(dir: &Path) -> Option<PathBuf> {
    // First, find the project root by looking for Cargo.toml
    let project_root = find_project_root(dir)?;
    
    // Look for target/debug/ binaries
    let debug_dir = project_root.join("target").join("debug");
    if !debug_dir.exists() {
        return None;
    }

    // Try to find the project name from Cargo.toml
    let cargo_toml = project_root.join("Cargo.toml");
    if cargo_toml.exists() {
        if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
            for line in content.lines() {
                if line.trim().starts_with("name") {
                    if let Some(name) = line.split('=').nth(1) {
                        let name = name.trim().trim_matches('"').trim_matches('\'');
                        let binary = debug_dir.join(name);
                        if binary.exists() && binary.is_file() {
                            return Some(binary);
                        }
                    }
                }
            }
        }
    }

    // Fallback: find any executable in target/debug
    if let Ok(entries) = std::fs::read_dir(&debug_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                // Check if it's executable on Unix
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if let Ok(metadata) = path.metadata() {
                        let permissions = metadata.permissions();
                        if permissions.mode() & 0o111 != 0 {
                            // Skip files with extensions
                            if path.extension().is_none() {
                                return Some(path);
                            }
                        }
                    }
                }
                #[cfg(not(unix))]
                {
                    if path.extension().is_none() {
                        return Some(path);
                    }
                }
            }
        }
    }

    None
}

/// Find the project root by searching upward for Cargo.toml
fn find_project_root(dir: &Path) -> Option<PathBuf> {
    let mut current = dir.to_path_buf();
    
    loop {
        if current.join("Cargo.toml").exists() {
            return Some(current);
        }
        
        if !current.pop() {
            break;
        }
    }
    
    None
}

/// Extract a field value from GDB MI output
/// Example: extract_mi_field("file=\"main.rs\"", "file=") returns Some("main.rs")
fn extract_mi_field(text: &str, field: &str) -> Option<String> {
    if let Some(start) = text.find(field) {
        let value_start = start + field.len();
        let remaining = &text[value_start..];
        
        if remaining.starts_with('"') {
            // Quoted value
            if let Some(end) = remaining[1..].find('"') {
                return Some(remaining[1..=end].to_string());
            }
        } else {
            // Unquoted value (number or identifier)
            let end = remaining.find(|c: char| c == ',' || c == '}' || c == ']')
                .unwrap_or(remaining.len());
            return Some(remaining[..end].to_string());
        }
    }
    None
}

/// Extract program output from GDB MI console output
/// GDB wraps program output in ~"..." format with escaped characters
fn extract_program_output(text: &str) -> Option<String> {
    if !text.starts_with("~\"") {
        return None;
    }
    
    // Find the closing quote
    if let Some(end) = text[2..].rfind('"') {
        let escaped = &text[2..2 + end];
        
        // Unescape common escape sequences
        let unescaped = escaped
            .replace("\\n", "\n")
            .replace("\\r", "\r")
            .replace("\\t", "\t")
            .replace("\\\"", "\"")
            .replace("\\\\", "\\");
        
        Some(unescaped)
    } else {
        None
    }
}

/// Parse stack frames from GDB MI output
fn parse_stack_frames(text: &str) -> Vec<StackFrame> {
    let mut frames = Vec::new();
    
    // Look for frame={...} patterns
    let mut pos = 0;
    while let Some(start) = text[pos..].find("frame={") {
        let frame_start = pos + start + 7;
        
        // Extract function name
        let func = extract_mi_field(&text[frame_start..], "func=")
            .unwrap_or_else(|| "??".to_string());
        
        // Extract file
        let file = extract_mi_field(&text[frame_start..], "file=")
            .or_else(|| extract_mi_field(&text[frame_start..], "fullname="))
            .unwrap_or_else(|| "??".to_string());
        
        // Extract line
        let line = extract_mi_field(&text[frame_start..], "line=")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        
        frames.push(StackFrame {
            function: func,
            file,
            line,
        });
        
        pos = frame_start;
        if pos >= text.len() {
            break;
        }
    }
    
    frames
}

/// Parse variables from GDB MI output
fn parse_variables(text: &str) -> Vec<Variable> {
    let mut vars = Vec::new();
    
    // Look for {name="...",value="..."} patterns
    let mut pos = 0;
    while let Some(start) = text[pos..].find("name=\"") {
        let var_start = pos + start;
        
        // Extract name
        let name = extract_mi_field(&text[var_start..], "name=")
            .unwrap_or_else(|| "??".to_string());
        
        // Extract value
        let value = extract_mi_field(&text[var_start..], "value=")
            .unwrap_or_else(|| "??".to_string());
        
        // Extract type (if available)
        let var_type = extract_mi_field(&text[var_start..], "type=")
            .unwrap_or_else(|| "".to_string());
        
        vars.push(Variable {
            name,
            value,
            var_type,
        });
        
        pos = var_start + 10;
        if pos >= text.len() {
            break;
        }
    }
    
    vars
}
