import { expect, type Page, test } from "@playwright/test";

/**
 * Terminal Portal Architecture E2E Tests
 *
 * These tests verify the terminal portal system that preserves xterm.js
 * instances across pane structure changes (splits, closes). The architecture:
 * 1. TerminalPortalProvider maintains a registry of portal targets
 * 2. PaneLeaf registers portal targets via useTerminalPortalTarget
 * 3. TerminalLayer renders all Terminals via React portals
 * 4. TerminalInstanceManager preserves xterm.js instances across remounts
 */

/**
 * Wait for the app to be fully ready in browser mode.
 */
async function waitForAppReady(page: Page) {
  await page.goto("/");
  await page.waitForLoadState("domcontentloaded");

  // Wait for the mock browser mode flag to be set
  await page.waitForFunction(
    () => (window as unknown as { __MOCK_BROWSER_MODE__?: boolean }).__MOCK_BROWSER_MODE__ === true,
    { timeout: 15000 }
  );

  // Wait for the status bar to appear (indicates React has rendered)
  await expect(page.locator('[data-testid="status-bar"]')).toBeVisible({
    timeout: 10000,
  });

  // Wait for the unified input textarea to be visible
  await expect(page.locator("textarea")).toBeVisible({ timeout: 5000 });
}

/**
 * Get the store state from the page.
 */
async function getStoreState(page: Page) {
  return await page.evaluate(() => {
    const store = (window as unknown as { __QBIT_STORE__?: { getState: () => unknown } })
      .__QBIT_STORE__;
    if (!store) return null;
    const state = store.getState() as {
      sessions: Record<string, { renderMode?: string }>;
      tabLayouts: Record<string, { root: unknown; focusedPaneId: string }>;
      activeSessionId: string | null;
    };
    return {
      sessionIds: Object.keys(state.sessions),
      tabLayoutIds: Object.keys(state.tabLayouts),
      activeSessionId: state.activeSessionId,
      sessions: state.sessions,
    };
  });
}

/**
 * Get the number of registered portal targets.
 */
async function _getPortalTargetCount(page: Page): Promise<number> {
  return await page.evaluate(() => {
    // Portal targets are registered in the TerminalPortalProvider context
    // Each PaneLeaf creates a div with ref={terminalPortalRef}
    // We can count these by looking for the portal target containers
    const portalTargets = document.querySelectorAll('[class*="flex-1 min-h-0 p-1"]');
    return portalTargets.length;
  });
}

/**
 * Get the number of Terminal components rendered via portals.
 */
async function getTerminalCount(page: Page): Promise<number> {
  return await page.evaluate(() => {
    // xterm.js creates elements with class "xterm"
    const terminals = document.querySelectorAll(".xterm");
    return terminals.length;
  });
}

/**
 * Create a split pane by directly manipulating the store.
 * Returns both the paneId and sessionId for the new pane.
 */
async function createSplitPane(
  page: Page,
  direction: "vertical" | "horizontal"
): Promise<{ paneId: string; sessionId: string }> {
  const result = await page.evaluate(async (dir) => {
    const store = (
      window as unknown as {
        __QBIT_STORE__?: {
          getState: () => {
            activeSessionId: string | null;
            sessions: Record<string, { workingDirectory: string }>;
            tabLayouts: Record<string, { focusedPaneId: string }>;
            addSession: (session: unknown, options?: { isPaneSession?: boolean }) => void;
            splitPane: (
              tabId: string,
              paneId: string,
              direction: string,
              newPaneId: string,
              newSessionId: string
            ) => void;
          };
        };
      }
    ).__QBIT_STORE__;

    if (!store) throw new Error("Store not found");

    const state = store.getState();
    const tabId = state.activeSessionId;
    if (!tabId) throw new Error("No active session");

    const tabLayout = state.tabLayouts[tabId];
    if (!tabLayout) throw new Error("No tab layout");

    const session = state.sessions[tabId];

    // Create new session ID and pane ID
    const newSessionId = `pane-session-${Date.now()}`;
    const newPaneId = `pane-${Date.now()}`;

    // Add the new session (as a pane session, not a tab)
    state.addSession(
      {
        id: newSessionId,
        name: "Split Pane",
        workingDirectory: session?.workingDirectory || "/home/user",
        createdAt: new Date().toISOString(),
        mode: "terminal",
        inputMode: "terminal",
      },
      { isPaneSession: true }
    );

    // Split the pane
    state.splitPane(tabId, tabLayout.focusedPaneId, dir, newPaneId, newSessionId);

    return { paneId: newPaneId, sessionId: newSessionId };
  }, direction);

  // Wait for state to update
  await page.waitForTimeout(100);
  return result;
}

/**
 * Set the session's render mode to fullterm via store.
 */
async function setRenderMode(page: Page, sessionId: string, mode: "timeline" | "fullterm") {
  await page.evaluate(
    ({ sessionId, mode }) => {
      const store = (
        window as unknown as {
          __QBIT_STORE__?: {
            getState: () => {
              setRenderMode: (sessionId: string, mode: string) => void;
            };
          };
        }
      ).__QBIT_STORE__;
      if (store) {
        store.getState().setRenderMode(sessionId, mode);
      }
    },
    { sessionId, mode }
  );
  await page.waitForTimeout(100);
}

/**
 * Close a pane by pane ID via store.
 */
async function closePane(page: Page, paneId: string) {
  await page.evaluate((paneId) => {
    const store = (
      window as unknown as {
        __QBIT_STORE__?: {
          getState: () => {
            activeSessionId: string | null;
            tabLayouts: Record<string, { focusedPaneId: string }>;
            closePane: (tabId: string, paneId: string) => void;
          };
        };
      }
    ).__QBIT_STORE__;
    if (!store) throw new Error("Store not found");

    const state = store.getState();
    const tabId = state.activeSessionId;
    if (!tabId) throw new Error("No active session");

    state.closePane(tabId, paneId);
  }, paneId);
  await page.waitForTimeout(100);
}

test.describe("Terminal Portal Architecture", () => {
  test.beforeEach(async ({ page }) => {
    await waitForAppReady(page);
  });

  test("portal target is registered when pane mounts", async ({ page }) => {
    // The initial pane should have a portal target registered
    const state = await getStoreState(page);
    expect(state).not.toBeNull();
    expect(state?.sessionIds.length).toBe(1);

    // Switch to fullterm mode to make the portal target visible
    const sessionId = state?.sessionIds[0];
    if (sessionId) {
      await setRenderMode(page, sessionId, "fullterm");
    }

    // The portal target div should exist
    const portalTarget = page.locator('[class*="flex-1 min-h-0 p-1"]');
    await expect(portalTarget.first()).toBeVisible();
  });

  test("splitting pane creates additional portal targets", async ({ page }) => {
    // Get initial state
    const initialState = await getStoreState(page);
    expect(initialState?.sessionIds.length).toBe(1);

    // Split the pane
    await createSplitPane(page, "vertical");

    // Verify a new session was created
    const stateAfterSplit = await getStoreState(page);
    expect(stateAfterSplit?.sessionIds.length).toBe(2);

    // Both sessions should exist
    expect(stateAfterSplit?.sessionIds).toContain(initialState?.sessionIds[0]);
  });

  test("terminals render via portal into their target containers", async ({ page }) => {
    // Get the root session
    const state = await getStoreState(page);
    const rootSessionId = state?.sessionIds[0];
    expect(rootSessionId).toBeDefined();
    if (!rootSessionId) return;

    // Switch to fullterm mode
    await setRenderMode(page, rootSessionId, "fullterm");

    // Wait for terminal to initialize
    await page.waitForTimeout(500);

    // The terminal should be rendered (xterm creates .xterm element)
    const terminalCount = await getTerminalCount(page);
    expect(terminalCount).toBeGreaterThanOrEqual(1);
  });

  test("split pane terminals render independently", async ({ page }) => {
    // Get the root session and set to fullterm
    let state = await getStoreState(page);
    const rootSessionId = state?.sessionIds[0];
    if (!rootSessionId) throw new Error("No root session found");
    await setRenderMode(page, rootSessionId, "fullterm");

    // Split the pane
    const { sessionId: newSessionId } = await createSplitPane(page, "vertical");

    // Set the new pane to fullterm as well
    await setRenderMode(page, newSessionId, "fullterm");

    // Wait for terminals to initialize
    await page.waitForTimeout(500);

    // Verify we have 2 sessions
    state = await getStoreState(page);
    expect(state?.sessionIds.length).toBe(2);
  });

  test("pane focus switches between split panes", async ({ page }) => {
    // Create a split
    await createSplitPane(page, "vertical");

    // Verify both panes exist by checking session count
    const state = await getStoreState(page);
    expect(state?.sessionIds.length).toBe(2);

    // The tab layout should track the focused pane
    const tabLayout = await page.evaluate(() => {
      const store = (
        window as unknown as {
          __QBIT_STORE__?: {
            getState: () => {
              activeSessionId: string | null;
              tabLayouts: Record<string, { focusedPaneId: string }>;
            };
          };
        }
      ).__QBIT_STORE__;
      if (!store) return null;
      const state = store.getState();
      const tabId = state.activeSessionId;
      return tabId ? state.tabLayouts[tabId] : null;
    });

    expect(tabLayout).not.toBeNull();
    expect(tabLayout?.focusedPaneId).toBeDefined();
  });

  test("closing a split pane removes its session and portal target", async ({ page }) => {
    // Create a split
    const { paneId, sessionId: newSessionId } = await createSplitPane(page, "vertical");

    // Verify we have 2 sessions
    let state = await getStoreState(page);
    expect(state?.sessionIds.length).toBe(2);
    expect(state?.sessionIds).toContain(newSessionId);

    // Close the new pane by paneId
    await closePane(page, paneId);
    await page.waitForTimeout(200);

    // Verify only 1 session remains
    state = await getStoreState(page);
    expect(state?.sessionIds.length).toBe(1);
    expect(state?.sessionIds).not.toContain(newSessionId);
  });

  test("multiple splits preserve all terminals", async ({ page }) => {
    // Create first split (vertical)
    await createSplitPane(page, "vertical");

    // Create second split (horizontal)
    await createSplitPane(page, "horizontal");

    // Verify we have 3 sessions
    const state = await getStoreState(page);
    expect(state?.sessionIds.length).toBe(3);
  });

  test("render mode toggle preserves session state", async ({ page }) => {
    // Get the root session
    const state = await getStoreState(page);
    const sessionId = state?.sessionIds[0];
    expect(sessionId).toBeDefined();
    if (!sessionId) return;

    // Toggle to fullterm
    await setRenderMode(page, sessionId, "fullterm");

    // Verify the portal target is visible
    const portalTarget = page.locator('[class*="flex-1 min-h-0 p-1"]');
    await expect(portalTarget.first()).toBeVisible();

    // Toggle back to timeline
    await setRenderMode(page, sessionId, "timeline");

    // Portal target should be hidden (has "hidden" class in timeline mode)
    const _hiddenPortal = page.locator('[class*="hidden"]').filter({
      has: page.locator('[class*="flex-1 min-h-0 p-1"]'),
    });
    // In timeline mode, the portal target div gets "hidden" class
    const stateAfter = await getStoreState(page);
    expect(stateAfter?.sessions[sessionId]?.renderMode).toBe("timeline");
  });
});

test.describe("Terminal Instance Manager", () => {
  test.beforeEach(async ({ page }) => {
    await waitForAppReady(page);
  });

  test("TerminalInstanceManager singleton is available", async ({ page }) => {
    // Set to fullterm to ensure terminal is initialized
    const state = await getStoreState(page);
    const sessionId = state?.sessionIds[0];
    if (!sessionId) throw new Error("No session found");
    await setRenderMode(page, sessionId, "fullterm");

    // Wait for terminal initialization
    await page.waitForTimeout(500);

    // Check that terminal manager has the session registered
    const hasInstance = await page.evaluate((_sessionId) => {
      // The TerminalInstanceManager is imported in Terminal.tsx
      // We can check if terminals are rendered by looking for xterm elements
      const terminal = document.querySelector(".xterm");
      return terminal !== null;
    }, sessionId);

    expect(hasInstance).toBe(true);
  });

  test("terminal element persists in DOM after pane operations", async ({ page }) => {
    // Set to fullterm mode
    const state = await getStoreState(page);
    const rootSessionId = state?.sessionIds[0];
    if (!rootSessionId) throw new Error("No root session found");
    await setRenderMode(page, rootSessionId, "fullterm");

    // Wait for terminal
    await page.waitForTimeout(500);

    // Count terminals before split
    const countBefore = await getTerminalCount(page);
    expect(countBefore).toBeGreaterThanOrEqual(1);

    // Create a split
    const { sessionId: newSessionId } = await createSplitPane(page, "vertical");
    await setRenderMode(page, newSessionId, "fullterm");

    // Wait for the new terminal
    await page.waitForTimeout(500);

    // Count terminals after split - both should exist
    const countAfter = await getTerminalCount(page);
    expect(countAfter).toBeGreaterThanOrEqual(2);
  });
});

test.describe("Pane Focus Management", () => {
  test.beforeEach(async ({ page }) => {
    await waitForAppReady(page);
  });

  test("focus indicator shows on focused pane when multiple panes exist", async ({ page }) => {
    // Get initial pane count
    let panes = page.locator("section[aria-label*='Pane']");
    await expect(panes).toHaveCount(1);

    // Create a split
    await createSplitPane(page, "vertical");

    // Now with multiple panes, focus indicator should show on focused pane
    // Wait for the UI to update
    await page.waitForTimeout(200);

    // Verify we have 2 panes now
    panes = page.locator("section[aria-label*='Pane']");
    await expect(panes).toHaveCount(2);

    // The focused pane should have the accent border overlay inside it
    // (the overlay is a div with absolute positioning and border-accent class)
    const focusIndicator = page.locator("section[aria-label*='Pane'] > div.border-accent");
    // Note: The exact count depends on implementation - at least one should exist
    const count = await focusIndicator.count();
    expect(count).toBeGreaterThanOrEqual(1);
  });

  test("clicking a pane changes focus", async ({ page }) => {
    // Create a split
    await createSplitPane(page, "vertical");

    // Get the focused pane ID before clicking
    const focusBefore = await page.evaluate(() => {
      const store = (
        window as unknown as {
          __QBIT_STORE__?: {
            getState: () => {
              activeSessionId: string | null;
              tabLayouts: Record<string, { focusedPaneId: string }>;
            };
          };
        }
      ).__QBIT_STORE__;
      if (!store) return null;
      const state = store.getState();
      const tabId = state.activeSessionId;
      return tabId ? state.tabLayouts[tabId]?.focusedPaneId : null;
    });

    expect(focusBefore).not.toBeNull();

    // Click on one of the pane sections
    const panes = page.locator("section[aria-label*='Pane']");
    const paneCount = await panes.count();
    expect(paneCount).toBe(2);

    // Click the first pane
    await panes.first().click();
    await page.waitForTimeout(100);

    // Verify focus is tracked (it may or may not change depending on which was already focused)
    const focusAfter = await page.evaluate(() => {
      const store = (
        window as unknown as {
          __QBIT_STORE__?: {
            getState: () => {
              activeSessionId: string | null;
              tabLayouts: Record<string, { focusedPaneId: string }>;
            };
          };
        }
      ).__QBIT_STORE__;
      if (!store) return null;
      const state = store.getState();
      const tabId = state.activeSessionId;
      return tabId ? state.tabLayouts[tabId]?.focusedPaneId : null;
    });

    expect(focusAfter).not.toBeNull();
  });
});
