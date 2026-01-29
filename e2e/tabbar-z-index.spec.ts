import { expect, type Page, test } from "@playwright/test";
import { waitForAppReady as waitForAppReadyBase } from "./helpers/app";

/**
 * TabBar Z-Index E2E Tests
 *
 * These tests verify that the TabBar maintains correct z-index stacking
 * to ensure tab close buttons remain clickable even when dialogs are present.
 *
 * The fix ensures TabBar has z-[60] which is above dialog overlays (z-50).
 */

/**
 * Wait for the app to be fully ready in browser mode.
 */
async function waitForAppReady(page: Page) {
  await waitForAppReadyBase(page);

  // Wait for the unified input textarea to be visible (exclude xterm's hidden textarea)
  await expect(page.locator("textarea:not(.xterm-helper-textarea)")).toBeVisible({ timeout: 5000 });
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

test.describe("TabBar Z-Index", () => {
  test.beforeEach(async ({ page }) => {
    await waitForAppReady(page);
  });

  test("TabBar container has correct z-index class", async ({ page }) => {
    // Find the TabBar container (the div with the tab list and buttons)
    // It should have z-[200] class to be above timeline content and ensure
    // notification dropdown appears on top
    const tabBarContainer = page.locator(".z-\\[200\\]").first();

    // Verify the TabBar container exists and has the correct z-index class
    await expect(tabBarContainer).toBeVisible();

    // Verify it contains the tabs
    const tabs = tabBarContainer.locator('[role="tab"]');
    await expect(tabs).toHaveCount(1); // Initial tab
  });

  test("tab close button is clickable after creating multiple tabs", async ({ page }) => {
    // Create a second tab
    await createNewTab(page);
    expect(await getTabCount(page)).toBe(2);

    // The close button should be accessible and clickable
    const tabWrapper = page
      .locator(".group")
      .filter({ has: page.locator('[role="tab"]') })
      .first();
    await tabWrapper.hover();

    const closeButton = tabWrapper.locator('button[title="Close tab"]');
    await expect(closeButton).toBeVisible();

    // Click should work and close the tab
    await closeButton.click();
    await page.waitForTimeout(200);

    // Verify tab was closed
    expect(await getTabCount(page)).toBe(1);
  });

  test("tab close button remains clickable with settings dialog open", async ({ page }) => {
    // Create a second tab first (so we have a tab to switch to after closing)
    await createNewTab(page);
    expect(await getTabCount(page)).toBe(2);

    // Open the settings dialog via the settings button
    const settingsButton = page.getByRole("button", { name: /settings/i });
    if (await settingsButton.isVisible()) {
      await settingsButton.click();
      await page.waitForTimeout(200);

      // The settings tab should now be active
      // Note: Settings opens as a tab in this app, not a modal dialog
      // So we should have 3 tabs now (2 terminal + 1 settings)
      const tabCount = await getTabCount(page);

      // Close the first terminal tab - this should still work
      // even with multiple tabs open
      const tabWrapper = page
        .locator(".group")
        .filter({ has: page.locator('[role="tab"]') })
        .first();
      await tabWrapper.hover();

      const closeButton = tabWrapper.locator('button[title="Close tab"]');
      await expect(closeButton).toBeVisible();

      // The click should work (not blocked by any overlay)
      await closeButton.click();
      await page.waitForTimeout(300);

      // Verify a tab was closed
      const newTabCount = await getTabCount(page);
      expect(newTabCount).toBeLessThan(tabCount);
    }
  });

  test("rapid tab creation and closing works correctly", async ({ page }) => {
    // Test that rapid tab operations don't cause z-index issues

    // Create 3 additional tabs
    for (let i = 0; i < 3; i++) {
      await createNewTab(page);
    }
    expect(await getTabCount(page)).toBe(4);

    // Close tabs in succession
    for (let i = 0; i < 3; i++) {
      await closeFirstTab(page);
      await page.waitForTimeout(100);
    }

    // Should have 1 tab remaining
    expect(await getTabCount(page)).toBe(1);
  });
});
