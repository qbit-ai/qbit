use serde::Serialize;
use std::process::Command;

#[derive(Debug, Clone, Serialize)]
pub struct GitStatusEntry {
    pub path: String,
    pub index_status: Option<char>,
    pub worktree_status: Option<char>,
    pub rename_from: Option<String>,
    pub rename_to: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GitStatusSummary {
    pub branch: Option<String>,
    pub ahead: i32,
    pub behind: i32,
    pub entries: Vec<GitStatusEntry>,
    pub insertions: i32,
    pub deletions: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct GitDiffResult {
    pub file: String,
    pub staged: bool,
    pub is_binary: bool,
    pub diff: String,
}

fn run_git_command(args: &[&str], working_directory: &str) -> Result<std::process::Output, String> {
    Command::new("git")
        .args(args)
        .current_dir(working_directory)
        .output()
        .map_err(|e| format!("failed to run git: {e}"))
        .and_then(|output| {
            if output.status.success() {
                Ok(output)
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                Err(if stderr.is_empty() {
                    "git command failed".to_string()
                } else {
                    stderr
                })
            }
        })
}

fn parse_branch_line(line: &str) -> (Option<String>, i32, i32) {
    let mut branch: Option<String> = None;
    let mut ahead = 0;
    let mut behind = 0;

    let rest = line.trim_start_matches("## ").trim();

    // Extract branch name (before ... or before space)
    if let Some((name, _)) = rest.split_once("...") {
        branch = Some(name.to_string());
    } else if let Some(name) = rest.split(' ').next() {
        branch = Some(name.to_string());
    }

    // Extract ahead/behind markers e.g., "[ahead 1]" or "[ahead 1, behind 2]"
    if let Some(start) = rest.find('[') {
        if let Some(end) = rest.find(']') {
            let meta = &rest[start + 1..end];
            for part in meta.split(',') {
                let trimmed = part.trim();
                if let Some(num) = trimmed.strip_prefix("ahead ") {
                    ahead = num.parse().unwrap_or(0);
                }
                if let Some(num) = trimmed.strip_prefix("behind ") {
                    behind = num.parse().unwrap_or(0);
                }
            }
        }
    }

    (branch, ahead, behind)
}

fn parse_status_line(line: &str) -> Option<GitStatusEntry> {
    if line.len() < 3 {
        return None;
    }

    let status = &line[0..2];
    let rest = line[3..].trim();

    let mut rename_from: Option<String> = None;
    let mut rename_to: Option<String> = None;
    let path: String;

    if let Some((from, to)) = rest.split_once(" -> ") {
        rename_from = Some(from.trim().to_string());
        rename_to = Some(to.trim().to_string());
        path = rename_to.clone().unwrap_or_else(|| to.trim().to_string());
    } else {
        path = rest.to_string();
    }

    let mut chars = status.chars();
    let index_status = chars.next();
    let worktree_status = chars.next();

    Some(GitStatusEntry {
        path,
        index_status,
        worktree_status,
        rename_from,
        rename_to,
    })
}

/// Parse git diff --numstat output to get insertions/deletions
/// Each line is: <insertions>\t<deletions>\t<filename>
/// Binary files show "-" for insertions/deletions
fn parse_diff_numstat(output: &str) -> (i32, i32) {
    let mut insertions = 0;
    let mut deletions = 0;

    for line in output.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 2 {
            // Skip binary files (marked with "-")
            if parts[0] != "-" {
                insertions += parts[0].parse::<i32>().unwrap_or(0);
            }
            if parts[1] != "-" {
                deletions += parts[1].parse::<i32>().unwrap_or(0);
            }
        }
    }

    (insertions, deletions)
}

#[tauri::command]
pub async fn git_status(working_directory: String) -> Result<GitStatusSummary, String> {
    let output = run_git_command(&["status", "--porcelain", "--branch"], &working_directory)?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    let mut entries: Vec<GitStatusEntry> = Vec::new();
    let mut branch: Option<String> = None;
    let mut ahead = 0;
    let mut behind = 0;

    for line in stdout.lines() {
        if line.starts_with("## ") {
            let (b, a, be) = parse_branch_line(line);
            branch = b;
            ahead = a;
            behind = be;
            continue;
        }

        if let Some(entry) = parse_status_line(line) {
            entries.push(entry);
        }
    }

    // Get insertions/deletions using git diff --numstat
    // This includes both staged and unstaged changes
    let mut insertions = 0;
    let mut deletions = 0;

    // Get unstaged changes
    if let Ok(unstaged_output) = run_git_command(&["diff", "--numstat"], &working_directory) {
        let unstaged_stdout = String::from_utf8_lossy(&unstaged_output.stdout);
        let (ins, del) = parse_diff_numstat(&unstaged_stdout);
        insertions += ins;
        deletions += del;
    }

    // Get staged changes
    if let Ok(staged_output) =
        run_git_command(&["diff", "--numstat", "--cached"], &working_directory)
    {
        let staged_stdout = String::from_utf8_lossy(&staged_output.stdout);
        let (ins, del) = parse_diff_numstat(&staged_stdout);
        insertions += ins;
        deletions += del;
    }

    Ok(GitStatusSummary {
        branch,
        ahead,
        behind,
        entries,
        insertions,
        deletions,
    })
}

#[tauri::command]
pub async fn git_diff(
    working_directory: String,
    file: String,
    staged: Option<bool>,
) -> Result<GitDiffResult, String> {
    let mut args = vec!["diff", "--no-color"];
    if staged.unwrap_or(false) {
        args.push("--cached");
    }
    args.push("--");
    args.push(&file);

    let output = run_git_command(&args, &working_directory)?;
    let diff = String::from_utf8_lossy(&output.stdout).to_string();

    // Rough binary detection: empty diff with status "binary" is handled elsewhere; here we just flag if git reports binary in stderr
    let is_binary = diff.contains("Binary files") || diff.contains("GIT binary patch");

    Ok(GitDiffResult {
        file,
        staged: staged.unwrap_or(false),
        is_binary,
        diff,
    })
}

/// Get the combined diff for all staged changes.
/// This is useful for generating commit messages.
#[tauri::command]
pub async fn git_diff_staged(working_directory: String) -> Result<String, String> {
    let args = vec!["diff", "--cached", "--no-color"];
    let output = run_git_command(&args, &working_directory)?;
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

#[tauri::command]
pub async fn git_stage(working_directory: String, files: Vec<String>) -> Result<(), String> {
    if files.is_empty() {
        return Ok(());
    }
    let mut args = vec!["add"];
    args.push("--");
    let file_refs: Vec<&str> = files.iter().map(|f| f.as_str()).collect();
    args.extend(file_refs);
    run_git_command(&args, &working_directory).map(|_| ())
}

#[tauri::command]
pub async fn git_unstage(working_directory: String, files: Vec<String>) -> Result<(), String> {
    if files.is_empty() {
        return Ok(());
    }
    let mut args = vec!["reset", "HEAD", "--"];
    let file_refs: Vec<&str> = files.iter().map(|f| f.as_str()).collect();
    args.extend(file_refs);
    run_git_command(&args, &working_directory).map(|_| ())
}

#[tauri::command]
pub async fn git_commit(
    working_directory: String,
    message: String,
    sign_off: Option<bool>,
    amend: Option<bool>,
) -> Result<(), String> {
    let mut args = vec!["commit", "-m", &message];
    if sign_off.unwrap_or(false) {
        args.push("--signoff");
    }
    if amend.unwrap_or(false) {
        args.push("--amend");
        args.push("--no-edit");
    }

    run_git_command(&args, &working_directory).map(|_| ())
}

#[tauri::command]
pub async fn git_push(
    working_directory: String,
    force: Option<bool>,
    set_upstream: Option<bool>,
) -> Result<(), String> {
    let mut args = vec!["push"];
    if force.unwrap_or(false) {
        args.push("--force");
    }
    if set_upstream.unwrap_or(false) {
        args.push("--set-upstream");
        args.push("origin");
        args.push("HEAD");
    }
    run_git_command(&args, &working_directory).map(|_| ())
}
