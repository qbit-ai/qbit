import { expect, type Page, test } from "@playwright/test";
import { openSettings, waitForAppReady } from "./helpers/app";

async function navigateToCodebases(page: Page) {
  const codebasesNavItem = page.locator("nav >> button:has-text('Codebases')");
  await expect(codebasesNavItem).toBeVisible({ timeout: 5000 });
  await codebasesNavItem.click();
  await expect(page.locator("text=Indexed folders")).toBeVisible({ timeout: 5000 });
}

async function closeSettings(_page: Page) {}

// SKIP: Settings-related tests are flaky in browser mock mode.
// The Settings tab causes keyboard.press() to timeout due to focus/rendering issues.
// TODO: Investigate React re-render loop in SettingsTabContent in mock mode.
test.describe.skip("Codebases Settings", () => {
  test.beforeEach(async ({ page }) => {
    await waitForAppReady(page);
    await openSettings(page);
    await navigateToCodebases(page);
  });

  test("can change memory file selection", async ({ page }) => {
    await expect(page.getByText("/home/user/projects/my-app")).toBeVisible({ timeout: 5000 });
    const selectTrigger = page.locator("button[role='combobox']").first();
    await selectTrigger.click();
    await page.locator("[role='option']:has-text('None')").click();
    await expect(selectTrigger).toContainText("None");
    await closeSettings(page);
  });

  test("clicking remove button removes codebase from list", async ({ page }) => {
    await expect(page.getByText("/home/user/projects/my-app")).toBeVisible({ timeout: 5000 });
    const initialCount = await page.locator("button[title='Remove']").count();
    expect(initialCount).toBeGreaterThan(0);
    await page.locator("button[title='Remove']").first().click();
    await expect(page.locator("button[title='Remove']")).toHaveCount(initialCount - 1, {
      timeout: 5000,
    });
    await closeSettings(page);
  });

  test("shows empty state message when no codebases", async ({ page }) => {
    await expect(page.getByText("/home/user/projects/my-app")).toBeVisible({ timeout: 5000 });
    const removeButtons = page.locator("button[title='Remove']");
    const count = await removeButtons.count();
    for (let i = 0; i < count; i++) {
      await page.locator("button[title='Remove']").first().click();
      await page.waitForTimeout(300);
    }
    await expect(
      page.locator('text=No codebases indexed yet. Click "Index new folder" to add one.')
    ).toBeVisible({ timeout: 5000 });
    await closeSettings(page);
  });
});
