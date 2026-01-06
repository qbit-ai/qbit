import type { GitStatusEntry } from "./tauri";

export type GitChangeKind =
  | "modified"
  | "added"
  | "deleted"
  | "renamed"
  | "untracked"
  | "conflict"
  | "unknown";

export interface GitChange {
  path: string;
  kind: GitChangeKind;
  staged: boolean;
  renameFrom?: string | null;
  renameTo?: string | null;
  indexStatus?: string | null;
  worktreeStatus?: string | null;
}

const CONFLICT_CODES = new Set(["DD", "AU", "UD", "UA", "DU", "AA", "UU"]);

export function formatStatus(index: string | null, worktree: string | null): string {
  return `${index ?? " "}${worktree ?? " "}`;
}

export function mapStatusEntries(entries: GitStatusEntry[]): GitChange[] {
  const results: GitChange[] = [];

  for (const entry of entries) {
    const combined = formatStatus(entry.index_status, entry.worktree_status);

    // Handle conflicts first - they go into the conflicts bucket
    if (CONFLICT_CODES.has(combined)) {
      results.push({
        path: entry.path,
        kind: "conflict",
        staged: false,
        renameFrom: entry.rename_from,
        renameTo: entry.rename_to,
        indexStatus: entry.index_status,
        worktreeStatus: entry.worktree_status,
      });
      continue;
    }

    // Handle untracked files (both chars are ?)
    if (entry.index_status === "?" && entry.worktree_status === "?") {
      results.push({
        path: entry.path,
        kind: "untracked",
        staged: false,
        renameFrom: null,
        renameTo: null,
        indexStatus: entry.index_status,
        worktreeStatus: entry.worktree_status,
      });
      continue;
    }

    // Check for staged changes (index_status is not empty or space)
    const hasStaged =
      entry.index_status && entry.index_status !== " " && entry.index_status !== "?";
    // Check for unstaged changes (worktree_status is not empty or space)
    const hasUnstaged =
      entry.worktree_status && entry.worktree_status !== " " && entry.worktree_status !== "?";

    // Determine kind based on status character
    const getKindFromStatus = (status: string | null): GitChangeKind => {
      if (status === "R") return "renamed";
      if (status === "A") return "added";
      if (status === "D") return "deleted";
      if (status === "M") return "modified";
      if (status === "C") return "added"; // copied
      return "modified";
    };

    // Add staged entry if there are staged changes
    if (hasStaged) {
      results.push({
        path: entry.path,
        kind: getKindFromStatus(entry.index_status),
        staged: true,
        renameFrom: entry.rename_from,
        renameTo: entry.rename_to,
        indexStatus: entry.index_status,
        worktreeStatus: entry.worktree_status,
      });
    }

    // Add unstaged entry if there are unstaged changes
    if (hasUnstaged) {
      results.push({
        path: entry.path,
        kind: getKindFromStatus(entry.worktree_status),
        staged: false,
        renameFrom: null,
        renameTo: null,
        indexStatus: entry.index_status,
        worktreeStatus: entry.worktree_status,
      });
    }
  }

  return results;
}

export function splitChanges(changes: GitChange[]) {
  const staged: GitChange[] = [];
  const unstaged: GitChange[] = [];
  const untracked: GitChange[] = [];
  const conflicts: GitChange[] = [];

  for (const change of changes) {
    if (change.kind === "conflict") {
      conflicts.push(change);
      continue;
    }
    if (change.kind === "untracked") {
      untracked.push(change);
      continue;
    }

    if (change.staged) {
      staged.push(change);
    } else {
      unstaged.push(change);
    }
  }

  return { staged, unstaged, untracked, conflicts };
}
