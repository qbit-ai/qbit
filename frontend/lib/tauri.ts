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
}

export async function listPathCompletions(
  sessionId: string,
  partialPath: string,
  limit?: number
): Promise<PathCompletion[]> {
  return invoke("list_path_completions", {
    sessionId,
    partialPath,
    limit,
  });
}
