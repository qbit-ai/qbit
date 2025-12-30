/**
 * Pane layout utilities for multi-pane support.
 * Pure functions for manipulating the pane tree structure.
 *
 * NOTE: Types are defined here as the single source of truth.
 * The store re-exports these types for consumer convenience.
 */

export type PaneId = string;
export type SplitDirection = "horizontal" | "vertical";

export type PaneNode =
  | { type: "leaf"; id: PaneId; sessionId: string }
  | {
      type: "split";
      id: PaneId;
      direction: SplitDirection;
      children: [PaneNode, PaneNode];
      ratio: number;
    };

export interface TabLayout {
  root: PaneNode;
  focusedPaneId: PaneId;
}

/**
 * Create a new leaf pane node.
 */
export function createLeafPane(paneId: PaneId, sessionId: string): PaneNode {
  return { type: "leaf", id: paneId, sessionId };
}

/**
 * Split an existing pane into two.
 * The original pane becomes the first child, new pane becomes second.
 *
 * @param root - Current tree root
 * @param targetPaneId - ID of the pane to split
 * @param direction - "vertical" (side by side) or "horizontal" (stacked)
 * @param newPaneId - ID for the new pane
 * @param newSessionId - Session ID for the new pane
 * @returns New tree root with the split applied
 */
export function splitPaneNode(
  root: PaneNode,
  targetPaneId: PaneId,
  direction: SplitDirection,
  newPaneId: PaneId,
  newSessionId: string
): PaneNode {
  if (root.type === "leaf") {
    if (root.id === targetPaneId) {
      // Found the target - split it
      return {
        type: "split",
        id: crypto.randomUUID(),
        direction,
        children: [root, createLeafPane(newPaneId, newSessionId)],
        ratio: 0.5,
      };
    }
    // Not the target, return unchanged
    return root;
  }

  // Split node - recurse into children
  const [first, second] = root.children;
  const newFirst = splitPaneNode(first, targetPaneId, direction, newPaneId, newSessionId);
  const newSecond = splitPaneNode(second, targetPaneId, direction, newPaneId, newSessionId);

  // Only create new object if children changed
  if (newFirst === first && newSecond === second) {
    return root;
  }

  return {
    ...root,
    children: [newFirst, newSecond],
  };
}

/**
 * Remove a pane from the tree.
 * When a pane is removed, its sibling replaces the parent split.
 *
 * @param root - Current tree root
 * @param targetPaneId - ID of the pane to remove
 * @returns New tree root, or null if the tree becomes empty
 */
export function removePaneNode(root: PaneNode, targetPaneId: PaneId): PaneNode | null {
  if (root.type === "leaf") {
    // If this is the target and it's the root, tree becomes empty
    if (root.id === targetPaneId) {
      return null;
    }
    return root;
  }

  const [first, second] = root.children;

  // Check if either child is the target leaf
  if (first.type === "leaf" && first.id === targetPaneId) {
    // Remove first child, return second (sibling becomes new subtree)
    return second;
  }
  if (second.type === "leaf" && second.id === targetPaneId) {
    // Remove second child, return first
    return first;
  }

  // Recurse into children
  const newFirst = removePaneNode(first, targetPaneId);
  const newSecond = removePaneNode(second, targetPaneId);

  // Handle cases where a child was removed
  if (newFirst === null) {
    return newSecond;
  }
  if (newSecond === null) {
    return newFirst;
  }

  // Neither child was fully removed, but they may have changed
  if (newFirst === first && newSecond === second) {
    return root;
  }

  return {
    ...root,
    children: [newFirst, newSecond],
  };
}

/**
 * Find a pane by ID.
 */
export function findPaneById(root: PaneNode, paneId: PaneId): PaneNode | null {
  if (root.id === paneId) {
    return root;
  }
  if (root.type === "split") {
    const inFirst = findPaneById(root.children[0], paneId);
    if (inFirst) return inFirst;
    return findPaneById(root.children[1], paneId);
  }
  return null;
}

/**
 * Find a pane's parent split node.
 */
export function findPaneParent(
  root: PaneNode,
  paneId: PaneId
): { parent: PaneNode & { type: "split" }; childIndex: 0 | 1 } | null {
  if (root.type === "leaf") {
    return null;
  }

  const [first, second] = root.children;

  if (first.id === paneId) {
    return { parent: root, childIndex: 0 };
  }
  if (second.id === paneId) {
    return { parent: root, childIndex: 1 };
  }

  // Recurse
  const inFirst = findPaneParent(first, paneId);
  if (inFirst) return inFirst;
  return findPaneParent(second, paneId);
}

/**
 * Get adjacent pane in a direction for keyboard navigation.
 *
 * Algorithm:
 * 1. Walk up the tree to find a split with matching axis
 * 2. If we came from the "toward" side, continue up
 * 3. If we came from the "away" side, go into the other child
 * 4. Walk down to find the closest leaf in that direction
 */
export function getPaneNeighbor(
  root: PaneNode,
  currentPaneId: PaneId,
  direction: "up" | "down" | "left" | "right"
): PaneId | null {
  // Determine which axis we're navigating on
  const isVerticalNav = direction === "up" || direction === "down";
  // For vertical nav, we need horizontal splits (stacked panes)
  // For horizontal nav, we need vertical splits (side-by-side panes)
  const targetSplitDirection: SplitDirection = isVerticalNav ? "horizontal" : "vertical";
  // Are we moving toward child[0] or child[1]?
  const movingToFirst = direction === "up" || direction === "left";

  // Build path from root to current pane
  const path = buildPathToPane(root, currentPaneId);
  if (!path || path.length === 0) return null;

  // Walk up the path looking for a split we can navigate through
  for (let i = path.length - 1; i >= 0; i--) {
    const step = path[i];
    if (step.node.type !== "split") continue;

    // Check if this split is on the right axis
    if (step.node.direction !== targetSplitDirection) continue;

    // Check if we can navigate through this split
    // If we're moving to first (up/left) and we came from second child, we can navigate
    // If we're moving to second (down/right) and we came from first child, we can navigate
    const cameFromIndex = step.childIndex;
    const canNavigate = movingToFirst ? cameFromIndex === 1 : cameFromIndex === 0;

    if (canNavigate) {
      // Navigate into the other child
      const targetChild = step.node.children[movingToFirst ? 0 : 1];
      // Find the closest leaf in that subtree
      return movingToFirst ? getLastLeafPane(targetChild) : getFirstLeafPane(targetChild);
    }
  }

  // No valid neighbor found
  return null;
}

interface PathStep {
  node: PaneNode;
  childIndex: 0 | 1 | null; // null for root or leaf
}

function buildPathToPane(root: PaneNode, targetId: PaneId): PathStep[] | null {
  if (root.id === targetId) {
    return [{ node: root, childIndex: null }];
  }

  if (root.type === "leaf") {
    return null;
  }

  // Check first child
  const firstPath = buildPathToPane(root.children[0], targetId);
  if (firstPath) {
    return [{ node: root, childIndex: 0 }, ...firstPath];
  }

  // Check second child
  const secondPath = buildPathToPane(root.children[1], targetId);
  if (secondPath) {
    return [{ node: root, childIndex: 1 }, ...secondPath];
  }

  return null;
}

/**
 * Update the split ratio for a split node.
 */
export function updatePaneRatio(root: PaneNode, splitPaneId: PaneId, ratio: number): PaneNode {
  if (root.type === "leaf") {
    return root;
  }

  if (root.id === splitPaneId) {
    return { ...root, ratio: Math.max(0.1, Math.min(0.9, ratio)) };
  }

  const [first, second] = root.children;
  const newFirst = updatePaneRatio(first, splitPaneId, ratio);
  const newSecond = updatePaneRatio(second, splitPaneId, ratio);

  if (newFirst === first && newSecond === second) {
    return root;
  }

  return { ...root, children: [newFirst, newSecond] };
}

/**
 * Get all leaf panes in the tree.
 */
export function getAllLeafPanes(root: PaneNode): Array<{ id: PaneId; sessionId: string }> {
  if (root.type === "leaf") {
    return [{ id: root.id, sessionId: root.sessionId }];
  }

  return [...getAllLeafPanes(root.children[0]), ...getAllLeafPanes(root.children[1])];
}

/**
 * Get the first (leftmost/topmost) leaf pane.
 */
export function getFirstLeafPane(root: PaneNode): PaneId {
  if (root.type === "leaf") {
    return root.id;
  }
  return getFirstLeafPane(root.children[0]);
}

/**
 * Get the last (rightmost/bottommost) leaf pane.
 */
export function getLastLeafPane(root: PaneNode): PaneId {
  if (root.type === "leaf") {
    return root.id;
  }
  return getLastLeafPane(root.children[1]);
}

/**
 * Count the number of leaf panes in the tree.
 */
export function countLeafPanes(root: PaneNode): number {
  if (root.type === "leaf") {
    return 1;
  }
  return countLeafPanes(root.children[0]) + countLeafPanes(root.children[1]);
}

/**
 * Find the session ID for a given pane ID.
 */
export function getSessionIdForPane(root: PaneNode, paneId: PaneId): string | null {
  const pane = findPaneById(root, paneId);
  if (pane?.type === "leaf") {
    return pane.sessionId;
  }
  return null;
}
