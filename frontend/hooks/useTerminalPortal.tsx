/**
 * Terminal Portal Context
 *
 * Manages portal targets for Terminal components. This allows Terminals to be
 * rendered at a stable position in the React tree (preventing unmount/remount
 * when pane structure changes) while displaying inside their respective panes
 * via React portals.
 *
 * Architecture:
 * 1. TerminalPortalProvider wraps the app and maintains a registry of portal targets
 * 2. PaneLeaf registers a portal target element when it mounts
 * 3. TerminalLayer renders all Terminals, each using createPortal to its target
 * 4. When pane structure changes, Terminals stay mounted because they're rendered
 *    at the provider level, not inside the recursive pane tree
 */

import {
  createContext,
  type ReactNode,
  useCallback,
  useContext,
  useMemo,
  useRef,
  useSyncExternalStore,
} from "react";

interface PortalTarget {
  sessionId: string;
  element: HTMLElement;
}

interface TerminalPortalContextValue {
  /** Register a portal target for a session */
  registerTarget: (sessionId: string, element: HTMLElement) => void;
  /** Unregister a portal target */
  unregisterTarget: (sessionId: string) => void;
  /** Get the current portal target for a session */
  getTarget: (sessionId: string) => HTMLElement | null;
  /** Subscribe to target changes (for useSyncExternalStore) */
  subscribe: (callback: () => void) => () => void;
  /** Get snapshot of all targets */
  getSnapshot: () => Map<string, PortalTarget>;
}

const TerminalPortalContext = createContext<TerminalPortalContextValue | null>(null);

export function TerminalPortalProvider({ children }: { children: ReactNode }) {
  const targetsRef = useRef(new Map<string, PortalTarget>());
  const listenersRef = useRef(new Set<() => void>());
  // Immutable snapshot for useSyncExternalStore - must return new reference on changes
  const snapshotRef = useRef(new Map<string, PortalTarget>());

  const notifyListeners = useCallback(() => {
    // Create a new immutable snapshot when targets change
    // This ensures useSyncExternalStore detects the change via reference comparison
    snapshotRef.current = new Map(targetsRef.current);
    for (const listener of listenersRef.current) {
      listener();
    }
  }, []);

  const registerTarget = useCallback(
    (sessionId: string, element: HTMLElement) => {
      targetsRef.current.set(sessionId, { sessionId, element });
      notifyListeners();
    },
    [notifyListeners]
  );

  const unregisterTarget = useCallback(
    (sessionId: string) => {
      targetsRef.current.delete(sessionId);
      notifyListeners();
    },
    [notifyListeners]
  );

  const getTarget = useCallback((sessionId: string) => {
    return targetsRef.current.get(sessionId)?.element ?? null;
  }, []);

  const subscribe = useCallback((callback: () => void) => {
    listenersRef.current.add(callback);
    return () => {
      listenersRef.current.delete(callback);
    };
  }, []);

  // Return the immutable snapshot - stable reference until notifyListeners is called
  const getSnapshot = useCallback(() => {
    return snapshotRef.current;
  }, []);

  const value = useMemo(
    () => ({
      registerTarget,
      unregisterTarget,
      getTarget,
      subscribe,
      getSnapshot,
    }),
    [registerTarget, unregisterTarget, getTarget, subscribe, getSnapshot]
  );

  return <TerminalPortalContext.Provider value={value}>{children}</TerminalPortalContext.Provider>;
}

/**
 * Hook to register a portal target element for a session.
 * Used by PaneLeaf to create the target where its Terminal will be rendered.
 */
export function useTerminalPortalTarget(sessionId: string) {
  const context = useContext(TerminalPortalContext);
  if (!context) {
    throw new Error("useTerminalPortalTarget must be used within TerminalPortalProvider");
  }

  const { registerTarget, unregisterTarget } = context;

  const setTargetRef = useCallback(
    (element: HTMLElement | null) => {
      if (element) {
        registerTarget(sessionId, element);
      } else {
        unregisterTarget(sessionId);
      }
    },
    [sessionId, registerTarget, unregisterTarget]
  );

  return setTargetRef;
}

/**
 * Hook to get the portal target for a session.
 * Used by TerminalLayer to render Terminals into their targets.
 */
export function useTerminalPortalTargets() {
  const context = useContext(TerminalPortalContext);
  if (!context) {
    throw new Error("useTerminalPortalTargets must be used within TerminalPortalProvider");
  }

  const { subscribe, getSnapshot } = context;

  // Use useSyncExternalStore to subscribe to target changes
  const targets = useSyncExternalStore(subscribe, getSnapshot, getSnapshot);

  return targets;
}
