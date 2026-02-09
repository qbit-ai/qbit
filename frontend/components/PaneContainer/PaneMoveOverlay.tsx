import { X } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { cn } from "@/lib/utils";
import type { PaneId } from "@/store";
import { useStore } from "@/store";

interface PaneMoveOverlayProps {
  paneId: PaneId;
}

type DropZone = "top" | "right" | "bottom" | "left";

export function PaneMoveOverlay({ paneId }: PaneMoveOverlayProps) {
  const paneMoveState = useStore((state) => state.paneMoveState);
  const completePaneMove = useStore((state) => state.completePaneMove);
  const cancelPaneMove = useStore((state) => state.cancelPaneMove);
  const [hoveredZone, setHoveredZone] = useState<DropZone | null>(null);

  const isSource = paneMoveState?.sourcePaneId === paneId;
  const isActive = paneMoveState !== null && !isSource;

  // ESC to cancel
  useEffect(() => {
    if (!paneMoveState) return;
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        cancelPaneMove();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [paneMoveState, cancelPaneMove]);

  const handleDrop = useCallback(
    (zone: DropZone) => {
      completePaneMove(paneId, zone);
    },
    [paneId, completePaneMove]
  );

  if (!paneMoveState) return null;

  // Source pane gets a dimmed overlay with cancel button
  if (isSource) {
    return (
      <div className="absolute inset-0 z-50 bg-background/60 flex items-center justify-center">
        <button
          type="button"
          onClick={cancelPaneMove}
          className="flex items-center gap-1.5 px-3 py-1.5 rounded-md bg-muted text-muted-foreground text-xs hover:bg-destructive/20 hover:text-destructive transition-colors"
        >
          <X className="w-3 h-3" />
          Cancel Move
        </button>
      </div>
    );
  }

  // Target panes get drop zones
  if (!isActive) return null;

  const zones: { zone: DropZone; clipPath: string; previewClass: string }[] = [
    {
      zone: "top",
      clipPath: "polygon(0 0, 100% 0, 70% 30%, 30% 30%)",
      previewClass: "inset-0 bottom-1/2",
    },
    {
      zone: "right",
      clipPath: "polygon(100% 0, 100% 100%, 70% 70%, 70% 30%)",
      previewClass: "inset-0 left-1/2",
    },
    {
      zone: "bottom",
      clipPath: "polygon(30% 70%, 70% 70%, 100% 100%, 0 100%)",
      previewClass: "inset-0 top-1/2",
    },
    {
      zone: "left",
      clipPath: "polygon(0 0, 30% 30%, 30% 70%, 0 100%)",
      previewClass: "inset-0 right-1/2",
    },
  ];

  return (
    <div className="absolute inset-0 z-50">
      {/* Drop zone hit areas */}
      {zones.map(({ zone, clipPath }) => (
        /* biome-ignore lint/a11y/useKeyWithClickEvents: drop zones use ESC to cancel and mouse interaction for spatial selection */
        /* biome-ignore lint/a11y/noStaticElementInteractions: spatial drop zones require mouse-based interaction */
        <div
          key={zone}
          className="absolute inset-0 cursor-pointer"
          style={{ clipPath }}
          onMouseEnter={() => setHoveredZone(zone)}
          onMouseLeave={() => setHoveredZone(null)}
          onClick={() => handleDrop(zone)}
        />
      ))}
      {/* Preview highlight */}
      {hoveredZone && (
        <div
          className={cn(
            "absolute pointer-events-none border-2 border-accent bg-accent/15 rounded-sm transition-all duration-100",
            zones.find((z) => z.zone === hoveredZone)?.previewClass
          )}
        />
      )}
      {/* Subtle overlay when no zone is hovered */}
      {!hoveredZone && (
        <div className="absolute inset-0 pointer-events-none border border-dashed border-muted-foreground/30 rounded-sm" />
      )}
    </div>
  );
}
