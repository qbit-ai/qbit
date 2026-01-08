/**
 * FilePathPopup - Popup for file path actions
 * Shows "Open in Editor" and "Copy Path" actions for detected file paths
 */

import { Copy, ExternalLink, File } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Popover, PopoverContent } from "@/components/ui/popover";
import { useCopyToClipboard } from "@/hooks/useCopyToClipboard";
import type { ResolvedPath } from "@/lib/pathResolution";

interface FilePathPopupProps {
  /** Whether the popup is open */
  open: boolean;
  /** Callback when open state changes */
  onOpenChange: (open: boolean) => void;
  /** Resolved path(s) - may be multiple for ambiguous filename */
  paths: ResolvedPath[];
  /** Callback to open file in editor */
  onOpenFile: (absolutePath: string, line?: number, column?: number) => void;
  /** Position for fixed positioning (for terminal links) */
  position?: { x: number; y: number };
  /** Whether we're still loading/resolving paths */
  loading?: boolean;
  children?: React.ReactNode;
}

export function FilePathPopup({
  open,
  onOpenChange,
  paths,
  onOpenFile,
  position,
  loading,
  children,
}: FilePathPopupProps) {
  const { copied, copy } = useCopyToClipboard();

  const handleOpenFile = (path: ResolvedPath) => {
    onOpenFile(path.absolutePath, path.line, path.column);
    onOpenChange(false);
  };

  const handleCopyPath = async (path: ResolvedPath) => {
    await copy(path.absolutePath);
  };

  // For fixed positioning (terminal links), we don't use children/anchor
  const isFixedPosition = !!position;

  const content = (
    <div className="bg-popover border border-border rounded-md overflow-hidden min-w-[280px]">
      {loading ? (
        <div className="py-3 px-4 text-sm text-muted-foreground">Resolving path...</div>
      ) : paths.length === 0 ? (
        <div className="py-3 px-4 text-sm text-muted-foreground">File not found</div>
      ) : paths.length === 1 ? (
        // Single path - show actions directly
        <SinglePathView
          path={paths[0]}
          copied={copied}
          onOpen={() => handleOpenFile(paths[0])}
          onCopy={() => handleCopyPath(paths[0])}
        />
      ) : (
        // Multiple paths - show list
        <MultiplePathsView paths={paths} onOpen={handleOpenFile} onCopy={handleCopyPath} />
      )}
    </div>
  );

  if (isFixedPosition) {
    // Fixed positioning for terminal links
    if (!open) return null;
    return (
      <>
        {/* Backdrop to close on click outside */}
        <button
          type="button"
          aria-label="Close file path popup"
          className="fixed inset-0 z-40 cursor-default bg-transparent p-0 m-0 border-0 focus:outline-none"
          onClick={() => onOpenChange(false)}
          onKeyDown={(e) => {
            if (e.key === "Escape") {
              e.preventDefault();
              onOpenChange(false);
            }
          }}
        />
        <div
          className="fixed z-50"
          style={{
            left: position.x,
            top: position.y,
            transform: "translateY(-100%)", // Position above click
          }}
        >
          {content}
        </div>
      </>
    );
  }

  // Popover mode for Markdown links
  return (
    <Popover open={open} onOpenChange={onOpenChange}>
      {children}
      <PopoverContent
        className="w-auto p-0"
        side="top"
        align="start"
        sideOffset={8}
        onOpenAutoFocus={(e) => e.preventDefault()}
      >
        {content}
      </PopoverContent>
    </Popover>
  );
}

// View for single resolved path
function SinglePathView({
  path,
  copied,
  onOpen,
  onCopy,
}: {
  path: ResolvedPath;
  copied: boolean;
  onOpen: () => void;
  onCopy: () => void;
}) {
  return (
    <div className="p-2">
      {/* Path display */}
      <div className="px-2 py-1.5 mb-2">
        <div className="flex items-center gap-2 text-sm text-foreground font-mono truncate">
          <File className="w-4 h-4 flex-shrink-0 text-muted-foreground" />
          <span className="truncate">{path.relativePath}</span>
          {path.line && (
            <span className="text-muted-foreground">
              :{path.line}
              {path.column ? `:${path.column}` : ""}
            </span>
          )}
        </div>
      </div>
      {/* Actions */}
      <div className="flex gap-1">
        <Button variant="ghost" size="sm" className="flex-1 justify-start gap-2" onClick={onOpen}>
          <ExternalLink className="w-4 h-4" />
          Open in Editor
        </Button>
        <Button variant="ghost" size="sm" className="justify-start gap-2" onClick={onCopy}>
          <Copy className="w-4 h-4" />
          {copied ? "Copied!" : "Copy"}
        </Button>
      </div>
    </div>
  );
}

// View for multiple resolved paths (ambiguous filename)
function MultiplePathsView({
  paths,
  onOpen,
  onCopy,
}: {
  paths: ResolvedPath[];
  onOpen: (path: ResolvedPath) => void;
  onCopy: (path: ResolvedPath) => void;
}) {
  return (
    <div className="py-1">
      <div className="px-3 py-1.5 text-xs text-muted-foreground font-medium uppercase tracking-wide border-b border-border">
        Multiple matches found
      </div>
      <div className="max-h-[200px] overflow-y-auto">
        {paths.map((path) => (
          <div key={path.absolutePath} className="px-2 py-1 hover:bg-card transition-colors">
            <div className="flex items-center gap-2 text-sm text-foreground font-mono">
              <File className="w-4 h-4 flex-shrink-0 text-muted-foreground" />
              <span className="truncate flex-1">{path.relativePath}</span>
              <div className="flex gap-1 flex-shrink-0">
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-7 w-7"
                  onClick={() => onOpen(path)}
                  title="Open in Editor"
                >
                  <ExternalLink className="w-3.5 h-3.5" />
                </Button>
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-7 w-7"
                  onClick={() => onCopy(path)}
                  title="Copy Path"
                >
                  <Copy className="w-3.5 h-3.5" />
                </Button>
              </div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
