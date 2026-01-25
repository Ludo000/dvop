// Rust Debugger module for Dvop
// Provides debugging capabilities for Rust projects using GDB/LLDB

pub mod rust_project;
pub mod ui;

use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::io::{BufRead, BufReader, Write};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::thread;
use std::time::Duration;

/// Helper function to get the parent PID of a process
fn get_parent_pid(pid: u32) -> Option<u32> {
    let stat_path = format!("/proc/{}/stat", pid);
    if let Ok(stat) = std::fs::read_to_string(&stat_path) {
        // stat format: pid (comm) state ppid ...
        // Find the closing paren and get ppid after it
        if let Some(paren_end) = stat.rfind(')') {
            let after_comm = &stat[paren_end + 2..];
            let fields: Vec<&str> = after_comm.split_whitespace().collect();
            if fields.len() >= 2 {
                return fields[1].parse::<u32>().ok();
            }
        }
    }
    None
}

/// Debugger state
#[derive(Debug, Clone, PartialEq)]
pub enum DebuggerState {
    Stopped,
    Running,
    Paused,
    Exited,
}

/// Breakpoint information
#[derive(Debug, Clone)]
pub struct Breakpoint {
    pub id: u32,
    pub file: PathBuf,
    pub line: u32,
    pub enabled: bool,
    pub hit_count: u32,
    pub condition: Option<String>,
}

/// Stack frame information
#[derive(Debug, Clone)]
pub struct StackFrame {
    pub level: u32,
    pub function: String,
    pub file: Option<PathBuf>,
    pub line: Option<u32>,
    pub address: String,
}

/// Variable information
#[derive(Debug, Clone)]
pub struct Variable {
    pub name: String,
    pub value: String,
    pub var_type: String,
    pub children: Vec<Variable>,
}

/// Debug event types
#[derive(Debug, Clone)]
pub enum DebugEvent {
    Started,
    Stopped { reason: String, file: Option<PathBuf>, line: Option<u32> },
    Continued,
    Exited { exit_code: i32 },
    BreakpointHit { id: u32, file: PathBuf, line: u32 },
    Output { text: String },
    Error { message: String },
}

/// Rust debugger configuration
#[derive(Debug, Clone)]
pub struct DebugConfig {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub working_dir: PathBuf,
    pub env_vars: HashMap<String, String>,
    pub stop_on_entry: bool,
}

impl Default for DebugConfig {
    fn default() -> Self {
        Self {
            program: PathBuf::new(),
            args: Vec::new(),
            working_dir: PathBuf::new(),
            env_vars: HashMap::new(),
            stop_on_entry: false,  // Run immediately, stop only at user breakpoints
        }
    }
}

/// Rust debugger backend (uses GDB/LLDB via MI interface)
pub struct RustDebugger {
    state: Arc<Mutex<DebuggerState>>,
    process: Arc<Mutex<Option<Child>>>,
    breakpoints: Arc<Mutex<Vec<Breakpoint>>>,
    next_breakpoint_id: Arc<Mutex<u32>>,
    config: Arc<Mutex<Option<DebugConfig>>>,
    output_buffer: Arc<Mutex<Vec<String>>>,
    stdin_writer: Arc<Mutex<Option<std::process::ChildStdin>>>,
    inferior_pid: Arc<Mutex<Option<u32>>>,  // PID of the debugged process
    all_inferior_pids: Arc<Mutex<Vec<u32>>>, // All PIDs seen for inferior (for fork tracking)
    gdb_pid: Arc<Mutex<Option<u32>>>,       // PID of the GDB process (session root)
    our_pid: u32,                            // PID of the main app (to never kill)
}

impl RustDebugger {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(DebuggerState::Stopped)),
            process: Arc::new(Mutex::new(None)),
            breakpoints: Arc::new(Mutex::new(Vec::new())),
            next_breakpoint_id: Arc::new(Mutex::new(1)),
            inferior_pid: Arc::new(Mutex::new(None)),
            all_inferior_pids: Arc::new(Mutex::new(Vec::new())),
            gdb_pid: Arc::new(Mutex::new(None)),
            our_pid: std::process::id(),  // Store our own PID to never kill it
            config: Arc::new(Mutex::new(None)),
            output_buffer: Arc::new(Mutex::new(Vec::new())),
            stdin_writer: Arc::new(Mutex::new(None)),
        }
    }

    /// Get the current debugger state
    pub fn state(&self) -> DebuggerState {
        self.state.lock().unwrap().clone()
    }

    /// Set the debug configuration
    pub fn set_config(&self, config: DebugConfig) {
        *self.config.lock().unwrap() = Some(config);
    }

    /// Get the debug configuration
    pub fn get_config(&self) -> Option<DebugConfig> {
        self.config.lock().unwrap().clone()
    }

    /// Check if GDB is available
    pub fn is_gdb_available() -> bool {
        Command::new("gdb")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok()
    }

    /// Check if LLDB is available
    pub fn is_lldb_available() -> bool {
        Command::new("lldb")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok()
    }

    /// Get the preferred debugger (GDB or LLDB)
    pub fn get_preferred_debugger() -> Option<&'static str> {
        if Self::is_gdb_available() {
            Some("gdb")
        } else if Self::is_lldb_available() {
            Some("lldb")
        } else {
            None
        }
    }

    /// Start debugging a program
    pub fn start(&self) -> Result<(), String> {
        let config = self.config.lock().unwrap().clone()
            .ok_or("No debug configuration set")?;

        // Verify the program exists
        if !config.program.exists() {
            return Err(format!("Program not found: {}. Please build the project first.", config.program.display()));
        }

        let debugger = Self::get_preferred_debugger()
            .ok_or("No debugger available (GDB or LLDB required)")?;

        // Build the debug command
        let mut cmd = Command::new(debugger);
        
        match debugger {
            "gdb" => {
                cmd.arg("--interpreter=mi2")  // Use GDB/MI interface
                    .arg("--quiet")           // Suppress banner
                    .arg(&config.program);
            }
            "lldb" => {
                cmd.arg("--")
                    .arg(&config.program);
            }
            _ => return Err("Unsupported debugger".to_string()),
        }

        // Set working directory
        if config.working_dir.exists() {
            cmd.current_dir(&config.working_dir);
        }

        // Set environment variables from config
        for (key, value) in &config.env_vars {
            cmd.env(key, value);
        }

        // Configure I/O
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // On Unix, create a new process group so we can kill all children
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            // Create new process group with GDB as the leader
            unsafe {
                cmd.pre_exec(|| {
                    libc::setpgid(0, 0);
                    Ok(())
                });
            }
        }

        // Start the debugger process
        let mut child = cmd.spawn()
            .map_err(|e| format!("Failed to start debugger: {}", e))?;

        // Store GDB's PID immediately - this is our session root
        let gdb_pid = child.id();
        *self.gdb_pid.lock().unwrap() = Some(gdb_pid);

        // Take stdin for writing commands
        let stdin = child.stdin.take()
            .ok_or("Failed to open stdin")?;
        
        // Take stdout for reading output in background thread
        let stdout = child.stdout.take()
            .ok_or("Failed to open stdout")?;
        
        // Spawn background thread to read GDB output
        let output_buffer = self.output_buffer.clone();
        let state = self.state.clone();
        let inferior_pid = self.inferior_pid.clone();
        let all_inferior_pids = self.all_inferior_pids.clone();
        let our_pid = self.our_pid;  // Copy our PID to check in the thread
        thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                match line {
                    Ok(line) => {
                        // Parse inferior PID from GDB output - multiple patterns
                        // Pattern 1: "[Inferior 1 (process 12345)" 
                        // Pattern 2: "process 12345"
                        // Pattern 3: "[New Thread ... (LWP 12345)]" - extract parent PID
                        
                        let mut found_pid: Option<u32> = None;
                        
                        // Check for "process " pattern (most reliable)
                        if line.contains("process ") {
                            if let Some(start) = line.find("process ") {
                                let after = &line[start + 8..];
                                let pid_str: String = after.chars()
                                    .take_while(|c| c.is_ascii_digit())
                                    .collect();
                                if let Ok(pid) = pid_str.parse::<u32>() {
                                    found_pid = Some(pid);
                                }
                            }
                        }
                        
                        // Check for LWP pattern to get thread PIDs (the parent of LWP is the main process)
                        // Format: [New Thread 0x... (LWP 12345)]
                        if found_pid.is_none() && line.contains("(LWP ") {
                            if let Some(start) = line.find("(LWP ") {
                                let after = &line[start + 5..];
                                let pid_str: String = after.chars()
                                    .take_while(|c| c.is_ascii_digit())
                                    .collect();
                                if let Ok(lwp_pid) = pid_str.parse::<u32>() {
                                    // LWP is actually a thread ID, get its parent (main process)
                                    if let Some(ppid) = get_parent_pid(lwp_pid) {
                                        found_pid = Some(ppid);
                                    }
                                }
                            }
                        }
                        
                        // Store the PID if we found one and it's not our own
                        if let Some(pid) = found_pid {
                            if pid != our_pid && pid > 1 {
                                // Track all PIDs we've seen
                                {
                                    let mut all_pids = all_inferior_pids.lock().unwrap();
                                    if !all_pids.contains(&pid) {
                                        all_pids.push(pid);
                                    }
                                }
                                // Set as current inferior if we don't have one
                                let mut guard = inferior_pid.lock().unwrap();
                                if guard.is_none() {
                                    *guard = Some(pid);
                                }
                            }
                        }
                        
                        output_buffer.lock().unwrap().push(line);
                    }
                    Err(_) => break,
                }
                // Check if we should stop
                if *state.lock().unwrap() == DebuggerState::Stopped {
                    break;
                }
            }
        });
        
        *self.stdin_writer.lock().unwrap() = Some(stdin);
        *self.process.lock().unwrap() = Some(child);
        *self.state.lock().unwrap() = DebuggerState::Paused;

        // Give GDB time to initialize
        thread::sleep(Duration::from_millis(100));

        // Configure GDB fork handling:
        // - follow-fork-mode parent: Stay with the main dvop process, don't follow into shell subprocesses
        // - detach-on-fork on: Let child processes (like terminal shells) run freely without GDB
        let _ = self.send_command_no_wait("set follow-fork-mode parent");
        let _ = self.send_command_no_wait("set detach-on-fork on");
        
        // Disable pagination and confirmations
        let _ = self.send_command_no_wait("set pagination off");
        let _ = self.send_command_no_wait("set confirm off");
        
        // CRITICAL: Set environment variables for the inferior (debugged) process
        // This prevents GTK single-instance detection from closing the debugged app
        // when another instance is already running (e.g., when debugging dvop with dvop)
        // The DVOP_DEBUG_INSTANCE variable is checked by main.rs to run in non-unique mode
        let _ = self.send_command_no_wait("set environment DVOP_DEBUG_INSTANCE=1");

        // Set any pending breakpoints
        let _ = self.apply_breakpoints();

        // Set program arguments if any
        if !config.args.is_empty() {
            let args_str = config.args.join(" ");
            let _ = self.send_command_no_wait(&format!("-exec-arguments {}", args_str));
        }

        // Set a breakpoint at main if stop_on_entry
        if config.stop_on_entry {
            let _ = self.send_command_no_wait("-break-insert -t main");
        }

        // Run the program
        let _ = self.send_command_no_wait("-exec-run");
        
        *self.state.lock().unwrap() = DebuggerState::Running;

        Ok(())
    }

    /// Apply all breakpoints to the debugger
    fn apply_breakpoints(&self) -> Result<(), String> {
        let breakpoints = self.breakpoints.lock().unwrap().clone();
        
        for bp in breakpoints {
            if bp.enabled {
                let _ = self.send_command_no_wait(&format!(
                    "-break-insert {}:{}",
                    bp.file.display(),
                    bp.line
                ));
            }
        }
        
        Ok(())
    }

    /// Continue execution
    pub fn continue_execution(&self) -> Result<(), String> {
        if self.state() != DebuggerState::Paused {
            return Err("Debugger is not paused".to_string());
        }

        self.send_command_no_wait("-exec-continue")?;
        *self.state.lock().unwrap() = DebuggerState::Running;
        Ok(())
    }

    /// Pause execution
    pub fn pause(&self) -> Result<(), String> {
        if self.state() != DebuggerState::Running {
            return Err("Debugger is not running".to_string());
        }

        self.send_command_no_wait("-exec-interrupt")?;
        *self.state.lock().unwrap() = DebuggerState::Paused;
        Ok(())
    }

    /// Step over (next line)
    pub fn step_over(&self) -> Result<(), String> {
        if self.state() != DebuggerState::Paused {
            return Err("Debugger is not paused".to_string());
        }

        self.send_command_no_wait("-exec-next")?;
        Ok(())
    }

    /// Step into function
    pub fn step_into(&self) -> Result<(), String> {
        if self.state() != DebuggerState::Paused {
            return Err("Debugger is not paused".to_string());
        }

        self.send_command_no_wait("-exec-step")?;
        Ok(())
    }

    /// Step out of function
    pub fn step_out(&self) -> Result<(), String> {
        if self.state() != DebuggerState::Paused {
            return Err("Debugger is not paused".to_string());
        }

        self.send_command_no_wait("-exec-finish")?;
        Ok(())
    }

    /// Stop debugging - safely kills only the debugged process, never the main app
    pub fn stop(&self) -> Result<(), String> {
        // Set state to stopped first (signals background thread to stop)
        *self.state.lock().unwrap() = DebuggerState::Stopped;
        
        // Get the GDB PID (our session root) - this is the key to safe killing
        let gdb_pid = *self.gdb_pid.lock().unwrap();
        
        // Get the inferior PID if we have it
        let inferior_pid = *self.inferior_pid.lock().unwrap();
        
        // Get all tracked PIDs
        let all_pids = self.all_inferior_pids.lock().unwrap().clone();
        
        eprintln!("[DEBUG STOP] GDB PID: {:?}, Inferior PID: {:?}, All tracked PIDs: {:?}, Our PID: {}", 
                  gdb_pid, inferior_pid, all_pids, self.our_pid);

        // First, try to interrupt and kill the inferior via GDB commands (before closing stdin)
        // This is the cleanest way to stop the debugged process
        {
            let mut stdin_guard = self.stdin_writer.lock().unwrap();
            if let Some(ref mut stdin) = *stdin_guard {
                eprintln!("[DEBUG STOP] Sending GDB commands...");
                // Send interrupt to pause the inferior
                let _ = writeln!(stdin, "-exec-interrupt --all");
                let _ = stdin.flush();
                thread::sleep(Duration::from_millis(20));
                
                // Kill the inferior process via GDB
                let _ = writeln!(stdin, "kill");
                let _ = stdin.flush();
                thread::sleep(Duration::from_millis(20));
                
                // Confirm kill
                let _ = writeln!(stdin, "y");
                let _ = stdin.flush();
                thread::sleep(Duration::from_millis(20));
                
                // Tell GDB to quit
                let _ = writeln!(stdin, "-gdb-exit");
                let _ = stdin.flush();
            }
        }
        
        // Give GDB a moment to process commands
        thread::sleep(Duration::from_millis(100));
        
        // Now close stdin
        *self.stdin_writer.lock().unwrap() = None;

        // Safety: Only kill processes that are descendants of our GDB session
        // This ensures we never kill the main app even if it has the same name
        #[cfg(unix)]
        {
            // First, try to kill by process group (GDB was started with setpgid)
            if let Some(gdb) = gdb_pid {
                eprintln!("[DEBUG STOP] Killing process group -{}", gdb);
                unsafe {
                    // Kill the entire process group with SIGKILL
                    // Negative PID means kill the process group
                    libc::kill(-(gdb as i32), libc::SIGKILL);
                }
            }
            
            // Also explicitly kill the inferior if we have its PID
            if let Some(inf_pid) = inferior_pid {
                if inf_pid != self.our_pid {
                    eprintln!("[DEBUG STOP] Killing inferior PID {}", inf_pid);
                    unsafe {
                        libc::kill(inf_pid as i32, libc::SIGKILL);
                    }
                    // Kill its process tree too
                    Self::kill_process_tree_safe(inf_pid, self.our_pid);
                }
            }
            
            // Kill all remaining descendants of the GDB process
            if let Some(gdb) = gdb_pid {
                let gdb_children = Self::get_all_descendants(gdb);
                eprintln!("[DEBUG STOP] GDB children: {:?}", gdb_children);
                for child_pid in gdb_children {
                    if child_pid != self.our_pid {
                        eprintln!("[DEBUG STOP] Killing GDB child {}", child_pid);
                        unsafe {
                            libc::kill(child_pid as i32, libc::SIGKILL);
                        }
                    }
                }
            }
            
            // Also kill all PIDs we've tracked during the session
            for pid in &all_pids {
                if *pid != self.our_pid && *pid > 1 {
                    eprintln!("[DEBUG STOP] Killing tracked PID {}", pid);
                    unsafe {
                        libc::kill(*pid as i32, libc::SIGKILL);
                    }
                    // And their children
                    Self::kill_process_tree_safe(*pid, self.our_pid);
                }
            }
            
            // Last resort: use pgrep/pkill to find any dvop processes that are children of GDB
            if let Some(gdb) = gdb_pid {
                eprintln!("[DEBUG STOP] Using pkill as last resort for parent {}", gdb);
                // Kill any processes whose parent is GDB
                let _ = Command::new("pkill")
                    .args(["-KILL", "-P", &gdb.to_string()])
                    .output();
            }
        }

        // Kill the GDB process itself and wait for it
        if let Some(mut child) = self.process.lock().unwrap().take() {
            eprintln!("[DEBUG STOP] Killing GDB process itself");
            // Force kill if still running
            let _ = child.kill();
            let _ = child.wait();
        }
        
        // Clear state
        *self.inferior_pid.lock().unwrap() = None;
        *self.gdb_pid.lock().unwrap() = None;
        self.all_inferior_pids.lock().unwrap().clear();
        
        eprintln!("[DEBUG STOP] Stop complete");

        Ok(())
    }
    
    /// Get all descendant PIDs of a process (recursive)
    fn get_all_descendants(pid: u32) -> Vec<u32> {
        let mut all_descendants = Vec::new();
        let mut to_process = vec![pid];
        
        while let Some(current) = to_process.pop() {
            let children = Self::get_child_pids(current);
            for child in children {
                all_descendants.push(child);
                to_process.push(child);
            }
        }
        
        all_descendants
    }
    
    /// Safely kill a process tree, never killing the protected PID
    fn kill_process_tree_safe(pid: u32, protected_pid: u32) {
        if pid == protected_pid || pid <= 1 {
            return;
        }
        
        // First, find all children of this process
        let children = Self::get_child_pids(pid);
        
        // Recursively kill children first (depth-first)
        for child_pid in children {
            Self::kill_process_tree_safe(child_pid, protected_pid);
        }
        
        // Now kill this process
        #[cfg(unix)]
        unsafe {
            libc::kill(pid as i32, libc::SIGKILL);
        }
    }
    
    /// Check if a process is a descendant of another process
    fn is_descendant_of(pid: u32, ancestor_pid: u32) -> bool {
        let mut current_pid = pid;
        
        // Walk up the process tree
        for _ in 0..20 {  // Limit depth to avoid infinite loops
            if current_pid == ancestor_pid {
                return true;
            }
            if current_pid <= 1 {
                return false;
            }
            
            // Get parent PID
            let stat_path = format!("/proc/{}/stat", current_pid);
            if let Ok(stat) = std::fs::read_to_string(&stat_path) {
                if let Some(paren_end) = stat.rfind(')') {
                    let after_comm = &stat[paren_end + 2..];
                    let fields: Vec<&str> = after_comm.split_whitespace().collect();
                    if fields.len() >= 2 {
                        if let Ok(ppid) = fields[1].parse::<u32>() {
                            current_pid = ppid;
                            continue;
                        }
                    }
                }
            }
            break;
        }
        
        false
    }
    
    /// Get all child PIDs of a process by reading /proc
    fn get_child_pids(parent_pid: u32) -> Vec<u32> {
        let mut children = Vec::new();
        
        // Read /proc to find children
        if let Ok(entries) = std::fs::read_dir("/proc") {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                
                // Check if this is a PID directory
                if let Ok(pid) = name_str.parse::<u32>() {
                    // Read the stat file to get parent PID
                    let stat_path = format!("/proc/{}/stat", pid);
                    if let Ok(stat) = std::fs::read_to_string(&stat_path) {
                        // stat format: pid (comm) state ppid ...
                        // Find the closing paren and get ppid after it
                        if let Some(paren_end) = stat.rfind(')') {
                            let after_comm = &stat[paren_end + 2..];
                            let fields: Vec<&str> = after_comm.split_whitespace().collect();
                            if fields.len() >= 2 {
                                if let Ok(ppid) = fields[1].parse::<u32>() {
                                    if ppid == parent_pid {
                                        children.push(pid);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        children
    }

    /// Add a breakpoint
    pub fn add_breakpoint(&self, file: PathBuf, line: u32) -> Result<Breakpoint, String> {
        let mut id_guard = self.next_breakpoint_id.lock().unwrap();
        let id = *id_guard;
        *id_guard += 1;

        let breakpoint = Breakpoint {
            id,
            file: file.clone(),
            line,
            enabled: true,
            hit_count: 0,
            condition: None,
        };

        self.breakpoints.lock().unwrap().push(breakpoint.clone());

        // If debugger is running, add breakpoint immediately
        if self.state() != DebuggerState::Stopped {
            self.send_command(&format!("-break-insert {}:{}", file.display(), line))?;
        }

        Ok(breakpoint)
    }

    /// Remove a breakpoint
    pub fn remove_breakpoint(&self, id: u32) -> Result<(), String> {
        let mut breakpoints = self.breakpoints.lock().unwrap();
        breakpoints.retain(|bp| bp.id != id);

        // If debugger is running, remove breakpoint
        if self.state() != DebuggerState::Stopped {
            self.send_command(&format!("-break-delete {}", id))?;
        }

        Ok(())
    }

    /// Toggle breakpoint enabled/disabled
    pub fn toggle_breakpoint(&self, id: u32) -> Result<(), String> {
        let mut breakpoints = self.breakpoints.lock().unwrap();
        
        if let Some(bp) = breakpoints.iter_mut().find(|bp| bp.id == id) {
            bp.enabled = !bp.enabled;
            
            // Update in running debugger
            if self.state() != DebuggerState::Stopped {
                if bp.enabled {
                    self.send_command(&format!("-break-enable {}", id))?;
                } else {
                    self.send_command(&format!("-break-disable {}", id))?;
                }
            }
        }

        Ok(())
    }

    /// Get all breakpoints
    pub fn get_breakpoints(&self) -> Vec<Breakpoint> {
        self.breakpoints.lock().unwrap().clone()
    }

    /// Get the stack trace
    pub fn get_stack_trace(&self) -> Result<Vec<StackFrame>, String> {
        if self.state() == DebuggerState::Stopped {
            return Err("Debugger is not running".to_string());
        }

        self.send_command("-stack-list-frames")?;
        
        // Parse response (simplified - real implementation would parse MI output)
        Ok(Vec::new())
    }

    /// Get local variables
    pub fn get_locals(&self) -> Result<Vec<Variable>, String> {
        if self.state() == DebuggerState::Stopped {
            return Err("Debugger is not running".to_string());
        }

        self.send_command("-stack-list-locals --all-values")?;
        
        // Parse response (simplified - real implementation would parse MI output)
        Ok(Vec::new())
    }

    /// Evaluate an expression
    pub fn evaluate(&self, expression: &str) -> Result<String, String> {
        if self.state() == DebuggerState::Stopped {
            return Err("Debugger is not running".to_string());
        }

        self.send_command(&format!("-data-evaluate-expression \"{}\"", expression))?;
        
        // Parse response
        Ok(String::new())
    }

    /// Send a command to the debugger without waiting for response
    fn send_command_no_wait(&self, command: &str) -> Result<(), String> {
        let mut stdin_guard = self.stdin_writer.lock().unwrap();
        
        if let Some(ref mut stdin) = *stdin_guard {
            writeln!(stdin, "{}", command)
                .map_err(|e| format!("Failed to send command: {}", e))?;
            stdin.flush()
                .map_err(|e| format!("Failed to flush: {}", e))?;
            return Ok(());
        }

        Err("Debugger stdin not available".to_string())
    }

    /// Send a command to the debugger (deprecated - use send_command_no_wait)
    fn send_command(&self, command: &str) -> Result<String, String> {
        self.send_command_no_wait(command)?;
        // For now, just return empty - proper async response handling would need channels
        Ok(String::new())
    }

    /// Read any available output from GDB (non-blocking - drains from buffer)
    pub fn read_output(&self) -> Vec<String> {
        let mut buffer = self.output_buffer.lock().unwrap();
        let output = buffer.drain(..).collect();
        output
    }

    /// Get output buffer (without draining)
    pub fn get_output(&self) -> Vec<String> {
        self.output_buffer.lock().unwrap().clone()
    }

    /// Clear output buffer
    pub fn clear_output(&self) {
        self.output_buffer.lock().unwrap().clear();
    }
}

impl Default for RustDebugger {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for RustDebugger {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debugger_creation() {
        let debugger = RustDebugger::new();
        assert_eq!(debugger.state(), DebuggerState::Stopped);
    }

    #[test]
    fn test_debugger_state() {
        let debugger = RustDebugger::new();
        assert_eq!(debugger.state(), DebuggerState::Stopped);
        
        // State should not change without starting
        assert!(debugger.continue_execution().is_err());
        assert!(debugger.pause().is_err());
    }

    #[test]
    fn test_breakpoint_management() {
        let debugger = RustDebugger::new();
        
        // Add a breakpoint
        let bp = debugger.add_breakpoint(PathBuf::from("/test/file.rs"), 10).unwrap();
        assert_eq!(bp.id, 1);
        assert_eq!(bp.line, 10);
        assert!(bp.enabled);
        
        // Check breakpoints list
        let breakpoints = debugger.get_breakpoints();
        assert_eq!(breakpoints.len(), 1);
        
        // Remove breakpoint
        debugger.remove_breakpoint(1).unwrap();
        let breakpoints = debugger.get_breakpoints();
        assert!(breakpoints.is_empty());
    }

    #[test]
    fn test_debug_config() {
        let debugger = RustDebugger::new();
        
        let config = DebugConfig {
            program: PathBuf::from("/test/program"),
            args: vec!["--arg1".to_string()],
            working_dir: PathBuf::from("/test"),
            env_vars: HashMap::new(),
            stop_on_entry: true,
        };
        
        debugger.set_config(config.clone());
        
        let retrieved = debugger.get_config().unwrap();
        assert_eq!(retrieved.program, config.program);
        assert_eq!(retrieved.args, config.args);
    }

    #[test]
    fn test_default_config() {
        let config = DebugConfig::default();
        assert!(config.program.as_os_str().is_empty());
        assert!(config.args.is_empty());
        assert!(config.env_vars.is_empty());
        assert!(config.stop_on_entry);
    }

    #[test]
    fn test_debugger_default() {
        let debugger = RustDebugger::default();
        assert_eq!(debugger.state(), DebuggerState::Stopped);
    }
}
