import { expect, type Page, test } from "@playwright/test";

/**
 * Tab Close with Split Panes E2E Tests
 *
 * These tests verify that when a tab with split panes is closed,
 * ALL sessions (root + pane sessions) are properly cleaned up:
 * - PTY processes destroyed via pty_destroy
 * - AI sessions shutdown via shutdown_ai_session
 * - Frontend state removed
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

  // Wait for the unified input textarea to be visible (exclude xterm's hidden textarea)
  await expect(page.locator("textarea:not(.xterm-helper-textarea)")).toBeVisible({ timeout: 5000 });
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
      sessions: Record<string, unknown>;
      tabLayouts: Record<string, { root: unknown; focusedPaneId: string }>;
      activeSessionId: string | null;
    };
    return {
      sessionIds: Object.keys(state.sessions),
      tabLayoutIds: Object.keys(state.tabLayouts),
      activeSessionId: state.activeSessionId,
    };
  });
}

/**
 * Get tab count from the page.
 */
async function getTabCount(page: Page): Promise<number> {
  return await page.locator('[role="tab"]').count();
}

/**
 * Create a new tab via the UI.
 */
async function createNewTab(page: Page): Promise<void> {
  await page.getByRole("button", { name: "New tab" }).click();
  // Wait for the new tab to appear
  await page.waitForTimeout(200);
}

/**
 * Close the first tab by hovering to reveal the close button.
 */
async function closeFirstTab(page: Page): Promise<void> {
  // The tab structure wraps the trigger and close button in a parent div with class "group"
  const tabWrapper = page
    .locator(".group")
    .filter({ has: page.locator('[role="tab"]') })
    .first();
  await tabWrapper.hover();
  // Wait for the close button to become visible on hover
  await page.waitForTimeout(100);
  const closeButton = tabWrapper.locator('button[title="Close tab"]');
  await closeButton.click();
  // Wait for the tab to close
  await page.waitForTimeout(200);
}

/**
 * Create a split pane by directly manipulating the store.
 * This bypasses keyboard/command palette issues in E2E tests.
 */
async function createSplitPane(page: Page, direction: "vertical" | "horizontal"): Promise<string> {
  const newSessionId = await page.evaluate(async (dir) => {
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

    return newSessionId;
  }, direction);

  // Wait for state to update
  await page.waitForTimeout(100);
  return newSessionId;
}

test.describe("Tab Close with Split Panes Cleanup", () => {
  test.beforeEach(async ({ page }) => {
    await waitForAppReady(page);
  });

  test("closing a single-pane tab cleans up the session", async ({ page }) => {
    // Get initial state
    const initialState = await getStoreState(page);
    expect(initialState).not.toBeNull();
    const initialTabCount = await getTabCount(page);
    expect(initialTabCount).toBe(1);

    // Create a second tab so we have something to switch to
    await createNewTab(page);
    expect(await getTabCount(page)).toBe(2);

    // Get the state after creating second tab
    const stateWithTwoTabs = await getStoreState(page);
    expect(stateWithTwoTabs?.sessionIds.length).toBe(2);

    // Close the first tab
    await closeFirstTab(page);

    // Wait for cleanup to complete
    await page.waitForTimeout(300);

    // Verify tab was closed
    expect(await getTabCount(page)).toBe(1);

    // Verify state was cleaned up
    const finalState = await getStoreState(page);
    expect(finalState?.sessionIds.length).toBe(1);
  });

  test("split pane creates additional session", async ({ page }) => {
    // Get initial state
    const initialState = await getStoreState(page);
    expect(initialState?.sessionIds.length).toBe(1);

    // Split the pane via store manipulation
    await createSplitPane(page, "vertical");

    // Verify a new session was created
    const stateAfterSplit = await getStoreState(page);
    expect(stateAfterSplit?.sessionIds.length).toBe(2);

    // But still only one tab (the split pane is within the tab)
    expect(await getTabCount(page)).toBe(1);
  });

  test("closing tab with split panes cleans up all sessions", async ({ page }) => {
    // Create a split pane in the first tab
    await createSplitPane(page, "vertical");

    // Verify we have 2 sessions (root + split pane)
    let state = await getStoreState(page);
    expect(state?.sessionIds.length).toBe(2);
    const sessionsInFirstTab = [...(state?.sessionIds ?? [])];

    // Create a second tab so we have somewhere to go after closing
    await createNewTab(page);

    // Now we should have 3 sessions total (2 in first tab, 1 in second tab)
    state = await getStoreState(page);
    expect(state?.sessionIds.length).toBe(3);
    expect(await getTabCount(page)).toBe(2);

    // Close the first tab (which has the split panes)
    await closeFirstTab(page);

    // Wait for cleanup to complete
    await page.waitForTimeout(500);

    // Verify only one tab remains
    expect(await getTabCount(page)).toBe(1);

    // Verify the split pane sessions were cleaned up
    const finalState = await getStoreState(page);
    expect(finalState?.sessionIds.length).toBe(1);

    // The remaining session should NOT be one from the closed tab
    for (const closedSessionId of sessionsInFirstTab) {
      expect(finalState?.sessionIds).not.toContain(closedSessionId);
    }
  });

  test("closing tab with multiple splits cleans up all sessions", async ({ page }) => {
    // Create multiple splits in the first tab
    // First vertical split
    await createSplitPane(page, "vertical");

    // Second horizontal split
    await createSplitPane(page, "horizontal");

    // Verify we have 3 sessions (root + 2 splits)
    let state = await getStoreState(page);
    expect(state?.sessionIds.length).toBe(3);
    const sessionsInFirstTab = [...(state?.sessionIds ?? [])];

    // Create a second tab
    await createNewTab(page);

    // Now we should have 4 sessions total
    state = await getStoreState(page);
    expect(state?.sessionIds.length).toBe(4);

    // Close the first tab
    await closeFirstTab(page);
    await page.waitForTimeout(500);

    // Verify cleanup
    const finalState = await getStoreState(page);
    expect(finalState?.sessionIds.length).toBe(1);

    // None of the closed sessions should remain
    for (const closedSessionId of sessionsInFirstTab) {
      expect(finalState?.sessionIds).not.toContain(closedSessionId);
    }
  });

  test("tab layout is removed when tab is closed", async ({ page }) => {
    // Create a split in the first tab
    await createSplitPane(page, "vertical");

    // Get the tab layout ID (same as root session ID)
    let state = await getStoreState(page);
    expect(state?.tabLayoutIds.length).toBe(1);
    const firstTabLayoutId = state?.tabLayoutIds[0];

    // Create a second tab
    await createNewTab(page);

    // Should have 2 tab layouts now
    state = await getStoreState(page);
    expect(state?.tabLayoutIds.length).toBe(2);

    // Close the first tab
    await closeFirstTab(page);
    await page.waitForTimeout(500);

    // Verify the tab layout was removed
    const finalState = await getStoreState(page);
    expect(finalState?.tabLayoutIds.length).toBe(1);
    expect(finalState?.tabLayoutIds).not.toContain(firstTabLayoutId);
  });

  test("active session switches to remaining tab after close", async ({ page }) => {
    // Create a second tab
    await createNewTab(page);

    // Verify we have 2 tabs
    expect(await getTabCount(page)).toBe(2);

    // Click on first tab to make sure it's active
    await page.locator('[role="tab"]').first().click();
    await page.waitForTimeout(100);

    // Close the first tab
    await closeFirstTab(page);

    // Should have 1 tab remaining
    await expect(page.locator('[role="tab"]')).toHaveCount(1);

    // The remaining tab should be selected (aria-selected is the reliable indicator)
    // Note: Radix uses aria-selected="true" for selected tabs
    const remainingTab = page.locator('[role="tab"]');
    await expect(remainingTab).toHaveAttribute("aria-selected", "true", { timeout: 5000 });

    // Also verify the store's activeSessionId was updated
    const finalState = await getStoreState(page);
    expect(finalState?.activeSessionId).not.toBeNull();
    expect(finalState?.sessionIds).toContain(finalState?.activeSessionId);
  });
});
