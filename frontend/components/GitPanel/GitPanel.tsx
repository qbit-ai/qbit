import {
  CheckCircle2,
  FileDiff,
  GitBranch,
  GripVertical,
  Loader2,
  RefreshCcw,
  ScrollText,
  X,
} from "lucide-react";
import { memo, useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Separator } from "@/components/ui/separator";
import { Textarea } from "@/components/ui/textarea";
import { gitCommit, gitDiff, gitStage, gitStatus as fetchGitStatus, gitUnstage } from "@/lib/tauri";
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

const MIN_WIDTH = 320;
const MAX_WIDTH = 560;
const DEFAULT_WIDTH = 380;

function ChangeRow({
  change,
  onStage,
  onUnstage,
  onDiff,
  primaryActionLabel,
}: {
  change: GitChange;
  onStage?: () => void;
  onUnstage?: () => void;
  onDiff?: () => void;
  primaryActionLabel: string;
}) {
  const badge = useMemo(() => {
    switch (change.kind) {
      case "added":
        return { label: "A", className: "bg-emerald-500/20 text-emerald-300" };
      case "deleted":
        return { label: "D", className: "bg-red-500/20 text-red-300" };
      case "renamed":
        return { label: "R", className: "bg-amber-500/20 text-amber-200" };
      case "modified":
        return { label: "M", className: "bg-sky-500/20 text-sky-200" };
      case "conflict":
        return { label: "U", className: "bg-pink-500/20 text-pink-200" };
      case "untracked":
        return { label: "?", className: "bg-slate-500/30 text-slate-200" };
      default:
        return { label: "•", className: "bg-slate-500/30 text-slate-200" };
    }
  }, [change.kind]);

  const pathParts = change.path.split("/");
  const fileName = pathParts.pop();
  const dir = pathParts.join("/");

  return (
    <div className="flex items-center gap-2 rounded-md px-2 py-1.5 hover:bg-muted/40">
      <span
        className={cn(
          "flex h-5 w-5 items-center justify-center rounded-full text-[11px] font-semibold",
          badge.className
        )}
      >
        {badge.label}
      </span>
      <div className="min-w-0 flex-1">
        <div className="text-sm text-foreground truncate">{fileName}</div>
        <div className="text-[11px] text-muted-foreground truncate">{dir}</div>
      </div>
      <div className="flex items-center gap-1">
        {onDiff && (
          <Button variant="ghost" size="icon" className="h-7 w-7" onClick={onDiff}>
            <FileDiff className="w-4 h-4" />
          </Button>
        )}
        {onStage && (
          <Button variant="secondary" size="sm" className="h-7" onClick={onStage}>
            {primaryActionLabel}
          </Button>
        )}
        {onUnstage && (
          <Button variant="outline" size="sm" className="h-7" onClick={onUnstage}>
            {primaryActionLabel}
          </Button>
        )}
      </div>
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
  const commitMessage = useGitCommitMessage(sessionId ?? "");
  const setGitStatus = useStore((state) => state.setGitStatus);
  const setGitStatusLoading = useStore((state) => state.setGitStatusLoading);
  const setGitCommitMessage = useStore((state) => state.setGitCommitMessage);

  const [width, setWidth] = useState(DEFAULT_WIDTH);
  const [diffContent, setDiffContent] = useState<string | null>(null);
  const [diffFile, setDiffFile] = useState<string | null>(null);
  const isResizing = useRef(false);
  const panelRef = useRef<HTMLDivElement>(null);
  const [isCommitting, setIsCommitting] = useState(false);

  const changes = useMemo(() => mapStatusEntries(gitStatus?.entries ?? []), [gitStatus]);
  const groups = useMemo(() => splitChanges(changes), [changes]);
  const branchLabel = gitStatus?.branch ?? "";

  const startResizing = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    isResizing.current = true;
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";
  }, []);

  useEffect(() => {
    const handleMouseMove = (e: MouseEvent) => {
      if (!isResizing.current || !panelRef.current) return;
      const newWidth = window.innerWidth - e.clientX;
      if (newWidth >= MIN_WIDTH && newWidth <= MAX_WIDTH) {
        setWidth(newWidth);
      }
    };

    const handleMouseUp = () => {
      if (isResizing.current) {
        isResizing.current = false;
        document.body.style.cursor = "";
        document.body.style.userSelect = "";
      }
    };

    document.addEventListener("mousemove", handleMouseMove);
    document.addEventListener("mouseup", handleMouseUp);
    return () => {
      document.removeEventListener("mousemove", handleMouseMove);
      document.removeEventListener("mouseup", handleMouseUp);
    };
  }, []);

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
    if (!sessionId || !workingDirectory || !commitMessage.trim()) {
      return;
    }
    setIsCommitting(true);
    try {
      await gitCommit(workingDirectory, commitMessage.trim());
      setGitCommitMessage(sessionId, "");
      await refreshStatus();
      notify.success("Commit created");
    } catch (error) {
      notify.error(`Commit failed: ${String(error)}`);
    } finally {
      setIsCommitting(false);
    }
  }, [commitMessage, workingDirectory, sessionId, refreshStatus, setGitCommitMessage]);

  // Auto-refresh when opened
  useEffect(() => {
    if (open) {
      void refreshStatus();
    }
  }, [open, refreshStatus]);

  if (!open) return null;

  return (
    <div
      ref={panelRef}
      className="bg-card border-l border-border flex flex-col relative"
      style={{ width: `${width}px`, minWidth: `${MIN_WIDTH}px`, maxWidth: `${MAX_WIDTH}px` }}
    >
      {/* Resize handle */}
      {/* biome-ignore lint/a11y/noStaticElementInteractions: resize handle is mouse-only */}
      <div
        className="absolute top-0 left-0 w-1 h-full cursor-col-resize hover:bg-[var(--ansi-blue)] transition-colors z-10 group"
        onMouseDown={startResizing}
      >
        <div className="absolute top-1/2 left-0 -translate-y-1/2 opacity-0 group-hover:opacity-100 transition-opacity">
          <GripVertical className="w-3 h-3 text-muted-foreground" />
        </div>
      </div>

      {/* Header */}
      <div className="flex items-center justify-between px-3 py-2 border-b border-border">
        <div className="flex items-center gap-2 min-w-0">
          <GitBranch className="w-4 h-4 text-[#7aa2f7] shrink-0" />
          <div className="flex flex-col min-w-0">
            <span className="text-sm font-medium truncate">Git</span>
            {branchLabel && (
              <span className="text-[11px] text-muted-foreground truncate">{branchLabel}</span>
            )}
          </div>
        </div>
        <div className="flex items-center gap-1">
          <Button
            variant="ghost"
            size="icon"
            className="h-7 w-7"
            onClick={() => void refreshStatus()}
            disabled={isLoading}
          >
            {isLoading ? <Loader2 className="w-4 h-4 animate-spin" /> : <RefreshCcw className="w-4 h-4" />}
          </Button>
          <Button variant="ghost" size="icon" className="h-7 w-7" onClick={() => onOpenChange(false)}>
            <X className="w-4 h-4" />
          </Button>
        </div>
      </div>

      <div className="flex items-center gap-2 px-3 py-2 text-xs text-muted-foreground border-b border-border">
        <div className="flex items-center gap-1">
          <ScrollText className="w-3.5 h-3.5" />
          <span>Staged {groups.staged.length}</span>
        </div>
        <Separator orientation="vertical" className="h-4" />
        <div className="flex items-center gap-1">
          <ScrollText className="w-3.5 h-3.5" />
          <span>Unstaged {groups.unstaged.length}</span>
        </div>
        <Separator orientation="vertical" className="h-4" />
        <div className="flex items-center gap-1">
          <ScrollText className="w-3.5 h-3.5" />
          <span>Untracked {groups.untracked.length}</span>
        </div>
      </div>

      <ScrollArea className="flex-1">
        <div className="p-3 space-y-4">
          {/* Conflicts */}
          {groups.conflicts.length > 0 && (
            <Section
              title={`Conflicts (${groups.conflicts.length})`}
              emptyText="No conflicts"
              renderItem={(item) => (
                <ChangeRow
                  key={item.path}
                  change={item}
                  primaryActionLabel="Open"
                  onStage={onOpenFile ? () => onOpenFile(item.path) : undefined}
                  onDiff={() => handleDiff(item.path)}
                />
              )}
              items={groups.conflicts}
            />
          )}

          <Section
            title={`Staged (${groups.staged.length})`}
            emptyText="No staged changes"
            items={groups.staged}
            renderItem={(item) => (
              <ChangeRow
                key={item.path}
                change={item}
                primaryActionLabel="Unstage"
                onUnstage={() => handleUnstage([item.path])}
                onDiff={() => handleDiff(item.path, true)}
              />
            )}
            headerAction={
              groups.staged.length > 0
                ? () => handleUnstage(groups.staged.map((c) => c.path))
                : undefined
            }
            headerActionLabel="Unstage all"
          />

          <Section
            title={`Unstaged (${groups.unstaged.length})`}
            emptyText="Working tree clean"
            items={groups.unstaged}
            renderItem={(item) => (
              <ChangeRow
                key={item.path}
                change={item}
                primaryActionLabel="Stage"
                onStage={() => handleStage([item.path])}
                onDiff={() => handleDiff(item.path)}
              />
            )}
            headerAction={
              groups.unstaged.length > 0
                ? () => handleStage(groups.unstaged.map((c) => c.path))
                : undefined
            }
            headerActionLabel="Stage all"
          />

          <Section
            title={`Untracked (${groups.untracked.length})`}
            emptyText="No untracked files"
            items={groups.untracked}
            renderItem={(item) => (
              <ChangeRow
                key={item.path}
                change={item}
                primaryActionLabel="Stage"
                onStage={() => handleStage([item.path])}
              />
            )}
            headerAction={
              groups.untracked.length > 0
                ? () => handleStage(groups.untracked.map((c) => c.path))
                : undefined
            }
            headerActionLabel="Stage all"
          />

          {/* Diff preview */}
          {diffFile && (
            <div className="rounded-md border border-border bg-muted/30 p-3">
              <div className="flex items-center justify-between mb-2">
                <div className="text-sm font-medium text-foreground">Diff: {diffFile}</div>
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-7 w-7"
                  onClick={() => {
                    setDiffContent(null);
                    setDiffFile(null);
                  }}
                >
                  <X className="w-4 h-4" />
                </Button>
              </div>
              <pre className="text-xs bg-background border border-border rounded-sm p-2 overflow-auto max-h-64 whitespace-pre-wrap">
{diffContent}
              </pre>
            </div>
          )}

          {/* Commit composer */}
          <div className="border border-border rounded-md p-3 bg-muted/10">
            <div className="flex items-center gap-2 mb-2">
              <CheckCircle2 className="w-4 h-4 text-[#7aa2f7]" />
              <div className="text-sm font-medium">Commit</div>
            </div>
            <Textarea
              placeholder="Commit message"
              value={commitMessage}
              onChange={(e) => setGitCommitMessage(sessionId ?? "", e.target.value)}
              className="min-h-[80px] text-sm"
            />
            <div className="flex justify-between items-center mt-2">
              <span className="text-[11px] text-muted-foreground">
                Staged: {groups.staged.length} file{groups.staged.length === 1 ? "" : "s"}
              </span>
              <Button
                size="sm"
                disabled={isCommitting || !commitMessage.trim() || groups.staged.length === 0}
                onClick={() => void handleCommit()}
              >
                {isCommitting ? <Loader2 className="w-4 h-4 animate-spin" /> : "Commit"}
              </Button>
            </div>
          </div>
        </div>
      </ScrollArea>

      <div className="px-3 py-2 border-t border-border text-xs text-muted-foreground flex items-center gap-2">
        <kbd className="bg-muted px-1 py-0.5 rounded text-[10px]">Cmd+Shift+G</kbd> to toggle
        {gitStatus?.behind ? <span className="text-amber-400">↓ {gitStatus.behind}</span> : null}
        {gitStatus?.ahead ? <span className="text-emerald-400">↑ {gitStatus.ahead}</span> : null}
      </div>
    </div>
  );
});

function Section<T extends { path: string }>({
  title,
  items,
  renderItem,
  emptyText,
  headerAction,
  headerActionLabel,
}: {
  title: string;
  items: T[];
  emptyText: string;
  renderItem: (item: T) => ReactNode;
  headerAction?: () => void;
  headerActionLabel?: string;
}) {
  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between">
        <div className="text-xs font-semibold text-muted-foreground uppercase tracking-wide">
          {title}
        </div>
        {headerAction && headerActionLabel && (
          <Button variant="ghost" size="sm" className="h-7" onClick={headerAction}>
            {headerActionLabel}
          </Button>
        )}
      </div>
      {items.length === 0 ? (
        <div className="text-xs text-muted-foreground px-2 py-1.5 border border-dashed border-border rounded">
          {emptyText}
        </div>
      ) : (
        <div className="space-y-1">{items.map((item) => renderItem(item))}</div>
      )}
    </div>
  );
}
