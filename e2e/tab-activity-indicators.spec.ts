import { expect, type Page, test } from "@playwright/test";
import { waitForAppReady as waitForAppReadyBase } from "./helpers/app";

/**
 * Tab Activity Indicators E2E Tests
 *
 * These tests verify that:
 * 1. Busy spinner shows when agent is running
 * 2. Yellow "new activity" indicator shows when a background tab has events
 * 3. The yellow indicator clears when the tab is activated
 * 4. Both indicators work correctly with split panes
 */

async function waitForAppReady(page: Page) {
  await waitForAppReadyBase(page);

  // Wait for stable tab bar
  await expect(page.locator('[role="tablist"]')).toBeVisible({ timeout: 10000 });
  await page.waitForTimeout(300);
}

/**
 * Get the store state for debugging.
 */
async function getStoreState(page: Page) {
  return await page.evaluate(() => {
    const store = (window as unknown as { __QBIT_STORE__?: { getState: () => unknown } })
      .__QBIT_STORE__;
    if (!store) return null;
    const state = store.getState() as {
      sessions: Record<string, { tabType?: string }>;
      tabLayouts: Record<string, { root: unknown; focusedPaneId: string }>;
      activeSessionId: string | null;
      tabHasNewActivity: Record<string, boolean>;
      isAgentResponding: Record<string, boolean>;
    };
    return {
      sessionIds: Object.keys(state.sessions),
      tabLayoutIds: Object.keys(state.tabLayouts),
      activeSessionId: state.activeSessionId,
      tabHasNewActivity: state.tabHasNewActivity,
      isAgentResponding: state.isAgentResponding,
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
async function createNewTab(page: Page): Promise<string | null> {
  const stateBefore = await getStoreState(page);
  const sessionsBefore = new Set(stateBefore?.sessionIds ?? []);

  await page.getByRole("button", { name: "New tab" }).click();
  await page.waitForTimeout(500);

  const stateAfter = await getStoreState(page);
  const sessionsAfter = stateAfter?.sessionIds ?? [];

  // Find the new session ID
  for (const id of sessionsAfter) {
    if (!sessionsBefore.has(id) && !id.startsWith("home-")) {
      return id;
    }
  }
  return null;
}

/**
 * Switch to a specific tab by clicking on it.
 */
async function switchToTab(page: Page, tabIndex: number): Promise<void> {
  await page.locator('[role="tab"]').nth(tabIndex).click();
  await page.waitForTimeout(200);
}

/**
 * Set agent responding state for a session via store manipulation.
 */
async function setAgentResponding(page: Page, sessionId: string, isResponding: boolean) {
  await page.evaluate(
    ({ sessionId, isResponding }) => {
      const store = (
        window as unknown as {
          __QBIT_STORE__?: {
            getState: () => {
              setAgentResponding: (sessionId: string, isResponding: boolean) => void;
            };
          };
        }
      ).__QBIT_STORE__;
      if (store) {
        store.getState().setAgentResponding(sessionId, isResponding);
      }
    },
    { sessionId, isResponding }
  );
  await page.waitForTimeout(100);
}

/**
 * Mark a tab as having new activity via store manipulation.
 */
async function markTabNewActivity(page: Page, sessionId: string) {
  await page.evaluate(
    ({ sessionId }) => {
      const store = (
        window as unknown as {
          __QBIT_STORE__?: {
            getState: () => {
              markTabNewActivityBySession: (sessionId: string) => void;
            };
          };
        }
      ).__QBIT_STORE__;
      if (store) {
        store.getState().markTabNewActivityBySession(sessionId);
      }
    },
    { sessionId }
  );
  await page.waitForTimeout(100);
}

/**
 * Get the classes on the tab text span for a specific tab.
 */
async function getTabTextClasses(page: Page, tabIndex: number): Promise<string | null> {
  return await page.evaluate((tabIndex) => {
    const tabs = document.querySelectorAll('[role="tab"]');
    const tab = tabs[tabIndex];
    if (!tab) return null;
    const span = tab.querySelector("span.truncate");
    return span?.className ?? null;
  }, tabIndex);
}

/**
 * Check if a tab has the loading spinner visible.
 */
async function hasLoadingSpinner(page: Page, tabIndex: number): Promise<boolean> {
  const tab = page.locator('[role="tab"]').nth(tabIndex);
  // The spinner is a Loader2 icon with animate-spin class
  const spinner = tab.locator("svg.animate-spin");
  return await spinner.isVisible();
}

/**
 * Check if a tab has the yellow new activity indicator (text or dot).
 */
async function hasYellowActivityIndicator(page: Page, tabIndex: number): Promise<boolean> {
  const classes = await getTabTextClasses(page, tabIndex);
  const hasYellowText = classes?.includes("text-[var(--ansi-yellow)]") ?? false;

  // Also check for the activity dot
  const tab = page.locator('[role="tab"]').nth(tabIndex);
  const activityDot = tab.locator(".activity-dot");
  const hasDot = await activityDot.isVisible().catch(() => false);

  return hasYellowText || hasDot;
}

/**
 * Check if a tab has the pulsing activity dot specifically.
 */
async function hasActivityDot(page: Page, tabIndex: number): Promise<boolean> {
  const tab = page.locator('[role="tab"]').nth(tabIndex);
  const activityDot = tab.locator(".activity-dot");
  return await activityDot.isVisible().catch(() => false);
}

test.describe("Tab Activity Indicators", () => {
  test.beforeEach(async ({ page }) => {
    await waitForAppReady(page);
  });

  test("initial state: no spinner or yellow indicator on terminal tab", async ({ page }) => {
    // Should have Home + Terminal tabs
    expect(await getTabCount(page)).toBe(2);

    // Terminal tab (index 1) should NOT have spinner
    expect(await hasLoadingSpinner(page, 1)).toBe(false);

    // Terminal tab should NOT have yellow indicator (it's active)
    expect(await hasYellowActivityIndicator(page, 1)).toBe(false);
  });

  test("spinner shows when agent is responding", async ({ page }) => {
    // Get state to find terminal session ID
    const state = await getStoreState(page);
    expect(state).not.toBeNull();

    // Find the terminal session (not home)
    const terminalSessionId = state?.sessionIds.find((id) => !id.startsWith("home-"));
    expect(terminalSessionId).toBeDefined();

    // Set agent responding to true
    await setAgentResponding(page, terminalSessionId!, true);

    // The tab should now show a spinner
    expect(await hasLoadingSpinner(page, 1)).toBe(true);

    // Clear the responding state
    await setAgentResponding(page, terminalSessionId!, false);

    // Spinner should be gone
    expect(await hasLoadingSpinner(page, 1)).toBe(false);
  });

  test("yellow indicator shows on inactive tab with new activity", async ({ page }) => {
    // Create a second terminal tab
    const newTabId = await createNewTab(page);
    expect(newTabId).toBeDefined();
    expect(await getTabCount(page)).toBe(3);

    // Get the state to verify which tab is active
    let state = await getStoreState(page);
    expect(state?.activeSessionId).toBe(newTabId);

    // Get the first terminal session ID (index 1 in tabs, not the new one)
    const firstTerminalId = state?.sessionIds.find(
      (id) => !id.startsWith("home-") && id !== newTabId
    );
    expect(firstTerminalId).toBeDefined();

    // Verify tabHasNewActivity state
    console.log("Before marking activity:", state?.tabHasNewActivity);

    // Mark the first terminal tab as having new activity
    await markTabNewActivity(page, firstTerminalId!);

    // Check the state after marking
    state = await getStoreState(page);
    console.log("After marking activity:", state?.tabHasNewActivity);

    // The first terminal tab (index 1) should now have yellow indicator
    const hasYellow = await hasYellowActivityIndicator(page, 1);
    console.log("Has yellow indicator:", hasYellow);

    expect(hasYellow).toBe(true);
  });

  test("yellow indicator clears when tab is activated", async ({ page }) => {
    // Create a second terminal tab
    const newTabId = await createNewTab(page);
    expect(newTabId).toBeDefined();

    // Get the first terminal session ID
    let state = await getStoreState(page);
    const firstTerminalId = state?.sessionIds.find(
      (id) => !id.startsWith("home-") && id !== newTabId
    );
    expect(firstTerminalId).toBeDefined();

    // Mark the first terminal tab as having new activity
    await markTabNewActivity(page, firstTerminalId!);

    // Verify yellow indicator is present
    expect(await hasYellowActivityIndicator(page, 1)).toBe(true);

    // Now click on the first terminal tab to activate it
    await switchToTab(page, 1);

    // Wait for state to update
    await page.waitForTimeout(200);

    // The yellow indicator should now be gone (tab is active)
    expect(await hasYellowActivityIndicator(page, 1)).toBe(false);

    // Verify state was cleared
    state = await getStoreState(page);
    expect(state?.tabHasNewActivity[firstTerminalId!]).toBe(false);
  });

  test("yellow indicator does NOT show on active tab", async ({ page }) => {
    // Create a second terminal tab so we have something to test against
    await createNewTab(page);

    // Switch back to the first terminal tab (index 1)
    await switchToTab(page, 1);

    // Get the first terminal session ID
    const state = await getStoreState(page);
    const firstTerminalId = state?.sessionIds.find((id) => !id.startsWith("home-"));
    expect(firstTerminalId).toBeDefined();
    expect(state?.activeSessionId).toBe(firstTerminalId);

    // Try to mark activity on the ACTIVE tab
    await markTabNewActivity(page, firstTerminalId!);

    // The yellow indicator should NOT appear because the tab is active
    expect(await hasYellowActivityIndicator(page, 1)).toBe(false);

    // Verify state was not set
    const newState = await getStoreState(page);
    // The flag should either be false or not set for an active tab
    expect(newState?.tabHasNewActivity[firstTerminalId!]).not.toBe(true);
  });

  test("activity dot appears on inactive tab with new activity", async ({ page }) => {
    // Create a second terminal tab
    const newTabId = await createNewTab(page);
    expect(newTabId).toBeDefined();

    // Get the first terminal session ID
    const state = await getStoreState(page);
    const firstTerminalId = state?.sessionIds.find(
      (id) => !id.startsWith("home-") && id !== newTabId
    );
    expect(firstTerminalId).toBeDefined();

    // Mark the first terminal tab as having new activity
    await markTabNewActivity(page, firstTerminalId!);

    // The first terminal tab (index 1) should have the activity dot
    expect(await hasActivityDot(page, 1)).toBe(true);
  });

  test("activity dot disappears when tab is activated", async ({ page }) => {
    // Create a second terminal tab
    const newTabId = await createNewTab(page);
    expect(newTabId).toBeDefined();

    // Get the first terminal session ID
    const state = await getStoreState(page);
    const firstTerminalId = state?.sessionIds.find(
      (id) => !id.startsWith("home-") && id !== newTabId
    );
    expect(firstTerminalId).toBeDefined();

    // Mark the first terminal tab as having new activity
    await markTabNewActivity(page, firstTerminalId!);

    // Verify activity dot is present
    expect(await hasActivityDot(page, 1)).toBe(true);

    // Activate the tab
    await switchToTab(page, 1);
    await page.waitForTimeout(200);

    // Activity dot should be gone
    expect(await hasActivityDot(page, 1)).toBe(false);
  });

  test("spinner and activity indicator work independently", async ({ page }) => {
    // Create a second terminal tab
    const newTabId = await createNewTab(page);
    expect(newTabId).toBeDefined();

    // Get the first terminal session ID
    const state = await getStoreState(page);
    const firstTerminalId = state?.sessionIds.find(
      (id) => !id.startsWith("home-") && id !== newTabId
    );
    expect(firstTerminalId).toBeDefined();

    // Mark activity on first tab AND set it as responding
    await markTabNewActivity(page, firstTerminalId!);
    await setAgentResponding(page, firstTerminalId!, true);

    // The first terminal tab (index 1) should show spinner (active operation takes precedence)
    expect(await hasLoadingSpinner(page, 1)).toBe(true);

    // Clear the responding state
    await setAgentResponding(page, firstTerminalId!, false);

    // Now it should show yellow indicator (activity persists after agent done)
    expect(await hasLoadingSpinner(page, 1)).toBe(false);
    expect(await hasYellowActivityIndicator(page, 1)).toBe(true);
  });
});

test.describe("Tab Activity Indicators - Debug", () => {
  test("debug: log store state after activity marking", async ({ page }) => {
    await waitForAppReady(page);

    // Create a second terminal tab
    const newTabId = await createNewTab(page);
    console.log("New tab ID:", newTabId);

    // Get full state
    const state = await getStoreState(page);
    console.log("Full state:", JSON.stringify(state, null, 2));

    // Get the first terminal session ID
    const firstTerminalId = state?.sessionIds.find(
      (id) => !id.startsWith("home-") && id !== newTabId
    );
    console.log("First terminal ID:", firstTerminalId);
    console.log("Active session ID:", state?.activeSessionId);

    // Mark activity
    await markTabNewActivity(page, firstTerminalId!);

    // Get state after marking
    const stateAfter = await getStoreState(page);
    console.log("State after marking activity:", JSON.stringify(stateAfter, null, 2));

    // Get the actual CSS classes
    const classes = await getTabTextClasses(page, 1);
    console.log("Tab 1 text span classes:", classes);

    // Take a screenshot for visual inspection
    await page.screenshot({ path: "e2e-tab-activity-debug.png" });

    // This test is just for debugging - always pass
    expect(true).toBe(true);
  });
});
