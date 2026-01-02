/**
 * TerminalLayer - Renders all Terminal instances using React portals.
 *
 * This component solves the problem of Terminal unmount/remount when pane structure
 * changes (e.g., during splits). By rendering Terminals at a stable position in the
 * React tree and using portals to display them in their respective panes, the
 * Terminal instances stay mounted even when the pane tree is restructured.
 *
 * Flow:
 * 1. PaneLeaf registers a portal target element via useTerminalPortalTarget
 * 2. This component gets all registered targets via useTerminalPortalTargets
 * 3. For each session with a registered target, render Terminal via createPortal
 * 4. When pane structure changes, targets may move but Terminals stay mounted
 */

import { createPortal } from "react-dom";
import { useTerminalPortalTargets } from "@/hooks/useTerminalPortal";
import { Terminal } from "./Terminal";

export function TerminalLayer() {
  const targets = useTerminalPortalTargets();

  // Render a Terminal for each registered portal target
  // The Terminal is portaled into its target element (inside PaneLeaf)
  return (
    <>
      {Array.from(targets.entries()).map(([sessionId, { element }]) =>
        createPortal(<Terminal sessionId={sessionId} />, element, sessionId)
      )}
    </>
  );
}
