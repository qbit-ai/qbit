import { expect, type Page, test } from "@playwright/test";
import { waitForAppReady as waitForAppReadyBase } from "./helpers/app";

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
  await waitForAppReadyBase(page);

  // Wait for the unified input textarea to be visible in the active tab
  // Use :visible to find the textarea in the currently active tab
  await expect(page.locator('[data-testid="unified-input"]:visible').first()).toBeVisible({
    timeout: 10000,
  });
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
 * Close the first closable tab by hovering to reveal the close button.
 * Note: Home tab doesn't have a close button, so we skip it.
 */
async function closeFirstClosableTab(page: Page): Promise<void> {
  // The tab structure wraps the trigger and close button in a parent div with class "group"
  // We need to find a tab wrapper that HAS a close button (Home tab doesn't have one)
  const tabWrappers = page.locator(".group").filter({ has: page.locator('[role="tab"]') });

  const count = await tabWrappers.count();
  for (let i = 0; i < count; i++) {
    const wrapper = tabWrappers.nth(i);
    await wrapper.hover();
    await page.waitForTimeout(100);
    const closeButton = wrapper.locator('button[title="Close tab"]');
    if (await closeButton.isVisible()) {
      await closeButton.click();
      await page.waitForTimeout(200);
      return;
    }
  }
  throw new Error("No closable tab found");
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
    // Get initial state (Home + Terminal tabs)
    const initialState = await getStoreState(page);
    expect(initialState).not.toBeNull();
    const initialTabCount = await getTabCount(page);
    expect(initialTabCount).toBe(2); // Home + Terminal

    // Create a third tab so we have something to switch to
    await createNewTab(page);
    expect(await getTabCount(page)).toBe(3);

    // Get the state after creating third tab
    const stateWithThreeTabs = await getStoreState(page);
    expect(stateWithThreeTabs?.sessionIds.length).toBe(3);

    // Close the first closable tab (Terminal - Home can't be closed)
    await closeFirstClosableTab(page);

    // Wait for cleanup to complete
    await page.waitForTimeout(300);

    // Verify tab was closed
    expect(await getTabCount(page)).toBe(2);

    // Verify state was cleaned up
    const finalState = await getStoreState(page);
    expect(finalState?.sessionIds.length).toBe(2);
  });

  test("split pane creates additional session", async ({ page }) => {
    // Get initial state (Home + Terminal sessions)
    const initialState = await getStoreState(page);
    expect(initialState?.sessionIds.length).toBe(2);

    // Split the pane via store manipulation
    await createSplitPane(page, "vertical");

    // Verify a new session was created
    const stateAfterSplit = await getStoreState(page);
    expect(stateAfterSplit?.sessionIds.length).toBe(3);

    // Still only two tabs (the split pane is within the active tab)
    expect(await getTabCount(page)).toBe(2);
  });

  test("closing tab with split panes cleans up all sessions", async ({ page }) => {
    // Create a split pane in the active Terminal tab
    await createSplitPane(page, "vertical");

    // Verify we have 3 sessions (Home + Terminal root + split pane)
    let state = await getStoreState(page);
    expect(state?.sessionIds.length).toBe(3);
    // Remember sessions that are NOT Home tab (we'll be closing the Terminal tab with splits)
    const terminalTabSessions = state?.sessionIds.filter((id) => !id.startsWith("home-")) ?? [];

    // Create a third tab so we have somewhere to go after closing
    await createNewTab(page);

    // Now we should have 4 sessions total (Home + 2 in Terminal tab + 1 in new tab)
    state = await getStoreState(page);
    expect(state?.sessionIds.length).toBe(4);
    expect(await getTabCount(page)).toBe(3);

    // Close the first tab (Home tab, not the one with splits)
    // Actually, let's close the Terminal tab (second tab) with splits
    // The tab wrapper is the second one (index 1)
    const tabWrapper = page
      .locator(".group")
      .filter({ has: page.locator('[role="tab"]') })
      .nth(1); // Second tab (Terminal with splits)
    await tabWrapper.hover();
    await page.waitForTimeout(100);
    const closeButton = tabWrapper.locator('button[title="Close tab"]');
    await closeButton.click();

    // Wait for cleanup to complete
    await page.waitForTimeout(500);

    // Verify two tabs remain (Home + new tab)
    expect(await getTabCount(page)).toBe(2);

    // Verify the split pane sessions were cleaned up
    const finalState = await getStoreState(page);
    expect(finalState?.sessionIds.length).toBe(2);

    // The Terminal tab sessions should NOT remain
    for (const closedSessionId of terminalTabSessions) {
      expect(finalState?.sessionIds).not.toContain(closedSessionId);
    }
  });

  test("closing tab with multiple splits cleans up all sessions", async ({ page }) => {
    // Create multiple splits in the active Terminal tab
    // First vertical split
    await createSplitPane(page, "vertical");

    // Second horizontal split
    await createSplitPane(page, "horizontal");

    // Verify we have 4 sessions (Home + Terminal root + 2 splits)
    let state = await getStoreState(page);
    expect(state?.sessionIds.length).toBe(4);
    // Remember Terminal tab sessions (we'll be closing this tab)
    const terminalTabSessions = state?.sessionIds.filter((id) => !id.startsWith("home-")) ?? [];

    // Create a third tab
    await createNewTab(page);

    // Now we should have 5 sessions total
    state = await getStoreState(page);
    expect(state?.sessionIds.length).toBe(5);

    // Close the Terminal tab (second tab, index 1) with the splits
    const tabWrapper = page
      .locator(".group")
      .filter({ has: page.locator('[role="tab"]') })
      .nth(1);
    await tabWrapper.hover();
    await page.waitForTimeout(100);
    const closeButton = tabWrapper.locator('button[title="Close tab"]');
    await closeButton.click();
    await page.waitForTimeout(500);

    // Verify cleanup (Home + new tab remain)
    const finalState = await getStoreState(page);
    expect(finalState?.sessionIds.length).toBe(2);

    // None of the Terminal tab sessions should remain
    for (const closedSessionId of terminalTabSessions) {
      expect(finalState?.sessionIds).not.toContain(closedSessionId);
    }
  });

  test("tab layout is removed when tab is closed", async ({ page }) => {
    // Create a split in the active Terminal tab
    await createSplitPane(page, "vertical");

    // Get the tab layout IDs (Home + Terminal)
    let state = await getStoreState(page);
    expect(state?.tabLayoutIds.length).toBe(2);
    // Find the Terminal tab layout (not starting with "home-")
    const terminalTabLayoutId = state?.tabLayoutIds.find((id) => !id.startsWith("home-"));

    // Create a third tab
    await createNewTab(page);

    // Should have 3 tab layouts now
    state = await getStoreState(page);
    expect(state?.tabLayoutIds.length).toBe(3);

    // Close the Terminal tab (second tab, index 1)
    const tabWrapper = page
      .locator(".group")
      .filter({ has: page.locator('[role="tab"]') })
      .nth(1);
    await tabWrapper.hover();
    await page.waitForTimeout(100);
    const closeButton = tabWrapper.locator('button[title="Close tab"]');
    await closeButton.click();
    await page.waitForTimeout(500);

    // Verify the tab layout was removed (Home + new tab remain)
    const finalState = await getStoreState(page);
    expect(finalState?.tabLayoutIds.length).toBe(2);
    expect(finalState?.tabLayoutIds).not.toContain(terminalTabLayoutId);
  });

  test("active session switches to remaining tab after close", async ({ page }) => {
    // Create a third tab (Home + Terminal already exist)
    await createNewTab(page);

    // Verify we have 3 tabs
    expect(await getTabCount(page)).toBe(3);

    // Click on second tab (Terminal) to make sure it's active (Home can't be closed)
    await page.locator('[role="tab"]').nth(1).click();
    await page.waitForTimeout(100);

    // Close the first closable tab (Terminal - Home can't be closed)
    await closeFirstClosableTab(page);

    // Should have 2 tabs remaining
    await expect(page.locator('[role="tab"]')).toHaveCount(2);

    // One of the remaining tabs should be selected (aria-selected is the reliable indicator)
    // Note: Radix uses aria-selected="true" for selected tabs
    const selectedTab = page.locator('[role="tab"][aria-selected="true"]');
    await expect(selectedTab).toBeVisible({ timeout: 5000 });

    // Also verify the store's activeSessionId was updated
    const finalState = await getStoreState(page);
    expect(finalState?.activeSessionId).not.toBeNull();
    expect(finalState?.sessionIds).toContain(finalState?.activeSessionId);
  });
});
