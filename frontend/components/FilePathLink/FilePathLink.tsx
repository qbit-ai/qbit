/**
 * FilePathLink - Clickable file path link for Markdown content
 * Renders detected file paths as styled, clickable text that opens a popup with actions
 */

import { type ReactNode, useCallback, useState } from "react";
import { FilePathPopup } from "@/components/FilePathPopup";
import { PopoverAnchor } from "@/components/ui/popover";
import { useFileEditorSidebar } from "@/hooks/useFileEditorSidebar";
import type { DetectedPath } from "@/lib/pathDetection";
import type { ResolvedPath } from "@/lib/pathResolution";
import { resolvePath } from "@/lib/pathResolution";

interface FilePathLinkProps {
  /** The detected path info */
  detected: DetectedPath;
  /** Working directory for path resolution */
  workingDirectory: string;
  /** Session ID for file editor */
  sessionId: string;
  /** The text to display (may differ from detected.raw if we only wrap part of it) */
  children: ReactNode;
}

export function FilePathLink({
  detected,
  workingDirectory,
  sessionId,
  children,
}: FilePathLinkProps) {
  const [open, setOpen] = useState(false);
  const [loading, setLoading] = useState(false);
  const [resolvedPaths, setResolvedPaths] = useState<ResolvedPath[]>([]);

  const { openFile } = useFileEditorSidebar(sessionId, workingDirectory);

  const handleClick = useCallback(async () => {
    if (open) {
      setOpen(false);
      return;
    }

    setOpen(true);
    setLoading(true);

    try {
      const paths = await resolvePath(detected, workingDirectory);
      setResolvedPaths(paths);
    } catch (error) {
      console.error("Failed to resolve path:", error);
      setResolvedPaths([]);
    } finally {
      setLoading(false);
    }
  }, [detected, workingDirectory, open]);

  const handleOpenFile = useCallback(
    (absolutePath: string, _line?: number, _column?: number) => {
      // TODO: Support line navigation when CodeMirror supports it
      openFile(absolutePath);
    },
    [openFile]
  );

  return (
    <FilePathPopup
      open={open}
      onOpenChange={setOpen}
      paths={resolvedPaths}
      loading={loading}
      onOpenFile={handleOpenFile}
    >
      <PopoverAnchor asChild>
        <button
          type="button"
          onClick={handleClick}
          className="file-path-link inline bg-transparent p-0 text-left text-[var(--ansi-cyan)] underline underline-offset-2 hover:text-accent transition-colors cursor-pointer"
        >
          {children}
        </button>
      </PopoverAnchor>
    </FilePathPopup>
  );
}
