import {
  ArrowDown,
  ArrowUp,
  ChevronDown,
  ChevronRight,
  File,
  FileDiff,
  GitBranch,
  GitCommitHorizontal,
  Loader2,
  Minus,
  Pencil,
  Plus,
  RefreshCcw,
  Sparkles,
  X,
} from "lucide-react";
import { memo, useCallback, useEffect, useMemo, useState, type ReactNode } from "react";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from "@/components/ui/resizable";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Textarea } from "@/components/ui/textarea";
import {
  gitCommit,
  gitDiff,
  gitPush,
  gitStage,
  gitStatus as fetchGitStatus,
  gitUnstage,
} from "@/lib/tauri";
import { generateCommitMessage } from "@/lib/ai";
import { mapStatusEntries, splitChanges, type GitChange } from "@/lib/git";
import { notify } from "@/lib/notify";
import { cn } from "@/lib/utils";
import { useGitCommitMessage, useGitStatus, useGitStatusLoading, useStore } from "@/store";

interface GitPanelProps {
  sessionId: string | null;
  workingDirectory?: string;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onOpenFile?: (path: string) => void;
}

interface TreeNode {
  name: string;
  path: string;
  isDirectory: boolean;
  children: TreeNode[];
  change?: GitChange;
}

const SUMMARY_MAX_LENGTH = 72;

interface DiffLine {
  oldLineNum: number | null;
  newLineNum: number | null;
  content: string;
  type: "header" | "hunk" | "add" | "remove" | "context";
}

function parseDiff(diffText: string): DiffLine[] {
  const lines = diffText.split("\n");
  const result: DiffLine[] = [];
  let oldLine = 0;
  let newLine = 0;

  for (const line of lines) {
    if (
      line.startsWith("diff ") ||
      line.startsWith("index ") ||
      line.startsWith("---") ||
      line.startsWith("+++")
    ) {
      result.push({ oldLineNum: null, newLineNum: null, content: line, type: "header" });
    } else if (line.startsWith("@@")) {
      // Parse hunk header: @@ -oldStart,oldCount +newStart,newCount @@
      const match = line.match(/@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@/);
      if (match) {
        oldLine = parseInt(match[1], 10);
        newLine = parseInt(match[2], 10);
      }
      result.push({ oldLineNum: null, newLineNum: null, content: line, type: "hunk" });
    } else if (line.startsWith("+")) {
      result.push({ oldLineNum: null, newLineNum: newLine, content: line, type: "add" });
      newLine++;
    } else if (line.startsWith("-")) {
      result.push({ oldLineNum: oldLine, newLineNum: null, content: line, type: "remove" });
      oldLine++;
    } else {
      // Context line or empty
      result.push({
        oldLineNum: oldLine,
        newLineNum: newLine,
        content: line || " ",
        type: "context",
      });
      oldLine++;
      newLine++;
    }
  }

  return result;
}

function DiffView({ content }: { content: string }) {
  const lines = useMemo(() => parseDiff(content), [content]);
  const lineNumWidth = useMemo(() => {
    const maxLine = lines.reduce(
      (max, l) => Math.max(max, l.oldLineNum ?? 0, l.newLineNum ?? 0),
      0
    );
    return Math.max(3, String(maxLine).length);
  }, [lines]);

  return (
    <div className="text-xs font-mono">
      {lines.map((line, i) => {
        let lineClass = "text-muted-foreground";
        let bgClass = "";
        let indicator = " ";
        let indicatorClass = "";

        if (line.type === "add") {
          lineClass = "text-emerald-400";
          bgClass = "bg-emerald-400/10";
          indicator = "+";
          indicatorClass = "text-emerald-400";
        } else if (line.type === "remove") {
          lineClass = "text-red-400";
          bgClass = "bg-red-400/10";
          indicator = "-";
          indicatorClass = "text-red-400";
        } else if (line.type === "hunk") {
          lineClass = "text-sky-400";
        } else if (line.type === "header") {
          lineClass = "text-muted-foreground font-semibold";
        }

        const showLineNums = line.type !== "header" && line.type !== "hunk";
        // Strip leading +/- from content for add/remove lines
        const displayContent =
          line.type === "add" || line.type === "remove" ? line.content.slice(1) : line.content;

        return (
          <div key={i} className="flex">
            {showLineNums ? (
              <>
                <span
                  className="text-muted-foreground/50 select-none px-1 text-right shrink-0"
                  style={{ width: `${lineNumWidth + 1}ch` }}
                >
                  {line.oldLineNum ?? ""}
                </span>
                <span
                  className="text-muted-foreground/50 select-none px-1 text-right shrink-0"
                  style={{ width: `${lineNumWidth + 1}ch` }}
                >
                  {line.newLineNum ?? ""}
                </span>
                <span
                  className={cn(
                    "select-none w-4 text-center shrink-0 border-r border-border",
                    indicatorClass
                  )}
                >
                  {indicator}
                </span>
              </>
            ) : (
              <span
                className="shrink-0 border-r border-border"
                style={{ width: `${(lineNumWidth + 1) * 2 + 2}ch` }}
              />
            )}
            <span className={cn("whitespace-pre flex-1 pl-2", lineClass, bgClass)}>
              {displayContent}
            </span>
          </div>
        );
      })}
    </div>
  );
}

function buildFileTree(changes: GitChange[]): TreeNode[] {
  const root: TreeNode[] = [];

  for (const change of changes) {
    const parts = change.path.split("/");
    let currentLevel = root;

    for (let i = 0; i < parts.length; i++) {
      const part = parts[i];
      const isFile = i === parts.length - 1;
      const pathSoFar = parts.slice(0, i + 1).join("/");

      let existing = currentLevel.find((n) => n.name === part);

      if (!existing) {
        existing = {
          name: part,
          path: pathSoFar,
          isDirectory: !isFile,
          children: [],
          change: isFile ? change : undefined,
        };
        currentLevel.push(existing);
      }

      if (!isFile) {
        currentLevel = existing.children;
      }
    }
  }

  // Compact single-child directory chains: backend/crates/qbit → one node
  const compactNodes = (nodes: TreeNode[]): TreeNode[] => {
    return nodes.map((node) => {
      if (!node.isDirectory) return node;

      // Recursively compact children first
      let compacted = { ...node, children: compactNodes(node.children) };

      // While this directory has exactly one child that is also a directory, merge them
      while (compacted.children.length === 1 && compacted.children[0].isDirectory) {
        const child = compacted.children[0];
        compacted = {
          ...compacted,
          name: `${compacted.name}/${child.name}`,
          path: child.path,
          children: child.children,
        };
      }

      return compacted;
    });
  };

  // Sort: directories first, then alphabetically
  const sortNodes = (nodes: TreeNode[]): TreeNode[] => {
    return nodes
      .map((node) => ({
        ...node,
        children: sortNodes(node.children),
      }))
      .sort((a, b) => {
        if (a.isDirectory && !b.isDirectory) return -1;
        if (!a.isDirectory && b.isDirectory) return 1;
        return a.name.localeCompare(b.name);
      });
  };

  return sortNodes(compactNodes(root));
}

function FileTreeItem({
  node,
  depth,
  onStage,
  onUnstage,
  onDiff,
  actionLabel,
  isStaged,
}: {
  node: TreeNode;
  depth: number;
  onStage?: (path: string) => void;
  onUnstage?: (path: string) => void;
  onDiff?: (path: string) => void;
  actionLabel: string;
  isStaged: boolean;
}) {
  const [expanded, setExpanded] = useState(true);

  // Get the appropriate icon based on change type
  const StatusIcon = useMemo(() => {
    if (!node.change) return { icon: File, className: "text-muted-foreground" };
    switch (node.change.kind) {
      case "added":
      case "untracked":
        return { icon: Plus, className: "text-emerald-400" };
      case "deleted":
        return { icon: Minus, className: "text-red-400" };
      case "modified":
      case "renamed":
        return { icon: Pencil, className: "text-amber-400" };
      case "conflict":
        return { icon: File, className: "text-pink-400" };
      default:
        return { icon: File, className: "text-muted-foreground" };
    }
  }, [node.change]);

  if (node.isDirectory) {
    return (
      <div>
        <div
          className="flex items-center gap-1 py-0.5 px-1 rounded hover:bg-muted/40 cursor-pointer select-none"
          style={{ paddingLeft: `${depth * 20 + 8}px` }}
          onClick={() => setExpanded(!expanded)}
        >
          {expanded ? (
            <ChevronDown className="w-3.5 h-3.5 text-muted-foreground shrink-0" />
          ) : (
            <ChevronRight className="w-3.5 h-3.5 text-muted-foreground shrink-0" />
          )}
          <span className="text-xs text-foreground truncate">{node.name}</span>
        </div>
        {expanded &&
          node.children.map((child) => (
            <FileTreeItem
              key={child.path}
              node={child}
              depth={depth + 1}
              onStage={onStage}
              onUnstage={onUnstage}
              onDiff={onDiff}
              actionLabel={actionLabel}
              isStaged={isStaged}
            />
          ))}
      </div>
    );
  }

  return (
    <div
      className="group flex items-center gap-1 py-0.5 px-1 rounded hover:bg-muted/40 cursor-pointer"
      style={{ paddingLeft: `${depth * 20 + 8}px` }}
      onClick={() => onDiff?.(node.path)}
    >
      <StatusIcon.icon className={cn("w-3.5 h-3.5 shrink-0", StatusIcon.className)} />
      <span className="text-xs text-foreground truncate flex-1">{node.name}</span>
      <div className="flex items-center gap-0.5 opacity-0 group-hover:opacity-100 transition-opacity">
        {isStaged && onUnstage && (
          <Button
            variant="ghost"
            size="icon"
            className="h-5 w-5"
            onClick={(e) => {
              e.stopPropagation();
              onUnstage(node.path);
            }}
          >
            <X className="w-3 h-3" />
          </Button>
        )}
        {!isStaged && onStage && (
          <Button
            variant="ghost"
            size="icon"
            className="h-5 w-5 text-emerald-400"
            onClick={(e) => {
              e.stopPropagation();
              onStage(node.path);
            }}
          >
            <GitCommitHorizontal className="w-3 h-3" />
          </Button>
        )}
      </div>
    </div>
  );
}

function CollapsibleSection({
  title,
  count,
  children,
  emptyText,
  headerAction,
  headerActionLabel,
  defaultCollapsed = false,
}: {
  title: string;
  count: number;
  children: ReactNode;
  emptyText: string;
  headerAction?: () => void;
  headerActionLabel?: string;
  defaultCollapsed?: boolean;
}) {
  const [collapsed, setCollapsed] = useState(defaultCollapsed);

  return (
    <div className="border-b border-border last:border-b-0">
      <div
        className="flex items-center justify-between px-2 py-1.5 hover:bg-muted/30 cursor-pointer select-none"
        onClick={() => setCollapsed(!collapsed)}
      >
        <div className="flex items-center gap-1.5">
          {collapsed ? (
            <ChevronRight className="w-3.5 h-3.5 text-muted-foreground" />
          ) : (
            <ChevronDown className="w-3.5 h-3.5 text-muted-foreground" />
          )}
          <span className="text-xs font-medium text-foreground">{title}</span>
          <span className="text-[10px] text-muted-foreground bg-muted px-1.5 py-0.5 rounded-full">
            {count}
          </span>
        </div>
        {headerAction && headerActionLabel && count > 0 && (
          <Button
            variant="ghost"
            size="sm"
            className="h-5 text-[10px] px-1.5"
            onClick={(e) => {
              e.stopPropagation();
              headerAction();
            }}
          >
            {headerActionLabel}
          </Button>
        )}
      </div>
      {!collapsed && (
        <div className="pb-1">
          {count === 0 ? (
            <div className="text-[11px] text-muted-foreground px-3 py-2 italic">{emptyText}</div>
          ) : (
            children
          )}
        </div>
      )}
    </div>
  );
}

export const GitPanel = memo(function GitPanel({
  sessionId,
  workingDirectory,
  open,
  onOpenChange,
  onOpenFile,
}: GitPanelProps) {
  const gitStatus = useGitStatus(sessionId ?? "");
  const isLoading = useGitStatusLoading(sessionId ?? "");
  const storedMessage = useGitCommitMessage(sessionId ?? "");
  const [commitSummary, setCommitSummary] = useState("");
  const [commitDescription, setCommitDescription] = useState("");
  const setGitStatus = useStore((state) => state.setGitStatus);
  const setGitStatusLoading = useStore((state) => state.setGitStatusLoading);
  const setGitCommitMessage = useStore((state) => state.setGitCommitMessage);

  const [diffContent, setDiffContent] = useState<string | null>(null);
  const [diffFile, setDiffFile] = useState<string | null>(null);
  const [isCommitting, setIsCommitting] = useState(false);
  const [isPushing, setIsPushing] = useState(false);
  const [isGenerating, setIsGenerating] = useState(false);

  const changes = useMemo(() => mapStatusEntries(gitStatus?.entries ?? []), [gitStatus]);
  const groups = useMemo(() => splitChanges(changes), [changes]);
  const branchLabel = gitStatus?.branch ?? "";

  // Combine unstaged and untracked into a single "unstaged" group (like GitKraken)
  const allUnstaged = useMemo(
    () => [...groups.unstaged, ...groups.untracked],
    [groups.unstaged, groups.untracked]
  );
  const totalChanges = groups.staged.length + allUnstaged.length;

  const stagedTree = useMemo(() => buildFileTree(groups.staged), [groups.staged]);
  const unstagedTree = useMemo(() => buildFileTree(allUnstaged), [allUnstaged]);
  const conflictsTree = useMemo(() => buildFileTree(groups.conflicts), [groups.conflicts]);

  // Clear diff if the displayed file is no longer in the changes list
  useEffect(() => {
    if (diffFile) {
      const allPaths = changes.map((c) => c.path);
      if (!allPaths.includes(diffFile)) {
        setDiffFile(null);
        setDiffContent(null);
      }
    }
  }, [changes, diffFile]);

  // Sync stored message to summary/description on mount
  useEffect(() => {
    if (storedMessage) {
      const lines = storedMessage.split("\n");
      setCommitSummary(lines[0] || "");
      setCommitDescription(lines.slice(1).join("\n").trim());
    }
  }, []);

  const summaryRemaining = SUMMARY_MAX_LENGTH - commitSummary.length;

  const refreshStatus = useCallback(async () => {
    if (!sessionId || !workingDirectory) return;
    setGitStatusLoading(sessionId, true);
    try {
      const status = await fetchGitStatus(workingDirectory);
      setGitStatus(sessionId, status);
    } catch (error) {
      console.error("Failed to fetch git status", error);
      notify.error("Failed to load git status");
      setGitStatus(sessionId, null);
    } finally {
      setGitStatusLoading(sessionId, false);
    }
  }, [sessionId, workingDirectory, setGitStatus, setGitStatusLoading]);

  const handleStage = useCallback(
    async (files: string[]) => {
      if (!sessionId || !workingDirectory || files.length === 0) return;
      setGitStatusLoading(sessionId, true);
      try {
        await gitStage(workingDirectory, files);
        await refreshStatus();
      } catch (error) {
        notify.error(`Stage failed: ${String(error)}`);
      } finally {
        setGitStatusLoading(sessionId, false);
      }
    },
    [sessionId, workingDirectory, refreshStatus, setGitStatusLoading]
  );

  const handleUnstage = useCallback(
    async (files: string[]) => {
      if (!sessionId || !workingDirectory || files.length === 0) return;
      setGitStatusLoading(sessionId, true);
      try {
        await gitUnstage(workingDirectory, files);
        await refreshStatus();
      } catch (error) {
        notify.error(`Unstage failed: ${String(error)}`);
      } finally {
        setGitStatusLoading(sessionId, false);
      }
    },
    [sessionId, workingDirectory, refreshStatus, setGitStatusLoading]
  );

  const handleDiff = useCallback(
    async (file: string, staged = false) => {
      if (!workingDirectory) return;
      try {
        const result = await gitDiff(workingDirectory, file, staged);
        setDiffFile(file);
        setDiffContent(result.diff || "(no diff)");
      } catch (error) {
        notify.error(`Diff failed: ${String(error)}`);
      }
    },
    [workingDirectory]
  );

  const handleCommit = useCallback(async () => {
    if (!sessionId || !workingDirectory || !commitSummary.trim()) {
      return;
    }
    setIsCommitting(true);
    try {
      const fullMessage = commitDescription.trim()
        ? `${commitSummary.trim()}\n\n${commitDescription.trim()}`
        : commitSummary.trim();
      await gitCommit(workingDirectory, fullMessage);
      setGitCommitMessage(sessionId, "");
      setCommitSummary("");
      setCommitDescription("");
      await refreshStatus();
      notify.success("Commit created");
    } catch (error) {
      notify.error(`Commit failed: ${String(error)}`);
    } finally {
      setIsCommitting(false);
    }
  }, [
    commitSummary,
    commitDescription,
    workingDirectory,
    sessionId,
    refreshStatus,
    setGitCommitMessage,
  ]);

  const handlePush = useCallback(async () => {
    if (!workingDirectory) return;
    setIsPushing(true);
    try {
      await gitPush(workingDirectory);
      await refreshStatus();
      notify.success("Pushed to remote");
    } catch (error) {
      notify.error(`Push failed: ${String(error)}`);
    } finally {
      setIsPushing(false);
    }
  }, [workingDirectory, refreshStatus]);

  const handleGenerateCommitMessage = useCallback(async () => {
    if (!sessionId || !workingDirectory || groups.staged.length === 0) return;
    setIsGenerating(true);
    try {
      // Get diff for all staged files
      const stagedPaths = groups.staged.map((c) => c.path);
      const diffResult = await gitDiff(workingDirectory, stagedPaths.join(" "), true);
      const fileSummary = `${stagedPaths.length} file${stagedPaths.length === 1 ? "" : "s"}: ${stagedPaths.slice(0, 3).join(", ")}${stagedPaths.length > 3 ? ", ..." : ""}`;

      const response = await generateCommitMessage(sessionId, diffResult.diff, fileSummary);
      setCommitSummary(response.summary);
      setCommitDescription(response.description);
      const full = response.summary + (response.description ? `\n\n${response.description}` : "");
      setGitCommitMessage(sessionId, full);
    } catch (error) {
      notify.error(`Failed to generate commit message: ${String(error)}`);
    } finally {
      setIsGenerating(false);
    }
  }, [sessionId, workingDirectory, groups.staged, setGitCommitMessage]);

  // Auto-refresh when opened
  useEffect(() => {
    if (open) {
      void refreshStatus();
    }
  }, [open, refreshStatus]);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        className="h-[calc(100vh-4rem)] p-0 gap-0 bg-card border-[var(--border-medium)] flex flex-col"
        style={{ maxWidth: "calc(100vw - 4rem)", width: "calc(100vw - 4rem)" }}
      >
        <DialogHeader className="px-4 py-3 border-b border-[var(--border-medium)] shrink-0">
          <DialogTitle className="text-foreground flex items-center gap-2">
            <GitBranch className="h-5 w-5 text-accent" />
            Git Changes
            {branchLabel && (
              <span className="text-sm font-normal text-muted-foreground">on {branchLabel}</span>
            )}
          </DialogTitle>
          <DialogDescription className="text-muted-foreground flex items-center gap-2">
            {totalChanges > 0 ? (
              <>
                {totalChanges} file{totalChanges === 1 ? "" : "s"} changed
              </>
            ) : (
              "Working tree clean"
            )}
            {(gitStatus?.ahead ?? 0) > 0 && (
              <span
                className="flex items-center gap-0.5 text-emerald-400"
                title={`${gitStatus?.ahead} commit${gitStatus?.ahead === 1 ? "" : "s"} to push`}
              >
                <ArrowUp className="w-3.5 h-3.5" />
                <span className="text-xs">{gitStatus?.ahead}</span>
              </span>
            )}
            {(gitStatus?.behind ?? 0) > 0 && (
              <span
                className="flex items-center gap-0.5 text-amber-400"
                title={`${gitStatus?.behind} commit${gitStatus?.behind === 1 ? "" : "s"} to pull`}
              >
                <ArrowDown className="w-3.5 h-3.5" />
                <span className="text-xs">{gitStatus?.behind}</span>
              </span>
            )}
            <Button
              variant="ghost"
              size="icon"
              className="h-5 w-5 ml-auto"
              onClick={() => void refreshStatus()}
              disabled={isLoading}
              title="Refresh"
            >
              {isLoading ? (
                <Loader2 className="w-3 h-3 animate-spin" />
              ) : (
                <RefreshCcw className="w-3 h-3" />
              )}
            </Button>
          </DialogDescription>
        </DialogHeader>

        {/* Two-column layout: Diff on left, file tree on right */}
        <ResizablePanelGroup direction="horizontal" className="flex-1 min-h-0">
          {/* Left panel: Diff preview */}
          <ResizablePanel defaultSize={75} minSize={30}>
            <div className="h-full flex flex-col min-h-0 bg-background">
              {diffFile && diffContent ? (
                <>
                  <div className="flex items-center justify-between px-3 py-2 border-b border-[var(--border-subtle)] shrink-0">
                    <span className="text-xs font-medium text-foreground truncate">
                      Diff: {diffFile}
                    </span>
                    <Button
                      variant="ghost"
                      size="icon"
                      className="h-6 w-6"
                      onClick={() => {
                        setDiffContent(null);
                        setDiffFile(null);
                      }}
                    >
                      <X className="w-3.5 h-3.5" />
                    </Button>
                  </div>
                  <ScrollArea className="flex-1 overflow-auto">
                    <div className="p-3">
                      <DiffView content={diffContent} />
                    </div>
                  </ScrollArea>
                </>
              ) : (
                <div className="flex-1 flex items-center justify-center text-muted-foreground text-sm">
                  <div className="text-center">
                    <FileDiff className="w-8 h-8 mx-auto mb-2 opacity-50" />
                    <p>Select a file to view diff</p>
                  </div>
                </div>
              )}
            </div>
          </ResizablePanel>

          <ResizableHandle />

          {/* Right panel: File tree + commit composer */}
          <ResizablePanel defaultSize={25} minSize={15}>
            <ResizablePanelGroup direction="vertical" className="h-full">
              {/* File tree panel */}
              <ResizablePanel defaultSize={60} minSize={20}>
                <ScrollArea className="h-full">
                  <div>
                    {/* Conflicts */}
                    {groups.conflicts.length > 0 && (
                      <CollapsibleSection
                        title="Conflicts"
                        count={groups.conflicts.length}
                        emptyText="No conflicts"
                      >
                        {conflictsTree.map((node) => (
                          <FileTreeItem
                            key={node.path}
                            node={node}
                            depth={0}
                            onStage={onOpenFile ? (path) => onOpenFile(path) : undefined}
                            onDiff={(path) => handleDiff(path)}
                            actionLabel="Open"
                            isStaged={false}
                          />
                        ))}
                      </CollapsibleSection>
                    )}

                    <CollapsibleSection
                      title="Unstaged Changes"
                      count={allUnstaged.length}
                      emptyText="Working tree clean"
                      headerAction={
                        allUnstaged.length > 0
                          ? () => handleStage(allUnstaged.map((c) => c.path))
                          : undefined
                      }
                      headerActionLabel="Stage All"
                    >
                      {unstagedTree.map((node) => (
                        <FileTreeItem
                          key={node.path}
                          node={node}
                          depth={0}
                          onStage={(path) => handleStage([path])}
                          onDiff={(path) => handleDiff(path)}
                          actionLabel="Stage"
                          isStaged={false}
                        />
                      ))}
                    </CollapsibleSection>

                    <CollapsibleSection
                      title="Staged Changes"
                      count={groups.staged.length}
                      emptyText="No staged changes"
                      headerAction={
                        groups.staged.length > 0
                          ? () => handleUnstage(groups.staged.map((c) => c.path))
                          : undefined
                      }
                      headerActionLabel="Unstage All"
                    >
                      {stagedTree.map((node) => (
                        <FileTreeItem
                          key={node.path}
                          node={node}
                          depth={0}
                          onUnstage={(path) => handleUnstage([path])}
                          onDiff={(path) => handleDiff(path, true)}
                          actionLabel="Unstage"
                          isStaged={true}
                        />
                      ))}
                    </CollapsibleSection>
                  </div>
                </ScrollArea>
              </ResizablePanel>

              <ResizableHandle />

              {/* Commit composer panel */}
              <ResizablePanel defaultSize={40} minSize={15}>
                <div className="h-full flex flex-col p-2 gap-2">
                  <div className="flex items-center justify-between shrink-0">
                    <span className="text-[10px] font-medium text-muted-foreground uppercase tracking-wide">
                      Commit Message
                    </span>
                    <Button
                      variant="ghost"
                      size="sm"
                      className="h-5 text-[10px] px-1.5 gap-1 text-violet-400 hover:text-violet-300"
                      disabled={isGenerating || groups.staged.length === 0}
                      onClick={() => void handleGenerateCommitMessage()}
                      title="Generate commit message with AI"
                    >
                      {isGenerating ? (
                        <Loader2 className="w-3 h-3 animate-spin" />
                      ) : (
                        <Sparkles className="w-3 h-3" />
                      )}
                      Generate
                    </Button>
                  </div>
                  <div className="relative shrink-0">
                    <Input
                      placeholder="Summary (required)"
                      value={commitSummary}
                      onChange={(e) => {
                        setCommitSummary(e.target.value);
                        const full =
                          e.target.value + (commitDescription ? `\n\n${commitDescription}` : "");
                        setGitCommitMessage(sessionId ?? "", full);
                      }}
                      className="h-8 text-xs pr-10"
                      maxLength={SUMMARY_MAX_LENGTH + 20}
                    />
                    <span
                      className={cn(
                        "absolute right-2 top-1/2 -translate-y-1/2 text-[10px]",
                        summaryRemaining < 0
                          ? "text-red-400"
                          : summaryRemaining < 10
                            ? "text-amber-400"
                            : "text-muted-foreground"
                      )}
                    >
                      {summaryRemaining}
                    </span>
                  </div>
                  <Textarea
                    placeholder="Description (optional)"
                    value={commitDescription}
                    onChange={(e) => {
                      setCommitDescription(e.target.value);
                      const full = commitSummary + (e.target.value ? `\n\n${e.target.value}` : "");
                      setGitCommitMessage(sessionId ?? "", full);
                    }}
                    className="flex-1 text-xs resize-none min-h-0"
                  />
                  <Button
                    className="w-full h-8 text-xs shrink-0 bg-emerald-600 hover:bg-emerald-500 text-white"
                    disabled={isCommitting || !commitSummary.trim() || groups.staged.length === 0}
                    onClick={() => void handleCommit()}
                  >
                    {isCommitting ? (
                      <Loader2 className="w-3.5 h-3.5 animate-spin mr-1.5" />
                    ) : groups.staged.length === 0 ? (
                      "Stage Changes to Commit"
                    ) : (
                      <>
                        <GitCommitHorizontal className="w-3.5 h-3.5 mr-1.5" />
                        Commit {groups.staged.length} file{groups.staged.length === 1 ? "" : "s"}
                      </>
                    )}
                  </Button>
                  {(gitStatus?.ahead ?? 0) > 0 && (
                    <Button
                      className="w-full h-8 text-xs shrink-0 bg-sky-600 hover:bg-sky-500 text-white"
                      disabled={isPushing}
                      onClick={() => void handlePush()}
                    >
                      {isPushing ? (
                        <Loader2 className="w-3.5 h-3.5 animate-spin" />
                      ) : (
                        <>Push {gitStatus?.ahead} commit{gitStatus?.ahead === 1 ? "" : "s"}</>
                      )}
                    </Button>
                  )}
                </div>
              </ResizablePanel>
            </ResizablePanelGroup>
          </ResizablePanel>
        </ResizablePanelGroup>
      </DialogContent>
    </Dialog>
  );
});
