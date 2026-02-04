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
import { logger } from "@/lib/logger";

interface FilePathLinkProps {
  /** The detected path info */
  detected: DetectedPath;
  /** Working directory for path resolution */
  workingDirectory: string;
  /** The text to display (may differ from detected.raw if we only wrap part of it) */
  children: ReactNode;
  /** Pre-resolved absolute path (if known from index) */
  absolutePath?: string;
}

export function FilePathLink({
  detected,
  workingDirectory,
  children,
  absolutePath,
}: FilePathLinkProps) {
  const [open, setOpen] = useState(false);
  const [loading, setLoading] = useState(false);
  const [resolvedPaths, setResolvedPaths] = useState<ResolvedPath[]>([]);

  const { openFile } = useFileEditorSidebar(workingDirectory);

  const handleClick = useCallback(async () => {
    if (open) {
      setOpen(false);
      return;
    }

    setOpen(true);

    // If absolutePath is provided, use it directly
    if (absolutePath) {
      setResolvedPaths([
        {
          absolutePath,
          relativePath: detected.path,
          line: detected.line,
          column: detected.column,
        },
      ]);
      setLoading(false);
    } else {
      // Otherwise, resolve the path
      setLoading(true);

      try {
        const paths = await resolvePath(detected, workingDirectory);
        setResolvedPaths(paths);
      } catch (error) {
        logger.error("Failed to resolve path:", error);
        setResolvedPaths([]);
      } finally {
        setLoading(false);
      }
    }
  }, [detected, workingDirectory, open, absolutePath]);

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
