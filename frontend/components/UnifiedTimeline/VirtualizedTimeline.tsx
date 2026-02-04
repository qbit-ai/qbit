import { useVirtualizer } from "@tanstack/react-virtual";
import { memo, useEffect, useMemo } from "react";
import { TimelineBlockErrorBoundary } from "@/components/TimelineBlockErrorBoundary";
import { estimateBlockHeight } from "@/lib/timeline/blockHeightEstimation";
import type { UnifiedBlock as UnifiedBlockType } from "@/store";
import { UnifiedBlock } from "./UnifiedBlock";

// Static style constants for virtualized items
const virtualItemBaseStyle = {
  position: "absolute",
  top: 0,
  left: 0,
  width: "100%",
} as const;

interface VirtualizedTimelineProps {
  blocks: UnifiedBlockType[];
  sessionId: string;
  containerRef: React.RefObject<HTMLDivElement | null>;
  shouldScrollToBottom: boolean;
  workingDirectory: string;
}

// Minimum block count before enabling virtualization
// Below this threshold, direct rendering is more efficient
const VIRTUALIZATION_THRESHOLD = 50;

/**
 * Renders timeline blocks using virtualization for improved performance.
 * Only blocks visible in the viewport (plus overscan) are rendered to the DOM.
 */
export const VirtualizedTimeline = memo(function VirtualizedTimeline({
  blocks,
  sessionId,
  containerRef,
  shouldScrollToBottom,
  workingDirectory,
}: VirtualizedTimelineProps) {
  const virtualizer = useVirtualizer({
    count: blocks.length,
    getScrollElement: () => containerRef.current,
    estimateSize: (index) => estimateBlockHeight(blocks[index]),
    overscan: 5, // Render 5 extra items above/below viewport for smooth scrolling
  });

  // Scroll to bottom when new blocks are added and user is at bottom
  useEffect(() => {
    if (shouldScrollToBottom && blocks.length > 0) {
      virtualizer.scrollToIndex(blocks.length - 1, { align: "end" });
    }
  }, [blocks.length, shouldScrollToBottom, virtualizer]);

  // For small timelines, skip virtualization overhead
  if (blocks.length < VIRTUALIZATION_THRESHOLD) {
    return (
      <div className="space-y-2">
        {blocks.map((block) => (
          <TimelineBlockErrorBoundary key={block.id} blockId={block.id}>
            <UnifiedBlock block={block} sessionId={sessionId} workingDirectory={workingDirectory} />
          </TimelineBlockErrorBoundary>
        ))}
      </div>
    );
  }

  const virtualItems = virtualizer.getVirtualItems();

  // Memoize container style since height changes with content
  const containerStyle = useMemo(
    () => ({
      height: virtualizer.getTotalSize(),
      width: "100%",
      position: "relative" as const,
    }),
    [virtualizer.getTotalSize()]
  );

  return (
    <div style={containerStyle}>
      {virtualItems.map((virtualRow) => {
        const block = blocks[virtualRow.index];
        return (
          <div
            key={block.id}
            data-index={virtualRow.index}
            ref={virtualizer.measureElement}
            style={{
              ...virtualItemBaseStyle,
              transform: `translateY(${virtualRow.start}px)`,
            }}
          >
            <div className="pb-2">
              <TimelineBlockErrorBoundary blockId={block.id}>
                <UnifiedBlock
                  block={block}
                  sessionId={sessionId}
                  workingDirectory={workingDirectory}
                />
              </TimelineBlockErrorBoundary>
            </div>
          </div>
        );
      })}
    </div>
  );
});
