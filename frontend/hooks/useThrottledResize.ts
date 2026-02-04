import { useCallback, useEffect, useRef, useState } from "react";

interface UseThrottledResizeOptions {
  /** Minimum allowed width */
  minWidth: number;
  /** Maximum allowed width */
  maxWidth: number;
  /** Callback when width changes (throttled by RAF) */
  onWidthChange: (width: number) => void;
  /**
   * Custom function to calculate width from mouse event.
   * Default: uses clientX directly (for left-side panels).
   * For right-side panels, use: (e) => window.innerWidth - e.clientX
   */
  calculateWidth?: (e: MouseEvent) => number;
}

interface UseThrottledResizeReturn {
  /** Call this from onMouseDown on the resize handle */
  startResizing: (e: React.MouseEvent) => void;
  /** Whether resize is currently in progress */
  isResizing: boolean;
}

/**
 * Hook for handling panel resize with RAF-based throttling.
 *
 * This hook provides smooth, performant resizing by:
 * 1. Only processing one mousemove per animation frame (RAF throttling)
 * 2. Properly cleaning up event listeners
 * 3. Cancelling pending RAF on mouseup or unmount
 *
 * @example
 * ```tsx
 * const { startResizing, isResizing } = useThrottledResize({
 *   minWidth: 200,
 *   maxWidth: 600,
 *   onWidthChange: setWidth,
 *   calculateWidth: (e) => e.clientX, // for left panel
 * });
 *
 * return (
 *   <div onMouseDown={startResizing} className="resize-handle" />
 * );
 * ```
 */
export function useThrottledResize({
  minWidth,
  maxWidth,
  onWidthChange,
  calculateWidth = (e) => e.clientX,
}: UseThrottledResizeOptions): UseThrottledResizeReturn {
  const [isResizing, setIsResizing] = useState(false);
  const isResizingRef = useRef(false);
  const rafRef = useRef<number | null>(null);
  const latestWidthRef = useRef<number | null>(null);

  const startResizing = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    isResizingRef.current = true;
    setIsResizing(true);
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";
  }, []);

  useEffect(() => {
    const handleMouseMove = (e: MouseEvent) => {
      if (!isResizingRef.current) return;

      // Store the latest calculated width
      latestWidthRef.current = calculateWidth(e);

      // Skip if we already have a RAF pending
      if (rafRef.current !== null) return;

      rafRef.current = requestAnimationFrame(() => {
        rafRef.current = null;
        const width = latestWidthRef.current;
        if (width !== null && width >= minWidth && width <= maxWidth) {
          onWidthChange(width);
        }
      });
    };

    const handleMouseUp = () => {
      if (isResizingRef.current) {
        isResizingRef.current = false;
        setIsResizing(false);
        document.body.style.cursor = "";
        document.body.style.userSelect = "";

        // Cancel any pending RAF
        if (rafRef.current !== null) {
          cancelAnimationFrame(rafRef.current);
          rafRef.current = null;
        }
      }
    };

    document.addEventListener("mousemove", handleMouseMove);
    document.addEventListener("mouseup", handleMouseUp);

    return () => {
      document.removeEventListener("mousemove", handleMouseMove);
      document.removeEventListener("mouseup", handleMouseUp);

      // Cancel any pending RAF on unmount
      if (rafRef.current !== null) {
        cancelAnimationFrame(rafRef.current);
        rafRef.current = null;
      }
    };
  }, [minWidth, maxWidth, onWidthChange, calculateWidth]);

  return {
    startResizing,
    isResizing,
  };
}
