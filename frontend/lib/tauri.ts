import { invoke } from "@tauri-apps/api/core";

// Types matching Rust structs
export interface PtySession {
  id: string;
  working_directory: string;
  rows: number;
  cols: number;
}

export interface IntegrationStatus {
  type: "NotInstalled" | "Installed" | "Outdated";
  version?: string;
  current?: string;
  latest?: string;
}

// PTY Commands
export async function ptyCreate(
  workingDirectory?: string,
  rows?: number,
  cols?: number
): Promise<PtySession> {
  return invoke("pty_create", {
    workingDirectory,
    rows: rows ?? 24,
    cols: cols ?? 80,
  });
}

export async function ptyWrite(sessionId: string, data: string): Promise<void> {
  return invoke("pty_write", { sessionId, data });
}

export async function ptyResize(sessionId: string, rows: number, cols: number): Promise<void> {
  return invoke("pty_resize", { sessionId, rows, cols });
}

export async function ptyDestroy(sessionId: string): Promise<void> {
  return invoke("pty_destroy", { sessionId });
}

export async function ptyGetSession(sessionId: string): Promise<PtySession> {
  return invoke("pty_get_session", { sessionId });
}

export async function ptyGetForegroundProcess(sessionId: string): Promise<string | null> {
  return invoke("pty_get_foreground_process", { sessionId });
}

// Shell Integration Commands
export async function shellIntegrationStatus(): Promise<IntegrationStatus> {
  return invoke("shell_integration_status");
}

export async function shellIntegrationInstall(): Promise<void> {
  return invoke("shell_integration_install");
}

export async function shellIntegrationUninstall(): Promise<void> {
  return invoke("shell_integration_uninstall");
}

export async function getGitBranch(path: string): Promise<string | null> {
  return invoke("get_git_branch", { path });
}

// Prompt Commands
export interface PromptInfo {
  name: string;
  path: string;
  source: "global" | "local";
}

export async function listPrompts(workingDirectory?: string): Promise<PromptInfo[]> {
  return invoke("list_prompts", { workingDirectory });
}

export async function readPrompt(path: string): Promise<string> {
  return invoke("read_prompt", { path });
}

// Skill Commands (for Agent Skills support)
export interface SkillInfo {
  name: string;
  path: string;
  source: "global" | "local";
  description: string;
  license?: string;
  compatibility?: string;
  metadata?: Record<string, string>;
  allowed_tools?: string[];
  has_scripts: boolean;
  has_references: boolean;
  has_assets: boolean;
}

export interface SkillFileInfo {
  name: string;
  relative_path: string;
  is_directory: boolean;
}

export async function listSkills(workingDirectory?: string): Promise<SkillInfo[]> {
  return invoke("list_skills", { workingDirectory });
}

export async function readSkill(path: string): Promise<string> {
  return invoke("read_skill", { path });
}

export async function readSkillBody(path: string): Promise<string> {
  return invoke("read_skill_body", { path });
}

export async function listSkillFiles(skillPath: string, subdir: string): Promise<SkillFileInfo[]> {
  return invoke("list_skill_files", { skillPath, subdir });
}

export async function readSkillFile(skillPath: string, relativePath: string): Promise<string> {
  return invoke("read_skill_file", { skillPath, relativePath });
}

// File Commands (for @ file references)
export interface FileInfo {
  name: string;
  relative_path: string;
}

export async function listWorkspaceFiles(
  workingDirectory: string,
  query?: string,
  limit?: number
): Promise<FileInfo[]> {
  return invoke("list_workspace_files", { workingDirectory, query, limit });
}

// Path Completion Commands (for Tab completion in terminal mode)
export type PathEntryType = "file" | "directory" | "symlink";

export interface PathCompletion {
  name: string;
  insert_text: string;
  entry_type: PathEntryType;
  score: number;
  match_indices: number[];
}

export interface PathCompletionResponse {
  completions: PathCompletion[];
  total_count: number;
}

export async function listPathCompletions(
  sessionId: string,
  partialPath: string,
  limit?: number
): Promise<PathCompletionResponse> {
  return invoke<PathCompletionResponse>("list_path_completions", {
    sessionId,
    partialPath,
    limit,
  });
}

// Git commands
export interface GitStatusEntry {
  path: string;
  index_status: string | null;
  worktree_status: string | null;
  rename_from: string | null;
  rename_to: string | null;
}

export interface GitStatusSummary {
  branch: string | null;
  ahead: number;
  behind: number;
  entries: GitStatusEntry[];
  insertions: number;
  deletions: number;
}

export interface GitDiffResult {
  file: string;
  staged: boolean;
  is_binary: boolean;
  diff: string;
}

export async function gitStatus(workingDirectory: string): Promise<GitStatusSummary> {
  return invoke("git_status", { workingDirectory });
}

export async function gitDiff(
  workingDirectory: string,
  file: string,
  staged?: boolean
): Promise<GitDiffResult> {
  return invoke("git_diff", { workingDirectory, file, staged });
}

/**
 * Get the combined diff for all staged changes.
 * Useful for generating commit messages.
 */
export async function gitDiffStaged(workingDirectory: string): Promise<string> {
  return invoke("git_diff_staged", { workingDirectory });
}

export async function gitStage(workingDirectory: string, files: string[]): Promise<void> {
  return invoke("git_stage", { workingDirectory, files });
}

export async function gitUnstage(workingDirectory: string, files: string[]): Promise<void> {
  return invoke("git_unstage", { workingDirectory, files });
}

export async function gitCommit(
  workingDirectory: string,
  message: string,
  options?: { signOff?: boolean; amend?: boolean }
): Promise<void> {
  return invoke("git_commit", {
    workingDirectory,
    message,
    sign_off: options?.signOff ?? false,
    amend: options?.amend ?? false,
  });
}

export async function gitPush(
  workingDirectory: string,
  options?: { force?: boolean; setUpstream?: boolean }
): Promise<void> {
  return invoke("git_push", {
    workingDirectory,
    force: options?.force ?? false,
    set_upstream: options?.setUpstream ?? false,
  });
}

export async function deleteWorktree(
  workingDirectory: string,
  worktreePath: string,
  force?: boolean
): Promise<void> {
  return invoke("git_delete_worktree", { workingDirectory, worktreePath, force });
}

/**
 * Read a file as base64 data URL.
 * Accepts absolute paths (for drag-drop from anywhere on the system).
 */
export async function readFileAsBase64(path: string): Promise<string> {
  return invoke("read_file_as_base64", { path });
}

/**
 * Read a text file's contents.
 * Used for displaying untracked file diffs.
 */
export async function readTextFile(
  workingDirectory: string,
  relativePath: string
): Promise<string> {
  // Use the read_prompt command which reads file contents
  const fullPath = `${workingDirectory}/${relativePath}`;
  return invoke("read_prompt", { path: fullPath });
}
