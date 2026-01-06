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

function detectKind(entry: GitStatusEntry): GitChangeKind {
  const combined = formatStatus(entry.index_status, entry.worktree_status);

  if (CONFLICT_CODES.has(combined)) {
    return "conflict";
  }
  if ((entry.index_status === "?" || entry.worktree_status === "?") && !entry.index_status) {
    return "untracked";
  }
  if (entry.index_status === "?" || entry.worktree_status === "?") {
    return "untracked";
  }
  if (
    entry.index_status === "R" ||
    entry.worktree_status === "R" ||
    entry.rename_from ||
    entry.rename_to
  ) {
    return "renamed";
  }
  if (entry.index_status === "A" || entry.worktree_status === "A") {
    return "added";
  }
  if (entry.index_status === "D" || entry.worktree_status === "D") {
    return "deleted";
  }
  if (entry.index_status === "M" || entry.worktree_status === "M") {
    return "modified";
  }
  return "unknown";
}

export function mapStatusEntries(entries: GitStatusEntry[]): GitChange[] {
  return entries.map((entry) => {
    const kind = detectKind(entry);
    const staged = !!(
      entry.index_status &&
      entry.index_status !== " " &&
      entry.index_status !== "?"
    );
    return {
      path: entry.path,
      kind,
      staged,
      renameFrom: entry.rename_from,
      renameTo: entry.rename_to,
      indexStatus: entry.index_status,
      worktreeStatus: entry.worktree_status,
    };
  });
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
