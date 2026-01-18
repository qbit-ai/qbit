import { invoke } from "@tauri-apps/api/core";

export interface FileReadResult {
  content: string;
  modifiedAt?: string;
  encoding?: string;
}

export interface FileWriteOptions {
  encoding?: string;
  expectedModifiedAt?: string;
  createIfMissing?: boolean;
}

export interface FileWriteResult {
  modifiedAt?: string;
}

export async function readWorkspaceFile(path: string): Promise<FileReadResult> {
  return invoke("read_workspace_file", { path });
}

export async function writeWorkspaceFile(
  path: string,
  content: string,
  options?: FileWriteOptions
): Promise<FileWriteResult> {
  return invoke("write_workspace_file", { path, content, options });
}

export async function statWorkspaceFile(
  path: string
): Promise<{ modifiedAt: string; size: number }> {
  return invoke("stat_workspace_file", { path });
}

export type DirEntryType = "file" | "directory" | "symlink";

export interface DirEntry {
  name: string;
  path: string;
  entryType: DirEntryType;
  size?: number;
  modifiedAt?: string;
}

export async function listDirectory(path: string): Promise<DirEntry[]> {
  return invoke("list_directory", { path });
}
