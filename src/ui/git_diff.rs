// Git Diff UI components for Dvop
// Displays git status and diff information in the sidebar

use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, Label, ListBoxRow, Orientation, pango, DrawingArea,
};
use sourceview5::prelude::{BufferExt, ViewExt};
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::rc::Rc;
use gtk4::glib;

use super::git_diff_panel_template::GitDiffPanel;

#[derive(Clone, Debug)]
struct GitFileChange {
    path: PathBuf,
    status: GitStatus,
}

#[derive(Clone, Debug, PartialEq)]
enum GitStatus {
    Modified,
    Added,
    Deleted,
    Renamed,
    Untracked,
    Staged,
    ModifiedStaged,
}

impl GitStatus {
    fn from_git_code(staged_char: char, unstaged_char: char) -> Option<Self> {
        match (staged_char, unstaged_char) {
            // Staged changes (first character non-space)
            ('M', ' ') => Some(GitStatus::Staged),
            ('A', ' ') => Some(GitStatus::Added),
            ('D', ' ') => Some(GitStatus::Deleted),
            ('R', ' ') => Some(GitStatus::Renamed),
            // Unstaged changes (second character non-space, first space)
            (' ', 'M') => Some(GitStatus::Modified),
            (' ', 'D') => Some(GitStatus::Deleted),
            // Both staged and modified
            ('M', 'M') | ('A', 'M') | ('R', 'M') => Some(GitStatus::ModifiedStaged),
            // Untracked
            ('?', '?') => Some(GitStatus::Untracked),
            _ => None,
        }
    }
}

/// Check if a directory is a git repository
fn is_git_repository(path: &Path) -> bool {
    if !path.is_dir() {
        return false;
    }

    let git_dir = path.join(".git");
    git_dir.exists()
}

/// Get the git repository root for a given path
fn find_git_root(path: &Path) -> Option<PathBuf> {
    let mut current = path.to_path_buf();

    loop {
        if is_git_repository(&current) {
            return Some(current);
        }

        if !current.pop() {
            break;
        }
    }

    None
}

/// Get git status for the repository
fn get_git_status(repo_path: &Path) -> Vec<GitFileChange> {
    let output = Command::new("git")
        .arg("status")
        .arg("--porcelain")
        .current_dir(repo_path)
        .output();

    let mut changes = Vec::new();

    if let Ok(output) = output {
        if output.status.success() {
            let status_text = String::from_utf8_lossy(&output.stdout);
            for line in status_text.lines() {
                if line.len() < 3 {
                    continue;
                }

                // Git status porcelain format: XY filename
                // X = staged status, Y = unstaged status
                let chars: Vec<char> = line.chars().collect();
                if chars.len() < 3 {
                    continue;
                }
                
                let staged_char = chars[0];
                let unstaged_char = chars[1];
                let file_path = line[3..].trim();

                if let Some(status) = GitStatus::from_git_code(staged_char, unstaged_char) {
                    let full_path = repo_path.join(file_path);
                    changes.push(GitFileChange {
                        path: full_path,
                        status,
                    });
                }
            }
        }
    }

    changes
}

/// Get the old version of a file (from HEAD)
fn get_old_file_content(repo_path: &Path, file_path: &Path) -> Option<String> {
    let rel_path = file_path.strip_prefix(repo_path).ok()?;

    let output = Command::new("git")
        .arg("show")
        .arg(format!("HEAD:{}", rel_path.display()))
        .current_dir(repo_path)
        .output();

    if let Ok(output) = output {
        if output.status.success() {
            return Some(String::from_utf8_lossy(&output.stdout).to_string());
        }
    }

    None
}

/// Get the new version of a file (working directory)
fn get_new_file_content(file_path: &Path) -> Option<String> {
    std::fs::read_to_string(file_path).ok()
}

/// Get the staged version of a file (from index)
fn get_staged_file_content(repo_path: &Path, file_path: &Path) -> Option<String> {
    let rel_path = file_path.strip_prefix(repo_path).ok()?;

    let output = Command::new("git")
        .arg("show")
        .arg(format!(":{}", rel_path.display()))
        .current_dir(repo_path)
        .output();

    if let Ok(output) = output {
        if output.status.success() {
            return Some(String::from_utf8_lossy(&output.stdout).to_string());
        }
    }

    None
}

/// Align two text contents for side-by-side comparison
/// Returns (aligned_old, aligned_new, left_line_map, right_line_map, old_width, new_width)
fn align_diff_content(old_content: &str, new_content: &str) -> (String, String, Vec<Option<usize>>, Vec<Option<usize>>, usize, usize) {
    let old_lines: Vec<&str> = old_content.lines().collect();
    let new_lines: Vec<&str> = new_content.lines().collect();
    
    let diff_ops = compute_diff_operations(&old_lines, &new_lines);
    
    let mut aligned_old = Vec::new();
    let mut aligned_new = Vec::new();
    let mut left_line_map = Vec::new();
    let mut right_line_map = Vec::new();
    
    // Calculate max line numbers for padding
    // Use at least 5 digits to ensure proper spacing
    let max_old_line = old_lines.len();
    let max_new_line = new_lines.len();
    let old_width = max_old_line.to_string().len().max(5);
    let new_width = max_new_line.to_string().len().max(5);
    
    eprintln!("DEBUG: max_old_line={}, max_new_line={}, old_width={}, new_width={}", 
              max_old_line, max_new_line, old_width, new_width);
    
    for (old_idx, new_idx) in diff_ops {
        match (old_idx, new_idx) {
            (Some(old_i), Some(new_i)) => {
                // Both sides have content
                let old_line_num = old_i + 1;
                let new_line_num = new_i + 1;
                // Format with line number prefix: "  123  content"
                aligned_old.push(format!("{:>width$}  {}", old_line_num, old_lines[old_i], width = old_width));
                aligned_new.push(format!("{:>width$}  {}", new_line_num, new_lines[new_i], width = new_width));
                left_line_map.push(Some(old_line_num));
                right_line_map.push(Some(new_line_num));
            }
            (Some(old_i), None) => {
                // Deleted line - add blank line on right
                let old_line_num = old_i + 1;
                aligned_old.push(format!("{:>width$}  {}", old_line_num, old_lines[old_i], width = old_width));
                // Right side: spaces for line number + 2 spaces
                aligned_new.push(" ".repeat(new_width + 2));
                left_line_map.push(Some(old_line_num));
                right_line_map.push(None);
            }
            (None, Some(new_i)) => {
                // Added line - add blank line on left
                let new_line_num = new_i + 1;
                // Left side: spaces for line number + 2 spaces
                aligned_old.push(" ".repeat(old_width + 2));
                aligned_new.push(format!("{:>width$}  {}", new_line_num, new_lines[new_i], width = new_width));
                left_line_map.push(None);
                right_line_map.push(Some(new_line_num));
            }
            (None, None) => {
                // Should not happen
            }
        }
    }
    
    (aligned_old.join("\n"), aligned_new.join("\n"), left_line_map, right_line_map, old_width, new_width)
}

/// Compute the longest common subsequence (LCS) based diff
fn compute_diff_operations(old_lines: &[&str], new_lines: &[&str]) -> Vec<(Option<usize>, Option<usize>)> {
    let mut operations = Vec::new();
    let m = old_lines.len();
    let n = new_lines.len();
    
    // Build LCS table
    let mut lcs = vec![vec![0; n + 1]; m + 1];
    for i in 0..m {
        for j in 0..n {
            if old_lines[i] == new_lines[j] {
                lcs[i + 1][j + 1] = lcs[i][j] + 1;
            } else {
                lcs[i + 1][j + 1] = lcs[i + 1][j].max(lcs[i][j + 1]);
            }
        }
    }
    
    // Backtrack to find the diff
    let mut i = m;
    let mut j = n;
    
    while i > 0 || j > 0 {
        if i > 0 && j > 0 && old_lines[i - 1] == new_lines[j - 1] {
            // Lines are equal
            operations.push((Some(i - 1), Some(j - 1)));
            i -= 1;
            j -= 1;
        } else if j > 0 && (i == 0 || lcs[i][j - 1] >= lcs[i - 1][j]) {
            // Line added in new
            operations.push((None, Some(j - 1)));
            j -= 1;
        } else if i > 0 {
            // Line deleted from old
            operations.push((Some(i - 1), None));
            i -= 1;
        }
    }
    
    operations.reverse();
    operations
}

/// Make embedded line numbers invisible to selection and copy operations
fn make_line_numbers_invisible(buffer: &sourceview5::Buffer, line_map: &[Option<usize>]) {
    let tag_table = buffer.tag_table();
    
    // Create a tag that makes text invisible to selection/clipboard
    let invisible_tag = gtk4::TextTag::new(Some("line_num_invisible"));
    invisible_tag.set_invisible(true);
    
    tag_table.add(&invisible_tag);
    
    // Apply the tag to line number portions (start of each line until content)
    for (line_idx, _) in line_map.iter().enumerate() {
        if let Some(line_start) = buffer.iter_at_line(line_idx as i32) {
            let mut line_end = line_start.clone();
            line_end.forward_to_line_end();
            
            let text = buffer.text(&line_start, &line_end, false);
            let text_str = text.as_str();
            
            // Find the position of the first occurrence of double space
            // This marks the end of line number section
            if let Some(pos) = text_str.find("  ") {
                // Apply invisible tag from start of line to end of line number + 2 spaces
                let mut num_end = line_start.clone();
                num_end.forward_chars((pos + 2) as i32);
                buffer.apply_tag(&invisible_tag, &line_start, &num_end);
            }
        }
    }
}

/// Apply diff highlighting to text buffers
fn apply_diff_highlighting(
    left_buffer: &sourceview5::Buffer,
    right_buffer: &sourceview5::Buffer,
    old_content: &str,
    new_content: &str,
    old_width: usize,
    new_width: usize,
) {
    let old_lines: Vec<&str> = old_content.lines().collect();
    let new_lines: Vec<&str> = new_content.lines().collect();
    
    // Helper function to strip line number prefix from a line
    let strip_line_number = |line: &str, width: usize| -> String {
        // Line format is "{:>width$}  content"
        // Skip exactly width + 2 characters (line number + two spaces)
        let skip_chars = width + 2;
        if line.len() > skip_chars {
            line[skip_chars..].to_string()
        } else {
            String::new()
        }
    };
    
    // Strip line numbers for comparison
    let old_lines_stripped: Vec<String> = old_lines.iter().map(|line| strip_line_number(line, old_width)).collect();
    let new_lines_stripped: Vec<String> = new_lines.iter().map(|line| strip_line_number(line, new_width)).collect();
    
    // If the old file is empty (new file), don't highlight anything
    if old_lines_stripped.is_empty() || (old_lines_stripped.len() == 1 && old_lines_stripped[0].is_empty()) {
        return;
    }
    
    // If the new file is empty (deleted file), don't highlight anything
    if new_lines_stripped.is_empty() || (new_lines_stripped.len() == 1 && new_lines_stripped[0].is_empty()) {
        return;
    }
    
    // Use RGBA colors with alpha channel for better theme adaptation
    // These colors will blend with the background
    let delete_color = gtk4::gdk::RGBA::new(1.0, 0.0, 0.0, 0.15);  // Red with 15% opacity
    let add_color = gtk4::gdk::RGBA::new(0.0, 1.0, 0.0, 0.15);     // Green with 15% opacity
    let modify_color = gtk4::gdk::RGBA::new(1.0, 1.0, 0.0, 0.15);  // Yellow with 15% opacity
    
    // Create text tags for highlighting
    let left_tag_table = left_buffer.tag_table();
    let delete_tag = gtk4::TextTag::new(Some("delete"));
    delete_tag.set_background_rgba(Some(&delete_color));
    left_tag_table.add(&delete_tag);
    
    let right_tag_table = right_buffer.tag_table();
    let add_tag = gtk4::TextTag::new(Some("add"));
    add_tag.set_background_rgba(Some(&add_color));
    right_tag_table.add(&add_tag);
    
    let left_modify_tag = gtk4::TextTag::new(Some("modify_old"));
    left_modify_tag.set_background_rgba(Some(&modify_color));
    left_tag_table.add(&left_modify_tag);
    
    let right_modify_tag = gtk4::TextTag::new(Some("modify_new"));
    right_modify_tag.set_background_rgba(Some(&modify_color));
    right_tag_table.add(&right_modify_tag);
    
    // Since content is already aligned, compare line by line using stripped content
    let max_lines = old_lines_stripped.len().max(new_lines_stripped.len());
    
    for i in 0..max_lines {
        let old_line = old_lines_stripped.get(i).map(|s| s.as_str()).unwrap_or("");
        let new_line = new_lines_stripped.get(i).map(|s| s.as_str()).unwrap_or("");
        
        if old_line.is_empty() && !new_line.is_empty() {
            // Added line (blank on left, content on right)
            if let Some(right_start) = right_buffer.iter_at_line(i as i32) {
                let mut right_end = right_start.clone();
                right_end.forward_to_line_end();
                right_buffer.apply_tag(&add_tag, &right_start, &right_end);
            }
        } else if !old_line.is_empty() && new_line.is_empty() {
            // Deleted line (content on left, blank on right)
            if let Some(left_start) = left_buffer.iter_at_line(i as i32) {
                let mut left_end = left_start.clone();
                left_end.forward_to_line_end();
                left_buffer.apply_tag(&delete_tag, &left_start, &left_end);
            }
        } else if old_line != new_line && !old_line.is_empty() && !new_line.is_empty() {
            // Modified line (different content on both sides)
            if let Some(left_start) = left_buffer.iter_at_line(i as i32) {
                let mut left_end = left_start.clone();
                left_end.forward_to_line_end();
                left_buffer.apply_tag(&left_modify_tag, &left_start, &left_end);
            }
            
            if let Some(right_start) = right_buffer.iter_at_line(i as i32) {
                let mut right_end = right_start.clone();
                right_end.forward_to_line_end();
                right_buffer.apply_tag(&right_modify_tag, &right_start, &right_end);
            }
        }
        // else: lines are identical or both empty, no highlighting
    }
}

/// Get the current branch name
fn get_current_branch(repo_path: &Path) -> Option<String> {
    let output = Command::new("git")
        .arg("rev-parse")
        .arg("--abbrev-ref")
        .arg("HEAD")
        .current_dir(repo_path)
        .output();

    if let Ok(output) = output {
        if output.status.success() {
            let branch = String::from_utf8_lossy(&output.stdout);
            return Some(branch.trim().to_string());
        }
    }

    None
}

/// Stage a file (git add)
fn stage_file(repo_path: &Path, file_path: &Path) -> Result<(), String> {
    let rel_path = file_path
        .strip_prefix(repo_path)
        .map_err(|e| format!("Invalid path: {}", e))?;

    let output = Command::new("git")
        .arg("add")
        .arg(rel_path)
        .current_dir(repo_path)
        .output()
        .map_err(|e| format!("Failed to run git: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

/// Unstage a file (git reset)
fn unstage_file(repo_path: &Path, file_path: &Path) -> Result<(), String> {
    let rel_path = file_path
        .strip_prefix(repo_path)
        .map_err(|e| format!("Invalid path: {}", e))?;

    let output = Command::new("git")
        .arg("reset")
        .arg("HEAD")
        .arg(rel_path)
        .current_dir(repo_path)
        .output()
        .map_err(|e| format!("Failed to run git: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

/// Discard changes for a file
fn discard_changes(repo_path: &Path, file_path: &Path) -> Result<(), String> {
    let rel_path = file_path
        .strip_prefix(repo_path)
        .map_err(|e| format!("Invalid path: {}", e))?;

    let output = Command::new("git")
        .arg("checkout")
        .arg("--")
        .arg(rel_path)
        .current_dir(repo_path)
        .output()
        .map_err(|e| format!("Failed to run git: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

/// Commit staged changes with a message
fn commit_changes(repo_path: &Path, message: &str) -> Result<(), String> {
    if message.trim().is_empty() {
        return Err("Commit message cannot be empty".to_string());
    }

    let output = Command::new("git")
        .arg("commit")
        .arg("-m")
        .arg(message)
        .current_dir(repo_path)
        .output()
        .map_err(|e| format!("Failed to run git: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

/// Push commits to remote
fn push_changes(repo_path: &Path) -> Result<(), String> {
    let output = Command::new("git")
        .arg("push")
        .current_dir(repo_path)
        .output()
        .map_err(|e| format!("Failed to run git: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

/// Count the number of unpushed commits
fn count_unpushed_commits(repo_path: &Path) -> usize {
    let output = Command::new("git")
        .arg("rev-list")
        .arg("--count")
        .arg("@{u}..")
        .current_dir(repo_path)
        .output();

    if let Ok(output) = output {
        if output.status.success() {
            let count_str = String::from_utf8_lossy(&output.stdout);
            return count_str.trim().parse().unwrap_or(0);
        }
    }
    
    0
}

/// Count the number of commits to pull from remote
fn count_incoming_commits(repo_path: &Path) -> usize {
    // First, fetch to get latest remote info (silently)
    let _ = Command::new("git")
        .arg("fetch")
        .current_dir(repo_path)
        .output();

    let output = Command::new("git")
        .arg("rev-list")
        .arg("--count")
        .arg("..@{u}")
        .current_dir(repo_path)
        .output();

    if let Ok(output) = output {
        if output.status.success() {
            let count_str = String::from_utf8_lossy(&output.stdout);
            return count_str.trim().parse().unwrap_or(0);
        }
    }
    
    0
}

/// Pull commits from remote
fn pull_changes(repo_path: &Path) -> Result<(), String> {
    let output = Command::new("git")
        .arg("pull")
        .current_dir(repo_path)
        .output()
        .map_err(|e| format!("Failed to run git: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

#[derive(Clone, Debug)]
struct BranchInfo {
    name: String,
    is_current: bool,
    is_remote: bool,
}

/// Get all branches (local and remote)
fn get_all_branches(repo_path: &Path) -> Vec<BranchInfo> {
    let mut branches = Vec::new();

    // Get local branches
    let output = Command::new("git")
        .arg("branch")
        .arg("--list")
        .current_dir(repo_path)
        .output();

    if let Ok(output) = output {
        if output.status.success() {
            let branch_text = String::from_utf8_lossy(&output.stdout);
            for line in branch_text.lines() {
                let is_current = line.starts_with("* ");
                let name = line.trim_start_matches("* ").trim().to_string();
                if !name.is_empty() {
                    branches.push(BranchInfo {
                        name,
                        is_current,
                        is_remote: false,
                    });
                }
            }
        }
    }

    // Get remote branches
    let output = Command::new("git")
        .arg("branch")
        .arg("-r")
        .current_dir(repo_path)
        .output();

    if let Ok(output) = output {
        if output.status.success() {
            let branch_text = String::from_utf8_lossy(&output.stdout);
            for line in branch_text.lines() {
                let name = line.trim().to_string();
                // Skip HEAD pointer
                if !name.is_empty() && !name.contains("HEAD ->") {
                    branches.push(BranchInfo {
                        name,
                        is_current: false,
                        is_remote: true,
                    });
                }
            }
        }
    }

    branches
}

/// Switch to a branch
fn switch_branch(repo_path: &Path, branch_name: &str, is_remote: bool) -> Result<(), String> {
    if is_remote {
        // For remote branches, create a local tracking branch
        // Extract the branch name without the remote prefix (e.g., "origin/feature" -> "feature")
        let local_name = if let Some(pos) = branch_name.find('/') {
            &branch_name[pos + 1..]
        } else {
            branch_name
        };

        // First check if local branch already exists
        let check_output = Command::new("git")
            .arg("rev-parse")
            .arg("--verify")
            .arg(local_name)
            .current_dir(repo_path)
            .output()
            .map_err(|e| format!("Failed to run git: {}", e))?;

        if check_output.status.success() {
            // Local branch exists, just switch to it
            switch_branch(repo_path, local_name, false)
        } else {
            // Local branch doesn't exist, create and switch to a new local branch tracking the remote
            let output = Command::new("git")
                .arg("checkout")
                .arg("-b")
                .arg(local_name)
                .arg(branch_name)
                .current_dir(repo_path)
                .output()
                .map_err(|e| format!("Failed to run git: {}", e))?;

            if output.status.success() {
                Ok(())
            } else {
                Err(String::from_utf8_lossy(&output.stderr).to_string())
            }
        }
    } else {
        // For local branches, just checkout
        let output = Command::new("git")
            .arg("checkout")
            .arg(branch_name)
            .current_dir(repo_path)
            .output()
            .map_err(|e| format!("Failed to run git: {}", e))?;

        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).to_string())
        }
    }
}

/// Set up a copy handler that strips line numbers from copied text
fn setup_copy_handler(view: &sourceview5::View, buffer: &sourceview5::Buffer) {
    let buffer_clone = buffer.clone();
    
    // Use key event controller to intercept copy operations
    let key_controller = gtk4::EventControllerKey::new();
    
    key_controller.connect_key_pressed(move |_, key, _, modifier| {
        // Check for Ctrl+C or Ctrl+Insert
        let is_ctrl = modifier.contains(gtk4::gdk::ModifierType::CONTROL_MASK);
        let is_copy = (key == gtk4::gdk::Key::c || key == gtk4::gdk::Key::C || 
                       key == gtk4::gdk::Key::Insert) && is_ctrl;
        
        if is_copy {
            // Get the selected text
            if let Some((start, end)) = buffer_clone.selection_bounds() {
                let text = buffer_clone.text(&start, &end, false);
                let text_str = text.as_str();
                
                // Strip line numbers from each line
                // Line format is "{:>width$}  content" - skip spaces, digits, then 2 more spaces
                let stripped_lines: Vec<String> = text_str.lines().map(|line| {
                    let chars: Vec<char> = line.chars().collect();
                    let mut i = 0;
                    
                    // Skip leading spaces and digits
                    while i < chars.len() && (chars[i].is_whitespace() || chars[i].is_ascii_digit()) {
                        i += 1;
                    }
                    
                    if i < line.len() {
                        line[i..].to_string()
                    } else {
                        String::new()
                    }
                }).collect();
                
                let stripped_text = stripped_lines.join("\n");
                
                // Set the clipboard with stripped text
                if let Some(display) = gtk4::gdk::Display::default() {
                    let clipboard = display.clipboard();
                    clipboard.set_text(&stripped_text);
                }
                
                // Prevent default copy behavior
                return gtk4::glib::Propagation::Stop;
            }
        }
        
        gtk4::glib::Propagation::Proceed
    });
    
    view.add_controller(key_controller);
}

/// Represents a line change type for minimap display
#[derive(Clone, Debug, PartialEq)]
enum LineChangeType {
    Unchanged,
    Added,
    Deleted,
    Modified,
}

/// Compute line changes for minimap display
fn compute_line_changes(old_content: &str, new_content: &str, old_width: usize, new_width: usize) -> Vec<LineChangeType> {
    let old_lines: Vec<&str> = old_content.lines().collect();
    let new_lines: Vec<&str> = new_content.lines().collect();
    
    // Helper function to strip line number prefix from a line
    let strip_line_number = |line: &str, width: usize| -> String {
        let skip_chars = width + 2;
        if line.len() > skip_chars {
            line[skip_chars..].to_string()
        } else {
            String::new()
        }
    };
    
    // Strip line numbers for comparison
    let old_lines_stripped: Vec<String> = old_lines.iter().map(|line| strip_line_number(line, old_width)).collect();
    let new_lines_stripped: Vec<String> = new_lines.iter().map(|line| strip_line_number(line, new_width)).collect();
    
    let mut changes = Vec::new();
    let max_lines = old_lines_stripped.len().max(new_lines_stripped.len());
    
    for i in 0..max_lines {
        let old_line = old_lines_stripped.get(i).map(|s| s.as_str()).unwrap_or("");
        let new_line = new_lines_stripped.get(i).map(|s| s.as_str()).unwrap_or("");
        
        if old_line.is_empty() && !new_line.is_empty() {
            changes.push(LineChangeType::Added);
        } else if !old_line.is_empty() && new_line.is_empty() {
            changes.push(LineChangeType::Deleted);
        } else if old_line != new_line && !old_line.is_empty() && !new_line.is_empty() {
            changes.push(LineChangeType::Modified);
        } else {
            changes.push(LineChangeType::Unchanged);
        }
    }
    
    changes
}

/// Set up minimap drawing and click handler
fn setup_minimap_drawing(
    minimap: &DrawingArea,
    scrolled: &gtk4::ScrolledWindow,
    buffer: &sourceview5::Buffer,
    line_changes: &Rc<RefCell<Vec<LineChangeType>>>,
    _is_left: bool,
) {
    let scrolled_weak = scrolled.downgrade();
    let buffer_weak = buffer.downgrade();
    let line_changes_clone = line_changes.clone();
    
    // Draw the minimap
    minimap.set_draw_func(move |widget, cr, width, height| {
        let Some(scrolled) = scrolled_weak.upgrade() else { return; };
        let Some(buffer) = buffer_weak.upgrade() else { return; };
        
        // Get theme colors from the widget's style context
        let style_context = widget.style_context();
        let color = style_context.color();
        let bg_color = style_context.lookup_color("view_bg_color")
            .or_else(|| style_context.lookup_color("theme_bg_color"))
            .unwrap_or_else(|| {
                // Fallback: determine if dark or light theme based on text color luminance
                let luminance = color.red() * 0.299 + color.green() * 0.587 + color.blue() * 0.114;
                if luminance > 0.5 {
                    // Light theme
                    gtk4::gdk::RGBA::new(0.95, 0.95, 0.95, 1.0)
                } else {
                    // Dark theme
                    gtk4::gdk::RGBA::new(0.2, 0.2, 0.2, 1.0)
                }
            });
        
        // Background with theme color
        cr.set_source_rgba(bg_color.red() as f64, bg_color.green() as f64, bg_color.blue() as f64, bg_color.alpha() as f64);
        let _ = cr.paint();
        
        let line_count = buffer.line_count() as usize;
        if line_count == 0 {
            return;
        }
        
        let changes = line_changes_clone.borrow();
        if changes.is_empty() {
            return;
        }
        
        let line_height = height as f64 / line_count as f64;
        
        // Determine if we're in dark mode
        let is_dark_theme = bg_color.red() < 0.5;
        
        // Draw colored bars for each line
        for (i, change) in changes.iter().enumerate() {
            let y = i as f64 * line_height;
            
            match change {
                LineChangeType::Added => {
                    // Green for added lines (muted colors)
                    if is_dark_theme {
                        cr.set_source_rgba(0.3, 0.55, 0.3, 0.7);
                    } else {
                        cr.set_source_rgba(0.4, 0.65, 0.4, 0.7);
                    }
                }
                LineChangeType::Deleted => {
                    // Red for deleted lines (muted colors)
                    if is_dark_theme {
                        cr.set_source_rgba(0.65, 0.3, 0.3, 0.7);
                    } else {
                        cr.set_source_rgba(0.75, 0.35, 0.35, 0.7);
                    }
                }
                LineChangeType::Modified => {
                    // Yellow/Orange for modified lines (muted colors)
                    if is_dark_theme {
                        cr.set_source_rgba(0.65, 0.55, 0.3, 0.7);
                    } else {
                        cr.set_source_rgba(0.75, 0.6, 0.35, 0.7);
                    }
                }
                LineChangeType::Unchanged => {
                    // Slightly different shade for unchanged lines
                    if is_dark_theme {
                        cr.set_source_rgba(0.25, 0.25, 0.25, 0.5);
                    } else {
                        cr.set_source_rgba(0.85, 0.85, 0.85, 0.5);
                    }
                }
            }
            
            let _ = cr.rectangle(0.0, y, width as f64, line_height.max(1.0));
            let _ = cr.fill();
        }
        
        // Draw viewport indicator (showing visible area)
        let vadj = scrolled.vadjustment();
        let visible_start = vadj.value();
        let visible_size = vadj.page_size();
        let total_size = vadj.upper();
        
        if total_size > 0.0 {
            let viewport_start = (visible_start / total_size) * height as f64;
            let viewport_height = (visible_size / total_size) * height as f64;
            
            // Semi-transparent overlay for viewport (adapted to theme)
            if is_dark_theme {
                cr.set_source_rgba(1.0, 1.0, 1.0, 0.15);
            } else {
                cr.set_source_rgba(0.0, 0.0, 0.0, 0.15);
            }
            let _ = cr.rectangle(0.0, viewport_start, width as f64, viewport_height);
            let _ = cr.fill();
            
            // Border for viewport
            if is_dark_theme {
                cr.set_source_rgba(1.0, 1.0, 1.0, 0.4);
            } else {
                cr.set_source_rgba(0.0, 0.0, 0.0, 0.4);
            }
            cr.set_line_width(1.0);
            let _ = cr.rectangle(0.0, viewport_start, width as f64, viewport_height);
            let _ = cr.stroke();
        }
    });
    
    // Add drag handler for continuous scrolling
    let scrolled_weak2 = scrolled.downgrade();
    let minimap_clone = minimap.clone();
    let drag_gesture = gtk4::GestureDrag::new();
    
    // Helper closure to handle scrolling
    let scroll_to_position = move |y: f64| {
        let Some(scrolled) = scrolled_weak2.upgrade() else { return; };
        
        let vadj = scrolled.vadjustment();
        let total_height = vadj.upper();
        let minimap_height = y;
        
        // Get minimap widget height
        let widget_height = minimap_clone.height() as f64;
        
        if widget_height > 0.0 {
            // Calculate scroll position
            let scroll_position = (minimap_height / widget_height) * total_height;
            let scroll_centered = scroll_position - vadj.page_size() / 2.0;
            
            // Clamp to valid range
            let clamped = scroll_centered.max(0.0).min(total_height - vadj.page_size());
            vadj.set_value(clamped);
        }
    };
    
    let scroll_to_position_rc = Rc::new(RefCell::new(scroll_to_position));
    let scroll_to_position_clone = scroll_to_position_rc.clone();
    
    // On drag begin, scroll to initial position
    drag_gesture.connect_drag_begin(move |_, _start_x, start_y| {
        let scroll_fn = scroll_to_position_clone.borrow();
        scroll_fn(start_y);
    });
    
    // On drag update, continuously scroll
    let scroll_to_position_clone2 = scroll_to_position_rc.clone();
    drag_gesture.connect_drag_update(move |gesture, _offset_x, offset_y| {
        if let Some((_start_x, start_y)) = gesture.start_point() {
            let current_y = start_y + offset_y;
            let scroll_fn = scroll_to_position_clone2.borrow();
            scroll_fn(current_y);
        }
    });
    
    minimap.add_controller(drag_gesture);
}

/// Helper function to create a diff tab
fn create_diff_tab(
    editor_notebook: &gtk4::Notebook,
    active_tab_path: &Rc<RefCell<Option<PathBuf>>>,
    file_path: &Path,
    _repo: &Path,
    old_content: &str,
    new_content: &str,
    tab_title: &str,
) {
    // Align the content for side-by-side comparison
    let (aligned_old, aligned_new, left_line_map, right_line_map, old_width, new_width) = align_diff_content(old_content, new_content);
    
    // Create a horizontal paned widget for side-by-side view
    let paned = gtk4::Paned::new(gtk4::Orientation::Horizontal);
    paned.set_wide_handle(true);
    paned.set_shrink_start_child(false);
    paned.set_shrink_end_child(false);
    
    // Track line change types for minimap
    let line_changes = Rc::new(RefCell::new(Vec::new()));
    
    // Create left side (old version)
    let (left_view, left_buffer) = crate::syntax::create_source_view();
    left_buffer.set_text(&aligned_old);
    left_view.set_editable(false);
    left_view.set_cursor_visible(false);
    left_view.set_show_line_numbers(false); // Line numbers are embedded in text
    
    // Set up copy handler to strip line numbers
    setup_copy_handler(&left_view, &left_buffer);
    
    let left_scrolled = crate::syntax::create_source_view_scrolled(&left_view);
    left_scrolled.set_vexpand(true);
    left_scrolled.set_hexpand(true);
    
    // Create left header
    let left_header = Label::new(Some("Original"));
    left_header.set_halign(gtk4::Align::Start);
    left_header.add_css_class("heading");
    left_header.set_margin_start(10);
    left_header.set_margin_top(5);
    left_header.set_margin_bottom(5);
    
    // Create minimap container (left side with minimap + content)
    let left_container = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    left_container.set_hexpand(true);
    
    // Create minimap for left side
    let left_minimap = DrawingArea::new();
    left_minimap.set_width_request(30);
    left_minimap.set_vexpand(true);
    left_minimap.set_valign(gtk4::Align::Fill);
    
    let left_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    left_box.set_hexpand(true);
    left_box.append(&left_header);
    
    let left_content_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    left_content_box.set_hexpand(true);
    left_content_box.append(&left_scrolled);
    left_content_box.append(&left_minimap);
    
    left_box.append(&left_content_box);
    
    // Create right side (new version)
    let (right_view, right_buffer) = crate::syntax::create_source_view();
    right_buffer.set_text(&aligned_new);
    right_view.set_editable(false);
    right_view.set_cursor_visible(false);
    right_view.set_show_line_numbers(false); // Line numbers are embedded in text
    
    // Set up copy handler to strip line numbers
    setup_copy_handler(&right_view, &right_buffer);
    
    let right_scrolled = crate::syntax::create_source_view_scrolled(&right_view);
    right_scrolled.set_vexpand(true);
    right_scrolled.set_hexpand(true);
    
    // Create right header
    let right_header = Label::new(Some("Modified"));
    right_header.set_halign(gtk4::Align::Start);
    right_header.add_css_class("heading");
    right_header.set_margin_start(10);
    right_header.set_margin_top(5);
    right_header.set_margin_bottom(5);
    
    // Create minimap for right side
    let right_minimap = DrawingArea::new();
    right_minimap.set_width_request(30);
    right_minimap.set_vexpand(true);
    right_minimap.set_valign(gtk4::Align::Fill);
    
    let right_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    right_box.set_hexpand(true);
    right_box.append(&right_header);
    
    let right_content_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    right_content_box.set_hexpand(true);
    right_content_box.append(&right_scrolled);
    right_content_box.append(&right_minimap);
    
    right_box.append(&right_content_box);
    
    // Detect language from file extension
    let lang_manager = sourceview5::LanguageManager::new();
    if let Some(_extension) = file_path.extension().and_then(|e| e.to_str()) {
        if let Some(lang) = lang_manager.guess_language(Some(file_path), None) {
            left_buffer.set_language(Some(&lang));
            right_buffer.set_language(Some(&lang));
        }
    }
    
    // Apply diff highlighting using aligned content and collect line changes
    let changes = compute_line_changes(&aligned_old, &aligned_new, old_width, new_width);
    *line_changes.borrow_mut() = changes.clone();
    apply_diff_highlighting(&left_buffer, &right_buffer, &aligned_old, &aligned_new, old_width, new_width);
    
    // Make line numbers non-selectable (invisible to selection/copy)
    make_line_numbers_invisible(&left_buffer, &left_line_map);
    make_line_numbers_invisible(&right_buffer, &right_line_map);
    
    // Set up scroll synchronization
    let left_vadj = left_scrolled.vadjustment();
    let right_vadj = right_scrolled.vadjustment();
    
    let right_vadj_clone = right_vadj.clone();
    let left_minimap_clone = left_minimap.clone();
    let right_minimap_clone = right_minimap.clone();
    left_vadj.connect_value_changed(move |adj| {
        right_vadj_clone.set_value(adj.value());
        left_minimap_clone.queue_draw();
        right_minimap_clone.queue_draw();
    });
    
    let left_vadj_clone = left_vadj.clone();
    let left_minimap_clone2 = left_minimap.clone();
    let right_minimap_clone2 = right_minimap.clone();
    right_vadj.connect_value_changed(move |adj| {
        left_vadj_clone.set_value(adj.value());
        left_minimap_clone2.queue_draw();
        right_minimap_clone2.queue_draw();
    });
    
    // Set up minimap drawing
    setup_minimap_drawing(&left_minimap, &left_scrolled, &left_buffer, &line_changes, true);
    setup_minimap_drawing(&right_minimap, &right_scrolled, &right_buffer, &line_changes, false);
    
    // Add both sides to the paned widget
    paned.set_start_child(Some(&left_box));
    paned.set_end_child(Some(&right_box));
    paned.set_resize_start_child(true);
    paned.set_resize_end_child(true);
    paned.set_shrink_start_child(false);
    paned.set_shrink_end_child(false);
    
    // Set initial position to middle after the paned is realized
    paned.connect_realize(|p| {
        let width = p.width();
        if width > 0 {
            p.set_position(width / 2);
        }
    });
    
    // Create tab widget
    let (tab_widget, _tab_label, tab_close_button) = crate::ui::create_tab_widget(tab_title);
    
    // Add the tab
    let page_num = editor_notebook.append_page(&paned, Some(&tab_widget));
    editor_notebook.set_tab_label(&paned, Some(&tab_widget));
    
    // Set up middle-click to close
    crate::ui::setup_tab_middle_click(&tab_widget, &tab_close_button);
    
    // Close button handler
    let notebook_clone = editor_notebook.clone();
    tab_close_button.connect_clicked(move |_| {
        if let Some(page) = notebook_clone.nth_page(Some(page_num)) {
            let page_index = notebook_clone.page_num(&page);
            if let Some(idx) = page_index {
                notebook_clone.remove_page(Some(idx));
            }
        }
    });
        
    // Focus the new tab
    editor_notebook.set_current_page(Some(page_num));
    
    // Don't track this in file_path_manager since it's not a real file
    *active_tab_path.borrow_mut() = None;
}

/// Creates the git diff panel UI (for embedding in the activity bar sidebar)
/// Returns the panel container that can be added to the sidebar stack
pub fn create_git_diff_panel(
    parent_window: &impl IsA<gtk4::ApplicationWindow>,
    current_dir: &Rc<RefCell<PathBuf>>,
    editor_notebook: &gtk4::Notebook,
    file_path_manager: &Rc<RefCell<std::collections::HashMap<u32, PathBuf>>>,
    active_tab_path: &Rc<RefCell<Option<PathBuf>>>,
) -> GtkBox {
    // Create the template-based panel
    let panel = GitDiffPanel::new();
    
    // Get references to widgets
    let branch_button = panel.branch_button();
    let refresh_button = panel.refresh_button();
    let stage_all_button = panel.stage_all_button();
    let staged_files_list = panel.staged_files_list();
    let files_list = panel.files_list();
    let commit_message_view = panel.commit_message_view();
    let commit_button = panel.commit_button();

    // Set up placeholder text behavior for commit message
    let buffer = commit_message_view.buffer();
    
    // Create placeholder styling
    let placeholder_tag = buffer.create_tag(
        Some("placeholder"),
        &[("foreground", &"gray"), ("style", &pango::Style::Italic)],
    ).expect("Failed to create placeholder tag");
    
    // Helper to show placeholder
    let show_placeholder = {
        let buffer = buffer.clone();
        let placeholder_tag = placeholder_tag.clone();
        move || {
            let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false);
            if text.is_empty() {
                buffer.set_text("Commit message");
                let start = buffer.start_iter();
                let end = buffer.end_iter();
                buffer.apply_tag(&placeholder_tag, &start, &end);
            }
        }
    };
    
    // Helper to hide placeholder
    let hide_placeholder = {
        let buffer = buffer.clone();
        move || {
            let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false);
            if text == "Commit message" {
                buffer.set_text("");
            }
        }
    };
    
    // Initialize with placeholder
    show_placeholder();
    
    // Use focus controller for GTK4
    let focus_controller = gtk4::EventControllerFocus::new();
    
    let hide_placeholder_clone = hide_placeholder.clone();
    focus_controller.connect_enter(move |_| {
        hide_placeholder_clone();
    });
    
    let show_placeholder_clone = show_placeholder.clone();
    let buffer_clone = buffer.clone();
    focus_controller.connect_leave(move |_| {
        let text = buffer_clone.text(&buffer_clone.start_iter(), &buffer_clone.end_iter(), false);
        if text.is_empty() {
            show_placeholder_clone();
        }
    });
    
    commit_message_view.add_controller(focus_controller);

    // State for the panel
    let repo_path_rc: Rc<RefCell<Option<PathBuf>>> = Rc::new(RefCell::new(None));
    let changes_rc: Rc<RefCell<Vec<GitFileChange>>> = Rc::new(RefCell::new(Vec::new()));
    let action_group_rc = Rc::new(RefCell::new(gtk4::gio::SimpleActionGroup::new()));
    
    // Clone widgets early for use in branch actions
    let refresh_button_for_actions = refresh_button.clone();
    let stage_all_button_for_actions = stage_all_button.clone();
    let branch_button_for_actions = branch_button.clone();
    let commit_button_for_actions = commit_button.clone();

    // Function to update the git status
    let update_git_status = {
        let current_dir = current_dir.clone();
        let repo_path_rc = repo_path_rc.clone();
        let changes_rc = changes_rc.clone();
        let branch_button = branch_button.clone();
        let staged_files_list = staged_files_list.clone();
        let files_list = files_list.clone();
        let commit_button = commit_button.clone();
        let action_group_rc = action_group_rc.clone();
        let refresh_button_clone = refresh_button_for_actions.clone();

        Rc::new(move || {
            // Clear previous content
            while let Some(child) = staged_files_list.first_child() {
                staged_files_list.remove(&child);
            }
            while let Some(child) = files_list.first_child() {
                files_list.remove(&child);
            }

            let dir = current_dir.borrow().clone();

            // Find git repository
            let repo_path = find_git_root(&dir);

            if let Some(repo) = repo_path {
                *repo_path_rc.borrow_mut() = Some(repo.clone());

                // Get branch name and update button
                if let Some(branch) = get_current_branch(&repo) {
                    branch_button.set_label(&format!("⎇ {}", branch));
                } else {
                    branch_button.set_label("⎇ (unknown)");
                }

                // Populate branch menu
                let branches = get_all_branches(&repo);
                let menu = gtk4::gio::Menu::new();
                
                // Create new action group for this update
                let new_action_group = gtk4::gio::SimpleActionGroup::new();
                
                // Add local branches section
                let local_section = gtk4::gio::Menu::new();
                let mut has_local = false;
                for branch_info in branches.iter().filter(|b| !b.is_remote) {
                    let label = if branch_info.is_current {
                        format!("● {}", branch_info.name)
                    } else {
                        format!("  {}", branch_info.name)
                    };
                    let action_name = format!("switch-local.{}", branch_info.name.replace('/', "-"));
                    local_section.append(Some(&label), Some(&format!("branch.{}", action_name)));
                    
                    // Create action for this branch
                    let action = gtk4::gio::SimpleAction::new(&action_name, None);
                    let branch_name = branch_info.name.clone();
                    let repo_path_for_action = repo_path_rc.clone();
                    let refresh_btn = refresh_button_clone.clone();
                    let branch_btn = branch_button_for_actions.clone();
                    let commit_btn = commit_button_for_actions.clone();
                    let stage_all_btn = stage_all_button_for_actions.clone();
                    
                    action.connect_activate(move |_, _| {
                        if let Some(repo) = repo_path_for_action.borrow().as_ref() {
                            // Disable UI elements during branch switch
                            branch_btn.set_sensitive(false);
                            commit_btn.set_sensitive(false);
                            stage_all_btn.set_sensitive(false);
                            refresh_btn.set_sensitive(false);
                            
                            crate::status_log::log_info(&format!("Switching to branch '{}'...", branch_name));
                            match switch_branch(repo, &branch_name, false) {
                                Ok(()) => {
                                    crate::status_log::log_success(&format!("Switched to branch '{}'", branch_name));
                                    // Trigger immediate refresh by clicking the refresh button
                                    let btn = refresh_btn.clone();
                                    let branch_btn_clone = branch_btn.clone();
                                    let commit_btn_clone = commit_btn.clone();
                                    let stage_all_btn_clone = stage_all_btn.clone();
                                    let refresh_btn_clone = refresh_btn.clone();
                                    
                                    glib::idle_add_local_once(move || {
                                        btn.emit_clicked();
                                        // Re-enable UI elements after refresh
                                        glib::timeout_add_local_once(std::time::Duration::from_millis(100), move || {
                                            branch_btn_clone.set_sensitive(true);
                                            commit_btn_clone.set_sensitive(true);
                                            stage_all_btn_clone.set_sensitive(true);
                                            refresh_btn_clone.set_sensitive(true);
                                        });
                                    });
                                }
                                Err(e) => {
                                    let error_msg = format!("Failed to switch branch: {}", e);
                                    crate::status_log::log_error(&error_msg);
                                    // Re-enable UI elements on error
                                    branch_btn.set_sensitive(true);
                                    commit_btn.set_sensitive(true);
                                    stage_all_btn.set_sensitive(true);
                                    refresh_btn.set_sensitive(true);
                                }
                            }
                        }
                    });
                    
                    new_action_group.add_action(&action);
                    has_local = true;
                }
                if has_local {
                    menu.append_section(Some("Local Branches"), &local_section);
                }

                // Add remote branches section
                let remote_section = gtk4::gio::Menu::new();
                let mut has_remote = false;
                for branch_info in branches.iter().filter(|b| b.is_remote) {
                    let action_name = format!("switch-remote.{}", branch_info.name.replace('/', "-"));
                    remote_section.append(Some(&branch_info.name), Some(&format!("branch.{}", action_name)));
                    
                    // Create action for this remote branch
                    let action = gtk4::gio::SimpleAction::new(&action_name, None);
                    let branch_name = branch_info.name.clone();
                    let repo_path_for_action = repo_path_rc.clone();
                    let refresh_btn = refresh_button_clone.clone();
                    let branch_btn = branch_button_for_actions.clone();
                    let commit_btn = commit_button_for_actions.clone();
                    let stage_all_btn = stage_all_button_for_actions.clone();
                    
                    action.connect_activate(move |_, _| {
                        if let Some(repo) = repo_path_for_action.borrow().as_ref() {
                            // Disable UI elements during branch switch
                            branch_btn.set_sensitive(false);
                            commit_btn.set_sensitive(false);
                            stage_all_btn.set_sensitive(false);
                            refresh_btn.set_sensitive(false);
                            
                            // Extract local name for better messaging
                            let local_name = if let Some(pos) = branch_name.find('/') {
                                &branch_name[pos + 1..]
                            } else {
                                branch_name.as_str()
                            };
                            
                            crate::status_log::log_info(&format!("Switching to branch '{}'...", local_name));
                            match switch_branch(repo, &branch_name, true) {
                                Ok(()) => {
                                    crate::status_log::log_success(&format!("Switched to branch '{}'", local_name));
                                    // Trigger immediate refresh by clicking the refresh button
                                    let btn = refresh_btn.clone();
                                    let branch_btn_clone = branch_btn.clone();
                                    let commit_btn_clone = commit_btn.clone();
                                    let stage_all_btn_clone = stage_all_btn.clone();
                                    let refresh_btn_clone = refresh_btn.clone();
                                    
                                    glib::idle_add_local_once(move || {
                                        btn.emit_clicked();
                                        // Re-enable UI elements after refresh
                                        glib::timeout_add_local_once(std::time::Duration::from_millis(100), move || {
                                            branch_btn_clone.set_sensitive(true);
                                            commit_btn_clone.set_sensitive(true);
                                            stage_all_btn_clone.set_sensitive(true);
                                            refresh_btn_clone.set_sensitive(true);
                                        });
                                    });
                                }
                                Err(e) => {
                                    let error_msg = format!("Failed to switch branch: {}", e);
                                    crate::status_log::log_error(&error_msg);
                                    // Re-enable UI elements on error
                                    branch_btn.set_sensitive(true);
                                    commit_btn.set_sensitive(true);
                                    stage_all_btn.set_sensitive(true);
                                    refresh_btn.set_sensitive(true);
                                }
                            }
                        }
                    });
                    
                    new_action_group.add_action(&action);
                    has_remote = true;
                }
                if has_remote {
                    menu.append_section(Some("Remote Branches"), &remote_section);
                }

                // Update the action group
                *action_group_rc.borrow_mut() = new_action_group;
                branch_button.insert_action_group("branch", Some(&*action_group_rc.borrow()));
                branch_button.set_menu_model(Some(&menu));

                // Get changes
                let changes = get_git_status(&repo);
                *changes_rc.borrow_mut() = changes.clone();

                // Separate staged and unstaged changes
                let mut staged_changes = Vec::new();
                let mut unstaged_changes = Vec::new();

                for change in &changes {
                    match change.status {
                        GitStatus::Staged | GitStatus::Added => {
                            staged_changes.push(change.clone());
                        }
                        GitStatus::Modified | GitStatus::Untracked | GitStatus::Deleted | GitStatus::Renamed => {
                            unstaged_changes.push(change.clone());
                        }
                        GitStatus::ModifiedStaged => {
                            // File appears in both lists
                            staged_changes.push(change.clone());
                            unstaged_changes.push(change.clone());
                        }
                    }
                }

                // Update commit button text based on state
                // Priority: incoming commits > staged changes > unpushed commits
                let incoming_count = count_incoming_commits(&repo);
                
                if incoming_count > 0 {
                    // There are commits to pull - show "Pull" button
                    commit_button.set_label(&format!("Pull ({})", incoming_count));
                    commit_button.set_tooltip_text(Some(&format!("Pull {} commit{} from remote", incoming_count, if incoming_count == 1 { "" } else { "s" })));
                } else if !staged_changes.is_empty() {
                    // There are staged changes - show "Commit"
                    commit_button.set_label("Commit");
                    commit_button.set_tooltip_text(Some("Commit staged changes"));
                } else {
                    // No staged changes - check for unpushed commits
                    let unpushed_count = count_unpushed_commits(&repo);
                    if unpushed_count > 0 {
                        commit_button.set_label(&format!("Push ({})", unpushed_count));
                        commit_button.set_tooltip_text(Some(&format!("Push {} unpushed commit{}", unpushed_count, if unpushed_count == 1 { "" } else { "s" })));
                    } else {
                        commit_button.set_label("Commit");
                        commit_button.set_tooltip_text(Some("No changes to commit"));
                    }
                }

                // Populate staged files list
                for change in &staged_changes {
                    let row = ListBoxRow::new();

                    let file_box = GtkBox::new(Orientation::Horizontal, 8);
                    file_box.set_margin_top(4);
                    file_box.set_margin_bottom(4);
                    file_box.set_margin_start(8);
                    file_box.set_margin_end(8);

                    // File path (relative to repo)
                    let rel_path = change.path.strip_prefix(&repo).unwrap_or(&change.path);
                    let path_label = Label::new(Some(&rel_path.to_string_lossy()));
                    path_label.set_xalign(0.0);
                    path_label.set_ellipsize(pango::EllipsizeMode::Middle);
                    path_label.set_hexpand(true);
                    file_box.append(&path_label);

                    // Store file info and indicate it's staged in tooltip
                    row.set_tooltip_text(Some(&format!("staged:{}", change.path.to_string_lossy())));

                    row.set_child(Some(&file_box));
                    staged_files_list.append(&row);
                }

                // Populate unstaged files list
                for change in &unstaged_changes {
                    let row = ListBoxRow::new();

                    let file_box = GtkBox::new(Orientation::Horizontal, 8);
                    file_box.set_margin_top(4);
                    file_box.set_margin_bottom(4);
                    file_box.set_margin_start(8);
                    file_box.set_margin_end(8);

                    // File path (relative to repo)
                    let rel_path = change.path.strip_prefix(&repo).unwrap_or(&change.path);
                    let path_label = Label::new(Some(&rel_path.to_string_lossy()));
                    path_label.set_xalign(0.0);
                    path_label.set_ellipsize(pango::EllipsizeMode::Middle);
                    path_label.set_hexpand(true);
                    file_box.append(&path_label);

                    // Store file info and indicate it's unstaged
                    row.set_tooltip_text(Some(&format!("unstaged:{}", change.path.to_string_lossy())));

                    row.set_child(Some(&file_box));
                    files_list.append(&row);
                }
            } else {
                *repo_path_rc.borrow_mut() = None;
                branch_button.set_label("Not a git repository");
                branch_button.set_menu_model(Option::<&gtk4::gio::Menu>::None);
            }
        })
    };

    // Initial update
    update_git_status();

    // Refresh button handler
    let update_git_status_for_refresh = update_git_status.clone();
    refresh_button.connect_clicked(move |_| {
        update_git_status_for_refresh();
        crate::status_log::log_info("Git status refreshed");
    });

    // Stage all button handler
    let update_git_status_for_stage_all = update_git_status.clone();
    let repo_path_for_stage_all = repo_path_rc.clone();
    let changes_for_stage_all = changes_rc.clone();
    stage_all_button.connect_clicked(move |_| {
        let repo = match repo_path_for_stage_all.borrow().as_ref() {
            Some(r) => r.clone(),
            None => return,
        };
        
        // Clone the changes to avoid holding a borrow during the update
        let changes_to_stage = changes_for_stage_all.borrow().clone();
        let mut staged_count = 0;

        for change in changes_to_stage.iter() {
            if let Ok(()) = stage_file(&repo, &change.path) {
                staged_count += 1;
            }
        }

        if staged_count > 0 {
            crate::status_log::log_success(&format!("Staged {} file{}", staged_count, if staged_count == 1 { "" } else { "s" }));
            
            // Schedule UI update on the main thread after current event completes
            let update_clone = update_git_status_for_stage_all.clone();
            glib::idle_add_local_once(move || {
                update_clone();
            });
        }
    });

    // Staged files list selection handler
    let repo_path_for_staged = repo_path_rc.clone();
    let editor_notebook_for_staged = editor_notebook.clone();
    let active_tab_path_for_staged = active_tab_path.clone();
    let files_list_for_staged = files_list.clone();
    
    staged_files_list.connect_row_activated(move |_, row| {
        // Unselect the unstaged list
        files_list_for_staged.unselect_all();
        
        if let Some(tooltip) = row.tooltip_text() {
            // Extract file path from tooltip (format: "staged:/path/to/file")
            let file_path_str = tooltip.strip_prefix("staged:").unwrap_or(&tooltip);
            let file_path = PathBuf::from(file_path_str);

            if let Some(repo) = repo_path_for_staged.borrow().as_ref() {
                // Get relative path for the tab title
                let rel_path = file_path.strip_prefix(repo).unwrap_or(&file_path);
                let tab_title = format!("Diff (Staged): {}", rel_path.display());
                
                // Check if a diff tab for this file is already open
                let mut existing_page = None;
                let num_pages = editor_notebook_for_staged.n_pages();
                
                for page_num in 0..num_pages {
                    if let Some(page) = editor_notebook_for_staged.nth_page(Some(page_num)) {
                        if let Some(tab_label) = editor_notebook_for_staged.tab_label(&page) {
                            if let Some(tab_box) = tab_label.downcast_ref::<gtk4::Box>() {
                                if let Some(label) = tab_box.first_child().and_then(|w| w.downcast::<Label>().ok()) {
                                    let label_text = label.text();
                                    let clean_text = label_text.trim_start_matches('*');
                                    if clean_text == tab_title {
                                        existing_page = Some(page_num);
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
                
                if let Some(page_num) = existing_page {
                    editor_notebook_for_staged.set_current_page(Some(page_num));
                } else {
                    // For staged changes: compare HEAD vs staged (index)
                    let old_content = get_old_file_content(repo, &file_path).unwrap_or_default();
                    let new_content = get_staged_file_content(repo, &file_path).unwrap_or_default();
                    
                    create_diff_tab(
                        &editor_notebook_for_staged,
                        &active_tab_path_for_staged,
                        &file_path,
                        repo,
                        &old_content,
                        &new_content,
                        &tab_title,
                    );
                }
            }
        }
    });

    // Unstaged files list selection handler
    let repo_path_for_selection = repo_path_rc.clone();
    let editor_notebook_for_selection = editor_notebook.clone();
    let _file_path_manager_for_selection = file_path_manager.clone();
    let active_tab_path_for_selection = active_tab_path.clone();
    let _parent_window_for_selection = parent_window.clone();
    let staged_files_list_for_selection = staged_files_list.clone();
    
    files_list.connect_row_activated(move |_, row| {
        // Unselect the staged list
        staged_files_list_for_selection.unselect_all();
        
        if let Some(tooltip) = row.tooltip_text() {
            // Extract file path from tooltip (format: "unstaged:/path/to/file")
            let file_path_str = tooltip.strip_prefix("unstaged:").unwrap_or(&tooltip);
            let file_path = PathBuf::from(file_path_str);

            if let Some(repo) = repo_path_for_selection.borrow().as_ref() {
                // Get relative path for the tab title
                let rel_path = file_path.strip_prefix(repo).unwrap_or(&file_path);
                let tab_title = format!("Diff: {}", rel_path.display());
                
                // Check if a diff tab for this file is already open
                let mut existing_page = None;
                let num_pages = editor_notebook_for_selection.n_pages();
                
                for page_num in 0..num_pages {
                    if let Some(page) = editor_notebook_for_selection.nth_page(Some(page_num)) {
                        if let Some(tab_label) = editor_notebook_for_selection.tab_label(&page) {
                            if let Some(tab_box) = tab_label.downcast_ref::<gtk4::Box>() {
                                if let Some(label) = tab_box.first_child().and_then(|w| w.downcast::<Label>().ok()) {
                                    let label_text = label.text();
                                    let clean_text = label_text.trim_start_matches('*');
                                    if clean_text == tab_title {
                                        existing_page = Some(page_num);
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
                
                if let Some(page_num) = existing_page {
                    editor_notebook_for_selection.set_current_page(Some(page_num));
                } else {
                    // For unstaged changes: compare staged (index) vs working directory
                    // If there's no staged version, compare HEAD vs working directory
                    let old_content = get_staged_file_content(repo, &file_path)
                        .or_else(|| get_old_file_content(repo, &file_path))
                        .unwrap_or_default();
                    let new_content = get_new_file_content(&file_path).unwrap_or_default();
                    
                    create_diff_tab(
                        &editor_notebook_for_selection,
                        &active_tab_path_for_selection,
                        &file_path,
                        repo,
                        &old_content,
                        &new_content,
                        &tab_title,
                    );
                }
            }
        }
    });

    // Context menu for unstaged files list (right-click)
    let files_list_for_menu = files_list.clone();
    let repo_path_for_menu = repo_path_rc.clone();
    let update_git_status_for_menu = update_git_status.clone();

    let gesture = gtk4::GestureClick::new();
    gesture.set_button(3); // Right click
    gesture.connect_pressed(move |_, _, x, y| {
        // Find which row was clicked
        if let Some(row) = files_list_for_menu.row_at_y(y as i32) {
            if let Some(tooltip) = row.tooltip_text() {
                let file_path_str = tooltip.strip_prefix("unstaged:").unwrap_or(&tooltip);
                let file_path = PathBuf::from(file_path_str);

                // Create context menu
                let popover = gtk4::Popover::new();
                popover.set_parent(&row);

                let menu_box = GtkBox::new(Orientation::Vertical, 4);
                menu_box.add_css_class("menu");

                // Stage button
                let stage_btn = Button::with_label("Stage");
                stage_btn.add_css_class("flat");
                stage_btn.set_hexpand(true);
                stage_btn.set_halign(gtk4::Align::Start);

                let repo_for_stage = repo_path_for_menu.clone();
                let file_for_stage = file_path.clone();
                let update_for_stage = update_git_status_for_menu.clone();
                let popover_for_stage = popover.downgrade();
                stage_btn.connect_clicked(move |_| {
                    let repo = match repo_for_stage.borrow().as_ref() {
                        Some(r) => r.clone(),
                        None => {
                            if let Some(p) = popover_for_stage.upgrade() {
                                p.popdown();
                            }
                            return;
                        }
                    };
                    
                    match stage_file(&repo, &file_for_stage) {
                        Ok(()) => {
                            crate::status_log::log_success("File staged");
                            let update = update_for_stage.clone();
                            glib::idle_add_local_once(move || {
                                update();
                            });
                        }
                        Err(e) => {
                            crate::status_log::log_error(&format!("Failed to stage: {}", e));
                        }
                    }
                    if let Some(p) = popover_for_stage.upgrade() {
                        p.popdown();
                    }
                });

                // Unstage button
                let unstage_btn = Button::with_label("Unstage");
                unstage_btn.add_css_class("flat");
                unstage_btn.set_hexpand(true);
                unstage_btn.set_halign(gtk4::Align::Start);

                let repo_for_unstage = repo_path_for_menu.clone();
                let file_for_unstage = file_path.clone();
                let update_for_unstage = update_git_status_for_menu.clone();
                let popover_for_unstage = popover.downgrade();
                unstage_btn.connect_clicked(move |_| {
                    let repo = match repo_for_unstage.borrow().as_ref() {
                        Some(r) => r.clone(),
                        None => {
                            if let Some(p) = popover_for_unstage.upgrade() {
                                p.popdown();
                            }
                            return;
                        }
                    };
                    
                    match unstage_file(&repo, &file_for_unstage) {
                        Ok(()) => {
                            crate::status_log::log_success("File unstaged");
                            let update = update_for_unstage.clone();
                            glib::idle_add_local_once(move || {
                                update();
                            });
                        }
                        Err(e) => {
                            crate::status_log::log_error(&format!("Failed to unstage: {}", e));
                        }
                    }
                    if let Some(p) = popover_for_unstage.upgrade() {
                        p.popdown();
                    }
                });

                // Discard changes button
                let discard_btn = Button::with_label("Discard Changes");
                discard_btn.add_css_class("flat");
                discard_btn.add_css_class("destructive-action");
                discard_btn.set_hexpand(true);
                discard_btn.set_halign(gtk4::Align::Start);

                let repo_for_discard = repo_path_for_menu.clone();
                let file_for_discard = file_path.clone();
                let update_for_discard = update_git_status_for_menu.clone();
                let popover_for_discard = popover.downgrade();
                discard_btn.connect_clicked(move |_| {
                    let repo = match repo_for_discard.borrow().as_ref() {
                        Some(r) => r.clone(),
                        None => {
                            if let Some(p) = popover_for_discard.upgrade() {
                                p.popdown();
                            }
                            return;
                        }
                    };
                    
                    // Show confirmation dialog
                    let dialog = gtk4::MessageDialog::new(
                        None::<&gtk4::Window>,
                        gtk4::DialogFlags::MODAL,
                        gtk4::MessageType::Warning,
                        gtk4::ButtonsType::None,
                        "Are you sure you want to discard changes? This cannot be undone.",
                    );
                    dialog.add_buttons(&[
                        ("Cancel", gtk4::ResponseType::Cancel),
                        ("Discard", gtk4::ResponseType::Accept),
                    ]);

                    let repo_clone = repo.clone();
                    let file_clone = file_for_discard.clone();
                    let update_clone = update_for_discard.clone();
                    dialog.connect_response(move |d, response| {
                        if response == gtk4::ResponseType::Accept {
                            match discard_changes(&repo_clone, &file_clone) {
                                Ok(()) => {
                                    crate::status_log::log_success("Changes discarded");
                                    let update = update_clone.clone();
                                    glib::idle_add_local_once(move || {
                                        update();
                                    });
                                }
                                Err(e) => {
                                    crate::status_log::log_error(&format!("Failed to discard: {}", e));
                                }
                            }
                        }
                        d.close();
                    });

                    dialog.show();
                    if let Some(p) = popover_for_discard.upgrade() {
                        p.popdown();
                    }
                });

                menu_box.append(&stage_btn);
                menu_box.append(&unstage_btn);
                menu_box.append(&discard_btn);

                popover.set_child(Some(&menu_box));

                let rect = gtk4::gdk::Rectangle::new(x as i32, y as i32, 1, 1);
                popover.set_pointing_to(Some(&rect));
                popover.popup();
            }
        }
    });

    files_list.add_controller(gesture);

    // Context menu for staged files list (right-click)
    let staged_files_list_for_menu = staged_files_list.clone();
    let repo_path_for_staged_menu = repo_path_rc.clone();
    let update_git_status_for_staged_menu = update_git_status.clone();

    let staged_gesture = gtk4::GestureClick::new();
    staged_gesture.set_button(3); // Right click
    staged_gesture.connect_pressed(move |_, _, x, y| {
        if let Some(row) = staged_files_list_for_menu.row_at_y(y as i32) {
            if let Some(tooltip) = row.tooltip_text() {
                let file_path_str = tooltip.strip_prefix("staged:").unwrap_or(&tooltip);
                let file_path = PathBuf::from(file_path_str);

                let popover = gtk4::Popover::new();
                popover.set_parent(&row);

                let menu_box = GtkBox::new(Orientation::Vertical, 4);
                menu_box.add_css_class("menu");

                // Unstage button (main action for staged files)
                let unstage_btn = Button::with_label("Unstage");
                unstage_btn.add_css_class("flat");
                unstage_btn.set_hexpand(true);
                unstage_btn.set_halign(gtk4::Align::Start);

                let repo_for_unstage = repo_path_for_staged_menu.clone();
                let file_for_unstage = file_path.clone();
                let update_for_unstage = update_git_status_for_staged_menu.clone();
                let popover_for_unstage = popover.downgrade();
                unstage_btn.connect_clicked(move |_| {
                    let repo = match repo_for_unstage.borrow().as_ref() {
                        Some(r) => r.clone(),
                        None => {
                            if let Some(p) = popover_for_unstage.upgrade() {
                                p.popdown();
                            }
                            return;
                        }
                    };
                    
                    match unstage_file(&repo, &file_for_unstage) {
                        Ok(()) => {
                            crate::status_log::log_success("File unstaged");
                            let update = update_for_unstage.clone();
                            glib::idle_add_local_once(move || {
                                update();
                            });
                        }
                        Err(e) => {
                            crate::status_log::log_error(&format!("Failed to unstage: {}", e));
                        }
                    }
                    if let Some(p) = popover_for_unstage.upgrade() {
                        p.popdown();
                    }
                });

                menu_box.append(&unstage_btn);

                popover.set_child(Some(&menu_box));

                let rect = gtk4::gdk::Rectangle::new(x as i32, y as i32, 1, 1);
                popover.set_pointing_to(Some(&rect));
                popover.popup();
            }
        }
    });

    staged_files_list.add_controller(staged_gesture);

    // Smart commit/push/pull button handler
    let repo_path_for_commit = repo_path_rc.clone();
    let update_git_status_for_commit = update_git_status.clone();
    let commit_message_view_for_commit = commit_message_view.clone();
    let changes_for_commit = changes_rc.clone();
    let show_placeholder_for_commit = show_placeholder.clone();
    
    commit_button.connect_clicked(move |button| {
        // Prevent spam clicking - disable button during operation
        if !button.is_sensitive() {
            return;
        }
        
        let repo = match repo_path_for_commit.borrow().as_ref() {
            Some(r) => r.clone(),
            None => {
                crate::status_log::log_error("Not in a git repository");
                return;
            }
        };
        
        // Check if we have staged changes
        let changes = changes_for_commit.borrow();
        let has_staged = changes.iter().any(|c| matches!(c.status, GitStatus::Staged | GitStatus::Added | GitStatus::ModifiedStaged));
        
        // Check button label to determine action
        let button_label = button.label();
        let label_str = button_label.as_ref().map(|s| s.as_str()).unwrap_or("");
        
        // Disable button during operation
        button.set_sensitive(false);
        
        if label_str.starts_with("Pull") {
            // Button says "Pull" - perform pull
            crate::status_log::log_info("Pulling from remote...");
            
            match pull_changes(&repo) {
                Ok(()) => {
                    crate::status_log::log_success("Pulled from remote successfully");
                    // Refresh git status
                    let update = update_git_status_for_commit.clone();
                    glib::idle_add_local_once(move || {
                        update();
                    });
                }
                Err(e) => {
                    crate::status_log::log_error(&format!("Pull failed: {}", e));
                }
            }
            
            // Re-enable button after operation
            button.set_sensitive(true);
        } else if has_staged {
            // We have staged changes - perform commit
            let buffer = commit_message_view_for_commit.buffer();
            let start = buffer.start_iter();
            let end = buffer.end_iter();
            let message = buffer.text(&start, &end, false);
            
            // Skip if message is just the placeholder
            if message.trim().is_empty() || message == "Commit message" {
                crate::status_log::log_error("Commit message cannot be empty");
                button.set_sensitive(true);
                return;
            }
            
            // Perform commit
            match commit_changes(&repo, &message) {
                Ok(()) => {
                    crate::status_log::log_success("Changes committed successfully");
                    // Clear commit message and restore placeholder
                    buffer.set_text("");
                    show_placeholder_for_commit();
                    // Refresh git status
                    let update = update_git_status_for_commit.clone();
                    glib::idle_add_local_once(move || {
                        update();
                    });
                }
                Err(e) => {
                    crate::status_log::log_error(&format!("Commit failed: {}", e));
                }
            }
            
            // Re-enable button after operation
            button.set_sensitive(true);
        } else if label_str.starts_with("Push") {
            // No staged changes but button says "Push" - perform push
            crate::status_log::log_info("Pushing to remote...");
            
            match push_changes(&repo) {
                Ok(()) => {
                    crate::status_log::log_success("Pushed to remote successfully");
                    // Refresh git status
                    let update = update_git_status_for_commit.clone();
                    glib::idle_add_local_once(move || {
                        update();
                    });
                }
                Err(e) => {
                    crate::status_log::log_error(&format!("Push failed: {}", e));
                }
            }
            
            // Re-enable button after operation
            button.set_sensitive(true);
        } else {
            crate::status_log::log_info("No changes to commit or push");
            button.set_sensitive(true);
        }
    });

    // Update when current directory changes (periodic check)
    let current_dir_for_check = current_dir.clone();
    let last_checked_dir: Rc<RefCell<Option<PathBuf>>> = Rc::new(RefCell::new(None));
    let update_git_status_for_check = update_git_status.clone();

    glib::timeout_add_local(std::time::Duration::from_secs(2), move || {
        let current = current_dir_for_check.borrow().clone();
        let last = last_checked_dir.borrow().clone();

        if last.as_ref() != Some(&current) {
            *last_checked_dir.borrow_mut() = Some(current);
            update_git_status_for_check();
        }

        glib::ControlFlow::Continue
    });

    // Trigger initial update
    update_git_status();

    // Return the panel as a GtkBox
    panel.upcast::<GtkBox>()
}
