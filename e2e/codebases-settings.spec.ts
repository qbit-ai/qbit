import { expect, type Page, test } from "@playwright/test";

/**
 * Codebases Settings E2E Tests
 *
 * These tests verify that the Codebases settings tab works correctly:
 * - The Codebases tab is visible in the settings navigation
 * - Indexed codebases are listed with their status
 * - Memory file dropdown works correctly
 * - Reindex and Remove buttons work
 * - "Index new folder" button is visible
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
}

/**
 * Open the settings dialog via keyboard shortcut.
 */
async function openSettings(page: Page) {
  await page.keyboard.press("Meta+,");
  // Wait for settings dialog to appear
  await expect(page.locator("text=Settings").first()).toBeVisible({ timeout: 5000 });
}

/**
 * Navigate to the Codebases section in settings.
 */
async function navigateToCodebases(page: Page) {
  // Click on the "Codebases" nav item in the sidebar
  const codebasesNavItem = page.locator("nav >> button:has-text('Codebases')");
  await expect(codebasesNavItem).toBeVisible({ timeout: 5000 });
  await codebasesNavItem.click();

  // Wait for the Codebases section content to appear
  await expect(page.locator("text=Indexed folders")).toBeVisible({ timeout: 5000 });
}

/**
 * Close the settings dialog by clicking Cancel.
 */
async function closeSettings(page: Page) {
  await page.locator("button:has-text('Cancel')").click();
  await expect(page.getByRole("dialog")).not.toBeVisible({ timeout: 3000 });
}

test.describe("Codebases Settings - Navigation", () => {
  test.beforeEach(async ({ page }) => {
    await waitForAppReady(page);
  });

  test("settings dialog shows Codebases tab in navigation", async ({ page }) => {
    await openSettings(page);

    // The Codebases navigation item should be visible
    const codebasesNavItem = page.locator("nav >> button:has-text('Codebases')");
    await expect(codebasesNavItem).toBeVisible();

    await closeSettings(page);
  });

  test("clicking Codebases tab navigates to codebases section", async ({ page }) => {
    await openSettings(page);
    await navigateToCodebases(page);

    // Should show the Codebases section header
    await expect(page.locator("text=Indexed folders")).toBeVisible();
    await expect(
      page.locator("text=Manage codebases indexed for AI context and code search")
    ).toBeVisible();

    await closeSettings(page);
  });
});

test.describe("Codebases Settings - Codebase List", () => {
  test.beforeEach(async ({ page }) => {
    await waitForAppReady(page);
  });

  test("displays indexed codebases with paths", async ({ page }) => {
    await openSettings(page);
    await navigateToCodebases(page);

    // Should show the mock codebases (use getByText to avoid regex interpretation)
    await expect(page.getByText("/home/user/projects/my-app")).toBeVisible({ timeout: 5000 });
    await expect(page.getByText("/home/user/projects/backend-api")).toBeVisible({
      timeout: 5000,
    });

    await closeSettings(page);
  });

  test("displays codebase status indicators", async ({ page }) => {
    await openSettings(page);
    await navigateToCodebases(page);

    // Should show "Synced" status for at least one codebase
    await expect(page.locator("text=Synced").first()).toBeVisible({ timeout: 5000 });

    await closeSettings(page);
  });

  test("displays file count for indexed codebases", async ({ page }) => {
    await openSettings(page);
    await navigateToCodebases(page);

    // Should show file count (e.g., "(150 files)")
    // The count pattern like "(150 files)" or "(89 files)"
    await expect(page.locator("text=/\\(\\d+ files\\)/").first()).toBeVisible({ timeout: 5000 });

    await closeSettings(page);
  });

  test("shows 'Index new folder' button", async ({ page }) => {
    await openSettings(page);
    await navigateToCodebases(page);

    // The "Index new folder" button should be visible
    const indexButton = page.locator("button:has-text('Index new folder')");
    await expect(indexButton).toBeVisible();

    await closeSettings(page);
  });
});

test.describe("Codebases Settings - Memory File Dropdown", () => {
  test.beforeEach(async ({ page }) => {
    await waitForAppReady(page);
  });

  test("displays memory file dropdown for each codebase", async ({ page }) => {
    await openSettings(page);
    await navigateToCodebases(page);

    // Wait for codebases to load
    await expect(page.getByText("/home/user/projects/my-app")).toBeVisible({ timeout: 5000 });

    // Should have select triggers for memory file selection
    const selectTriggers = page.locator("button[role='combobox']");
    await expect(selectTriggers.first()).toBeVisible();

    await closeSettings(page);
  });

  test("memory file dropdown shows available options when clicked", async ({ page }) => {
    await openSettings(page);
    await navigateToCodebases(page);

    // Wait for codebases to load
    await expect(page.getByText("/home/user/projects/my-app")).toBeVisible({ timeout: 5000 });

    // Click on the first memory file dropdown
    const selectTrigger = page.locator("button[role='combobox']").first();
    await selectTrigger.click();

    // Should show the memory file options
    await expect(page.locator("[role='option']:has-text('None')")).toBeVisible({ timeout: 3000 });
    await expect(page.locator("[role='option']:has-text('AGENTS.md')")).toBeVisible({
      timeout: 3000,
    });
    await expect(page.locator("[role='option']:has-text('CLAUDE.md')")).toBeVisible({
      timeout: 3000,
    });

    // Close the dropdown by pressing Escape
    await page.keyboard.press("Escape");
    await closeSettings(page);
  });

  test("can change memory file selection", async ({ page }) => {
    await openSettings(page);
    await navigateToCodebases(page);

    // Wait for codebases to load
    await expect(page.getByText("/home/user/projects/my-app")).toBeVisible({ timeout: 5000 });

    // Get the first codebase row's dropdown using a more specific selector
    // The row container has rounded-lg and border classes
    const selectTrigger = page.locator("button[role='combobox']").first();

    // Click to open dropdown
    await selectTrigger.click();

    // Select "None" option
    await page.locator("[role='option']:has-text('None')").click();

    // The dropdown should now show "None" as selected
    await expect(selectTrigger).toContainText("None");

    await closeSettings(page);
  });
});

test.describe("Codebases Settings - Action Buttons", () => {
  test.beforeEach(async ({ page }) => {
    await waitForAppReady(page);
  });

  test("displays reindex button for each codebase", async ({ page }) => {
    await openSettings(page);
    await navigateToCodebases(page);

    // Wait for codebases to load
    await expect(page.getByText("/home/user/projects/my-app")).toBeVisible({ timeout: 5000 });

    // Should have reindex buttons (RefreshCw icon)
    const reindexButtons = page.locator("button[title='Re-index']");
    await expect(reindexButtons.first()).toBeVisible();

    await closeSettings(page);
  });

  test("displays remove button for each codebase", async ({ page }) => {
    await openSettings(page);
    await navigateToCodebases(page);

    // Wait for codebases to load
    await expect(page.getByText("/home/user/projects/my-app")).toBeVisible({ timeout: 5000 });

    // Should have remove buttons (Trash2 icon)
    const removeButtons = page.locator("button[title='Remove']");
    await expect(removeButtons.first()).toBeVisible();

    await closeSettings(page);
  });

  test("clicking remove button removes codebase from list", async ({ page }) => {
    await openSettings(page);
    await navigateToCodebases(page);

    // Wait for codebases to load
    await expect(page.getByText("/home/user/projects/my-app")).toBeVisible({ timeout: 5000 });

    // Count initial codebases
    const initialCount = await page.locator("button[title='Remove']").count();
    expect(initialCount).toBeGreaterThan(0);

    // Click the first remove button
    const removeButton = page.locator("button[title='Remove']").first();
    await removeButton.click();

    // Wait for the removal to complete (the count should decrease)
    await expect(page.locator("button[title='Remove']")).toHaveCount(initialCount - 1, {
      timeout: 5000,
    });

    await closeSettings(page);
  });

  test("clicking reindex button triggers reindexing", async ({ page }) => {
    await openSettings(page);
    await navigateToCodebases(page);

    // Wait for codebases to load
    await expect(page.getByText("/home/user/projects/my-app")).toBeVisible({ timeout: 5000 });

    // Count the reindex buttons to verify codebases are present
    const initialCount = await page.locator("button[title='Re-index']").count();
    expect(initialCount).toBeGreaterThan(0);

    // Click the first reindex button
    const reindexButton = page.locator("button[title='Re-index']").first();
    await reindexButton.click();

    // After reindexing completes, the codebase should still be in the list
    // The count should remain the same (reindex doesn't remove the codebase)
    await expect(page.locator("button[title='Re-index']")).toHaveCount(initialCount, {
      timeout: 5000,
    });

    await closeSettings(page);
  });
});

test.describe("Codebases Settings - Empty State", () => {
  test("shows empty state message when no codebases", async ({ page }) => {
    await waitForAppReady(page);
    await openSettings(page);
    await navigateToCodebases(page);

    // Wait for codebases to load
    await expect(page.getByText("/home/user/projects/my-app")).toBeVisible({ timeout: 5000 });

    // Remove all codebases
    const removeButtons = page.locator("button[title='Remove']");
    const count = await removeButtons.count();

    for (let i = 0; i < count; i++) {
      // Always click the first button since the list shrinks after each removal
      await page.locator("button[title='Remove']").first().click();
      // Wait a bit for the removal to process
      await page.waitForTimeout(300);
    }

    // Should show empty state message
    await expect(
      page.locator('text=No codebases indexed yet. Click "Index new folder" to add one.')
    ).toBeVisible({ timeout: 5000 });

    await closeSettings(page);
  });
});
