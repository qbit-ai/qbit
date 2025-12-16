import { listen } from "@tauri-apps/api/event";
import {
  Check,
  ChevronDown,
  ChevronRight,
  Clock,
  FileCode,
  FileText,
  GitCommit,
  GripVertical,
  Package,
  RefreshCw,
  ScrollText,
  X,
} from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { Markdown } from "@/components/Markdown/Markdown";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  type Artifact,
  getAppliedPatches,
  getCurrentSession,
  getPendingArtifacts,
  getSessionLog,
  getSessionState,
  getStagedPatches,
  previewArtifact,
  type SidecarEventType,
  type StagedPatch,
} from "@/lib/sidecar";
import { cn } from "@/lib/utils";

interface ContextPanelProps {
  /** Session ID to show context for (uses current session if not provided) */
  sessionId?: string;
  /** Whether the panel is open */
  open: boolean;
  /** Callback when panel should close */
  onOpenChange: (open: boolean) => void;
}

type TabId = "state" | "log" | "patches" | "artifacts";

const MIN_WIDTH = 300;
const MAX_WIDTH = 900;
const DEFAULT_WIDTH = 450;

/**
 * Side panel showing the current session's markdown state and log.
 * Displays the state.md (LLM-managed session context) and log.md (event history).
 * Renders inline as part of the flex layout (not a modal overlay).
 */
export function ContextPanel({ sessionId, open, onOpenChange }: ContextPanelProps) {
  const [activeTab, setActiveTab] = useState<TabId>("state");
  const [stateContent, setStateContent] = useState<string>("");
  const [logContent, setLogContent] = useState<string>("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [resolvedSessionId, setResolvedSessionId] = useState<string | null>(null);

  // Patches state
  const [stagedPatches, setStagedPatches] = useState<StagedPatch[]>([]);
  const [appliedPatches, setAppliedPatches] = useState<StagedPatch[]>([]);
  const [selectedPatchId, setSelectedPatchId] = useState<number | null>(null);

  // Artifacts state
  const [pendingArtifacts, setPendingArtifacts] = useState<Artifact[]>([]);
  const [selectedArtifact, setSelectedArtifact] = useState<string | null>(null);
  const [artifactPreview, setArtifactPreview] = useState<string | null>(null);

  // Resize state
  const [width, setWidth] = useState(DEFAULT_WIDTH);
  const isResizing = useRef(false);
  const panelRef = useRef<HTMLDivElement>(null);

  // Handle resize
  const startResizing = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    isResizing.current = true;
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";
  }, []);

  useEffect(() => {
    const handleMouseMove = (e: MouseEvent) => {
      if (!isResizing.current || !panelRef.current) return;

      // Calculate new width based on distance from right edge of viewport
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

  // Fetch content for the current (or specified) session
  const fetchContent = useCallback(async () => {
    setLoading(true);
    setError(null);

    try {
      // Resolve session ID
      let sid: string | undefined = sessionId;
      if (!sid) {
        sid = (await getCurrentSession()) ?? undefined;
      }

      if (!sid) {
        setError(null);
        setStateContent(
          "No active capture session.\n\nSend a message to the AI to start context capture."
        );
        setLogContent(
          "No active capture session.\n\nSend a message to the AI to start context capture."
        );
        setStagedPatches([]);
        setAppliedPatches([]);
        setPendingArtifacts([]);
        setResolvedSessionId(null);
        return;
      }

      setResolvedSessionId(sid);

      // Fetch all data in parallel
      const [state, log, staged, applied, artifacts] = await Promise.all([
        getSessionState(sid).catch(() => ""),
        getSessionLog(sid).catch(() => ""),
        getStagedPatches(sid).catch(() => []),
        getAppliedPatches(sid).catch(() => []),
        getPendingArtifacts(sid).catch(() => []),
      ]);

      setStateContent(state || "(empty)");
      setLogContent(log || "(empty)");
      setStagedPatches(staged);
      setAppliedPatches(applied);
      setPendingArtifacts(artifacts);
    } catch (e) {
      // Tauri errors may be strings, not Error objects
      const message =
        e instanceof Error
          ? e.message
          : typeof e === "string"
            ? e
            : "Failed to fetch session content";
      setError(message);
    } finally {
      setLoading(false);
    }
  }, [sessionId]);

  // Fetch content when panel opens
  useEffect(() => {
    if (!open) return;
    fetchContent();
  }, [open, fetchContent]);

  // Subscribe to sidecar events for auto-refresh
  useEffect(() => {
    if (!open) return;

    const unlisten = listen<SidecarEventType>("sidecar-event", (event) => {
      const eventType = event.payload.event_type;
      // Auto-refresh on session and patch/artifact events
      if (
        eventType === "session_started" ||
        eventType === "session_ended" ||
        eventType === "patch_created" ||
        eventType === "patch_applied" ||
        eventType === "patch_discarded" ||
        eventType === "artifact_created" ||
        eventType === "artifact_applied" ||
        eventType === "artifact_discarded"
      ) {
        fetchContent();
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [open, fetchContent]);

  // Handle artifact preview loading
  useEffect(() => {
    if (!selectedArtifact || !resolvedSessionId) {
      setArtifactPreview(null);
      return;
    }

    setArtifactPreview(null);
    previewArtifact(resolvedSessionId, selectedArtifact)
      .then(setArtifactPreview)
      .catch(() => setArtifactPreview("Failed to load preview"));
  }, [selectedArtifact, resolvedSessionId]);

  // Get all patches combined with status
  const allPatches = [
    ...stagedPatches.map((p) => ({ ...p, status: "staged" as const })),
    ...appliedPatches.map((p) => ({ ...p, status: "applied" as const })),
  ].sort((a, b) => a.meta.id - b.meta.id);

  // Get selected patch
  const selectedPatch = allPatches.find((p) => p.meta.id === selectedPatchId) ?? null;

  // Get selected artifact data
  const selectedArtifactData =
    pendingArtifacts.find((a) => a.filename === selectedArtifact) ?? null;

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
          <FileText className="w-4 h-4 text-muted-foreground shrink-0" />
          <h2 className="text-sm font-medium truncate">Session Context</h2>
          {resolvedSessionId && (
            <span className="text-xs text-muted-foreground font-mono shrink-0">
              {resolvedSessionId.slice(0, 8)}...
            </span>
          )}
        </div>
        <div className="flex items-center gap-1 shrink-0">
          <Button
            variant="ghost"
            size="icon"
            className="h-6 w-6"
            onClick={fetchContent}
            disabled={loading}
          >
            <RefreshCw className={cn("w-3.5 h-3.5", loading && "animate-spin")} />
          </Button>
          <Button
            variant="ghost"
            size="icon"
            className="h-6 w-6"
            onClick={() => onOpenChange(false)}
          >
            <X className="w-3.5 h-3.5" />
          </Button>
        </div>
      </div>

      {/* Tabs */}
      <div className="flex border-b border-border">
        <button
          type="button"
          onClick={() => setActiveTab("state")}
          className={cn(
            "flex-1 px-3 py-1.5 text-xs font-medium transition-colors",
            activeTab === "state"
              ? "text-foreground border-b-2 border-[var(--ansi-blue)]"
              : "text-muted-foreground hover:text-foreground"
          )}
        >
          <FileText className="w-3.5 h-3.5 inline mr-1" />
          State
        </button>
        <button
          type="button"
          onClick={() => setActiveTab("log")}
          className={cn(
            "flex-1 px-3 py-1.5 text-xs font-medium transition-colors",
            activeTab === "log"
              ? "text-foreground border-b-2 border-[var(--ansi-blue)]"
              : "text-muted-foreground hover:text-foreground"
          )}
        >
          <ScrollText className="w-3.5 h-3.5 inline mr-1" />
          Log
        </button>
        <button
          type="button"
          onClick={() => setActiveTab("patches")}
          className={cn(
            "flex-1 px-3 py-1.5 text-xs font-medium transition-colors",
            activeTab === "patches"
              ? "text-foreground border-b-2 border-[var(--ansi-blue)]"
              : "text-muted-foreground hover:text-foreground"
          )}
        >
          <GitCommit className="w-3.5 h-3.5 inline mr-1" />
          Patches
          {allPatches.length > 0 && (
            <span className="ml-1 text-[10px] bg-muted px-1 rounded">{allPatches.length}</span>
          )}
        </button>
        <button
          type="button"
          onClick={() => setActiveTab("artifacts")}
          className={cn(
            "flex-1 px-3 py-1.5 text-xs font-medium transition-colors",
            activeTab === "artifacts"
              ? "text-foreground border-b-2 border-[var(--ansi-blue)]"
              : "text-muted-foreground hover:text-foreground"
          )}
        >
          <Package className="w-3.5 h-3.5 inline mr-1" />
          Artifacts
          {pendingArtifacts.length > 0 && (
            <span className="ml-1 text-[10px] bg-muted px-1 rounded">
              {pendingArtifacts.length}
            </span>
          )}
        </button>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-hidden flex flex-col">
        {error ? (
          <div className="text-[var(--ansi-red)] text-xs p-3">{error}</div>
        ) : loading ? (
          <div className="text-muted-foreground text-xs animate-pulse p-3">Loading...</div>
        ) : activeTab === "state" ? (
          <ScrollArea className="flex-1">
            <div className="p-3 text-xs [&_h1]:text-base [&_h2]:text-sm [&_h3]:text-xs [&_p]:text-xs [&_li]:text-xs [&_code]:text-[10px] [&_pre]:text-[10px]">
              <Markdown content={stateContent} />
            </div>
          </ScrollArea>
        ) : activeTab === "log" ? (
          <ScrollArea className="flex-1">
            <div className="p-3 text-xs [&_h1]:text-base [&_h2]:text-sm [&_h3]:text-xs [&_p]:text-xs [&_li]:text-xs [&_code]:text-[10px] [&_pre]:text-[10px]">
              <Markdown content={logContent} />
            </div>
          </ScrollArea>
        ) : activeTab === "patches" ? (
          <PatchesView
            patches={allPatches}
            selectedPatchId={selectedPatchId}
            selectedPatch={selectedPatch}
            onSelectPatch={setSelectedPatchId}
          />
        ) : (
          <ArtifactsView
            artifacts={pendingArtifacts}
            selectedArtifact={selectedArtifact}
            selectedArtifactData={selectedArtifactData}
            artifactPreview={artifactPreview}
            onSelectArtifact={setSelectedArtifact}
          />
        )}
      </div>

      {/* Footer */}
      <div className="px-3 py-1.5 border-t border-border text-[10px] text-muted-foreground">
        {activeTab === "state"
          ? "LLM-managed session state (state.md)"
          : activeTab === "log"
            ? "Append-only event log (log.md)"
            : activeTab === "patches"
              ? "Git patches from this session (staged & applied)"
              : "Generated documentation artifacts"}
      </div>
    </div>
  );
}

// ============================================================================
// PatchesView Component - Split view with list and detail
// ============================================================================

interface PatchWithStatus extends StagedPatch {
  status: "staged" | "applied";
}

interface PatchesViewProps {
  patches: PatchWithStatus[];
  selectedPatchId: number | null;
  selectedPatch: PatchWithStatus | null;
  onSelectPatch: (id: number | null) => void;
}

function PatchesView({ patches, selectedPatchId, selectedPatch, onSelectPatch }: PatchesViewProps) {
  if (patches.length === 0) {
    return (
      <div className="flex-1 flex items-center justify-center">
        <div className="text-center text-muted-foreground">
          <GitCommit className="w-8 h-8 mx-auto mb-2 opacity-50" />
          <p className="text-sm">No patches generated yet</p>
          <p className="text-xs mt-1">Patches will appear here as you work</p>
        </div>
      </div>
    );
  }

  return (
    <div className="flex-1 flex flex-col overflow-hidden">
      {/* Patch list */}
      <div className="border-b border-border">
        <ScrollArea className="max-h-48">
          <div className="p-2 space-y-1">
            {patches.map((patch) => (
              <PatchListItem
                key={patch.meta.id}
                patch={patch}
                isSelected={selectedPatchId === patch.meta.id}
                onSelect={() =>
                  onSelectPatch(selectedPatchId === patch.meta.id ? null : patch.meta.id)
                }
              />
            ))}
          </div>
        </ScrollArea>
      </div>

      {/* Patch detail */}
      <div className="flex-1 overflow-hidden">
        {selectedPatch ? (
          <PatchDetail patch={selectedPatch} />
        ) : (
          <div className="h-full flex items-center justify-center text-muted-foreground text-sm">
            Select a patch to view details
          </div>
        )}
      </div>
    </div>
  );
}

interface PatchListItemProps {
  patch: PatchWithStatus;
  isSelected: boolean;
  onSelect: () => void;
}

function PatchListItem({ patch, isSelected, onSelect }: PatchListItemProps) {
  return (
    <button
      type="button"
      onClick={onSelect}
      className={cn(
        "w-full p-2 rounded text-left transition-colors border border-transparent",
        isSelected ? "bg-[var(--ansi-blue)]/15 border-[var(--ansi-blue)]/50" : "hover:bg-muted/50"
      )}
    >
      <div className="flex items-start gap-2">
        <div
          className={cn(
            "mt-0.5 p-1 rounded",
            patch.status === "applied" ? "bg-[var(--ansi-green)]/20" : "bg-[var(--ansi-yellow)]/20"
          )}
        >
          {patch.status === "applied" ? (
            <Check className="w-3 h-3 text-[var(--ansi-green)]" />
          ) : (
            <Clock className="w-3 h-3 text-[var(--ansi-yellow)]" />
          )}
        </div>
        <div className="flex-1 min-w-0">
          <p className="text-xs font-medium leading-tight line-clamp-2">{patch.subject}</p>
          <div className="flex items-center gap-2 mt-1 text-[10px] text-muted-foreground">
            <span>
              {patch.files.length} file{patch.files.length !== 1 ? "s" : ""}
            </span>
            <span>•</span>
            <span>{new Date(patch.meta.created_at).toLocaleTimeString()}</span>
            {patch.status === "applied" && patch.meta.applied_sha && (
              <>
                <span>•</span>
                <span className="font-mono">{patch.meta.applied_sha.slice(0, 7)}</span>
              </>
            )}
          </div>
        </div>
      </div>
    </button>
  );
}

interface PatchDetailProps {
  patch: PatchWithStatus;
}

function PatchDetail({ patch }: PatchDetailProps) {
  const [showFiles, setShowFiles] = useState(true);

  return (
    <ScrollArea className="h-full">
      <div className="p-3 space-y-3">
        {/* Header */}
        <div>
          <div className="flex items-center gap-2 mb-1">
            <span
              className={cn(
                "text-[10px] px-1.5 py-0.5 rounded font-medium",
                patch.status === "applied"
                  ? "bg-[var(--ansi-green)]/20 text-[var(--ansi-green)]"
                  : "bg-[var(--ansi-yellow)]/20 text-[var(--ansi-yellow)]"
              )}
            >
              {patch.status.toUpperCase()}
            </span>
            <span className="text-[10px] text-muted-foreground">
              #{patch.meta.id} • {new Date(patch.meta.created_at).toLocaleString()}
            </span>
          </div>
          <h3 className="text-sm font-medium leading-snug">{patch.subject}</h3>
        </div>

        {/* Commit message (if different from subject) */}
        {patch.message !== patch.subject && (
          <div>
            <p className="text-[10px] text-muted-foreground mb-1 font-medium">COMMIT MESSAGE</p>
            <pre className="text-xs font-mono whitespace-pre-wrap bg-muted p-2 rounded">
              {patch.message}
            </pre>
          </div>
        )}

        {/* Files */}
        {patch.files.length > 0 && (
          <div>
            <button
              type="button"
              onClick={() => setShowFiles(!showFiles)}
              className="flex items-center gap-1 text-[10px] text-muted-foreground mb-1 font-medium hover:text-foreground"
            >
              {showFiles ? (
                <ChevronDown className="w-3 h-3" />
              ) : (
                <ChevronRight className="w-3 h-3" />
              )}
              FILES CHANGED ({patch.files.length})
            </button>
            {showFiles && (
              <div className="space-y-0.5">
                {patch.files.map((file) => (
                  <div
                    key={file}
                    className="flex items-center gap-1.5 text-xs font-mono py-1 px-2 bg-muted/50 rounded"
                  >
                    <FileCode className="w-3 h-3 text-[var(--ansi-blue)] shrink-0" />
                    <span className="truncate" title={file}>
                      {file}
                    </span>
                  </div>
                ))}
              </div>
            )}
          </div>
        )}

        {/* Applied SHA */}
        {patch.status === "applied" && patch.meta.applied_sha && (
          <div>
            <p className="text-[10px] text-muted-foreground mb-1 font-medium">COMMIT SHA</p>
            <code className="text-xs font-mono bg-muted px-2 py-1 rounded">
              {patch.meta.applied_sha}
            </code>
          </div>
        )}

        {/* Diff */}
        {patch.patch_content && (
          <div>
            <p className="text-[10px] text-muted-foreground mb-1 font-medium">DIFF</p>
            <DiffViewer content={patch.patch_content} />
          </div>
        )}
      </div>
    </ScrollArea>
  );
}

// ============================================================================
// DiffViewer Component
// ============================================================================

interface DiffViewerProps {
  content: string;
}

function DiffViewer({ content }: DiffViewerProps) {
  const lines = content.split("\n");
  const diffSections: {
    file: string;
    lines: { text: string; type: "add" | "del" | "hunk" | "context" | "header" }[];
  }[] = [];

  let currentFile = "";
  let currentLines: { text: string; type: "add" | "del" | "hunk" | "context" | "header" }[] = [];

  for (const line of lines) {
    if (line.startsWith("diff --git ")) {
      // Save previous section
      if (currentFile && currentLines.length > 0) {
        diffSections.push({ file: currentFile, lines: currentLines });
      }
      // Extract file name
      const match = line.match(/diff --git a\/(.+?) b\//);
      currentFile = match?.[1] ?? "unknown";
      currentLines = [];
    } else if (line.startsWith("@@")) {
      currentLines.push({ text: line, type: "hunk" });
    } else if (line.startsWith("+") && !line.startsWith("+++")) {
      currentLines.push({ text: line, type: "add" });
    } else if (line.startsWith("-") && !line.startsWith("---")) {
      currentLines.push({ text: line, type: "del" });
    } else if (line.startsWith("index ") || line.startsWith("--- ") || line.startsWith("+++ ")) {
      currentLines.push({ text: line, type: "header" });
    } else if (currentFile) {
      currentLines.push({ text: line, type: "context" });
    }
  }

  // Save last section
  if (currentFile && currentLines.length > 0) {
    diffSections.push({ file: currentFile, lines: currentLines });
  }

  if (diffSections.length === 0) {
    return <p className="text-xs text-muted-foreground">No diff content</p>;
  }

  return (
    <div className="space-y-2">
      {diffSections.map((section, idx) => (
        <div
          key={`${section.file}-${idx}`}
          className="rounded overflow-hidden border border-border"
        >
          <div className="bg-muted px-2 py-1 text-[10px] font-mono text-muted-foreground border-b border-border">
            {section.file}
          </div>
          <pre className="text-[11px] font-mono overflow-x-auto">
            {section.lines.map((line, lineIdx) => (
              <div
                key={`${lineIdx}-${line.type}-${line.text.slice(0, 20)}`}
                className={cn(
                  "px-2 leading-5",
                  line.type === "add" && "bg-[var(--ansi-green)]/10 text-[var(--ansi-green)]",
                  line.type === "del" && "bg-[var(--ansi-red)]/10 text-[var(--ansi-red)]",
                  line.type === "hunk" && "bg-[var(--ansi-blue)]/10 text-[var(--ansi-blue)]",
                  line.type === "header" && "text-muted-foreground",
                  line.type === "context" && "text-foreground/70"
                )}
              >
                {line.text || " "}
              </div>
            ))}
          </pre>
        </div>
      ))}
    </div>
  );
}

// ============================================================================
// ArtifactsView Component - Split view with list and detail
// ============================================================================

interface ArtifactsViewProps {
  artifacts: Artifact[];
  selectedArtifact: string | null;
  selectedArtifactData: Artifact | null;
  artifactPreview: string | null;
  onSelectArtifact: (filename: string | null) => void;
}

function ArtifactsView({
  artifacts,
  selectedArtifact,
  selectedArtifactData,
  artifactPreview,
  onSelectArtifact,
}: ArtifactsViewProps) {
  if (artifacts.length === 0) {
    return (
      <div className="flex-1 flex items-center justify-center">
        <div className="text-center text-muted-foreground">
          <Package className="w-8 h-8 mx-auto mb-2 opacity-50" />
          <p className="text-sm">No artifacts generated yet</p>
          <p className="text-xs mt-1">Documentation artifacts will appear here</p>
        </div>
      </div>
    );
  }

  return (
    <div className="flex-1 flex flex-col overflow-hidden">
      {/* Artifact list */}
      <div className="border-b border-border">
        <ScrollArea className="max-h-48">
          <div className="p-2 space-y-1">
            {artifacts.map((artifact) => (
              <ArtifactListItem
                key={artifact.filename}
                artifact={artifact}
                isSelected={selectedArtifact === artifact.filename}
                onSelect={() =>
                  onSelectArtifact(
                    selectedArtifact === artifact.filename ? null : artifact.filename
                  )
                }
              />
            ))}
          </div>
        </ScrollArea>
      </div>

      {/* Artifact detail */}
      <div className="flex-1 overflow-hidden">
        {selectedArtifactData ? (
          <ArtifactDetail artifact={selectedArtifactData} preview={artifactPreview} />
        ) : (
          <div className="h-full flex items-center justify-center text-muted-foreground text-sm">
            Select an artifact to view details
          </div>
        )}
      </div>
    </div>
  );
}

interface ArtifactListItemProps {
  artifact: Artifact;
  isSelected: boolean;
  onSelect: () => void;
}

function ArtifactListItem({ artifact, isSelected, onSelect }: ArtifactListItemProps) {
  return (
    <button
      type="button"
      onClick={onSelect}
      className={cn(
        "w-full p-2 rounded text-left transition-colors border border-transparent",
        isSelected ? "bg-[var(--ansi-blue)]/15 border-[var(--ansi-blue)]/50" : "hover:bg-muted/50"
      )}
    >
      <div className="flex items-start gap-2">
        <div className="mt-0.5 p-1 rounded bg-[var(--ansi-cyan)]/20">
          <Package className="w-3 h-3 text-[var(--ansi-cyan)]" />
        </div>
        <div className="flex-1 min-w-0">
          <p className="text-xs font-medium font-mono">{artifact.filename}</p>
          <p className="text-[10px] text-muted-foreground mt-0.5 truncate">
            → {artifact.meta.target}
          </p>
          <p className="text-[10px] text-muted-foreground">
            Based on {artifact.meta.based_on_patches.length} patch
            {artifact.meta.based_on_patches.length !== 1 ? "es" : ""}
          </p>
        </div>
      </div>
    </button>
  );
}

interface ArtifactDetailProps {
  artifact: Artifact;
  preview: string | null;
}

function ArtifactDetail({ artifact, preview }: ArtifactDetailProps) {
  return (
    <ScrollArea className="h-full">
      <div className="p-3 space-y-3">
        {/* Header */}
        <div>
          <span className="text-[10px] px-1.5 py-0.5 rounded font-medium bg-[var(--ansi-cyan)]/20 text-[var(--ansi-cyan)]">
            PENDING
          </span>
          <h3 className="text-sm font-medium font-mono mt-1">{artifact.filename}</h3>
        </div>

        {/* Target */}
        <div>
          <p className="text-[10px] text-muted-foreground mb-1 font-medium">TARGET PATH</p>
          <code className="text-xs font-mono bg-muted px-2 py-1 rounded block">
            {artifact.meta.target}
          </code>
        </div>

        {/* Reason */}
        <div>
          <p className="text-[10px] text-muted-foreground mb-1 font-medium">REASON</p>
          <p className="text-xs">{artifact.meta.reason}</p>
        </div>

        {/* Based on patches */}
        <div>
          <p className="text-[10px] text-muted-foreground mb-1 font-medium">BASED ON PATCHES</p>
          <div className="flex flex-wrap gap-1">
            {artifact.meta.based_on_patches.map((id) => (
              <span key={id} className="text-[10px] font-mono bg-muted px-1.5 py-0.5 rounded">
                #{id}
              </span>
            ))}
          </div>
        </div>

        {/* Preview */}
        <div>
          <p className="text-[10px] text-muted-foreground mb-1 font-medium">CONTENT PREVIEW</p>
          {preview ? (
            <pre className="text-xs font-mono whitespace-pre-wrap bg-muted p-2 rounded overflow-x-auto">
              {preview}
            </pre>
          ) : (
            <div className="text-xs text-muted-foreground animate-pulse">Loading preview...</div>
          )}
        </div>
      </div>
    </ScrollArea>
  );
}
