import { expect, type Page, test } from "@playwright/test";

async function waitForAppReady(page: Page) {
  await page.goto("/");
  await page.waitForLoadState("domcontentloaded");
  await page.waitForFunction(
    () => (window as unknown as { __MOCK_BROWSER_MODE__?: boolean }).__MOCK_BROWSER_MODE__ === true,
    { timeout: 15000 }
  );
  await page.waitForSelector('[data-testid="notification-widget"]', { timeout: 10000 });
  await page.evaluate(() => {
    const store = (
      window as unknown as {
        __QBIT_STORE__?: { getState: () => { clearNotifications: () => void } };
      }
    ).__QBIT_STORE__;
    if (store) {
      store.getState().clearNotifications();
    }
  });
  await page.waitForTimeout(100);
}

async function addNotification(
  page: Page,
  type: "info" | "success" | "warning" | "error",
  title: string,
  message?: string
) {
  return page.evaluate(
    ({ type, title, message }) => {
      const store = (
        window as unknown as {
          __QBIT_STORE__?: { getState: () => { addNotification: (n: unknown) => void } };
        }
      ).__QBIT_STORE__;
      if (store) {
        store.getState().addNotification({ type, title, message });
        return true;
      }
      return false;
    },
    { type, title, message }
  );
}

test.describe("Notification Widget - Core behavior", () => {
  test.beforeEach(async ({ page }) => {
    await waitForAppReady(page);
  });

  test("adding a notification updates badge and overlay list", async ({ page }) => {
    await addNotification(page, "success", "Build Complete", "Your project built successfully");

    const countBadge = page.locator('[data-testid="notification-widget"] button');
    await expect(countBadge).toContainText("1", { timeout: 3000 });

    await countBadge.click();
    await expect(page.locator("text=Build Complete")).toBeVisible({ timeout: 3000 });
    await expect(page.locator("text=Your project built successfully")).toBeVisible();
  });

  test("clear all removes notifications and resets badge", async ({ page }) => {
    await addNotification(page, "success", "First");
    await addNotification(page, "error", "Second");

    const countBadge = page.locator('[data-testid="notification-widget"] button');
    await expect(countBadge).toContainText("2", { timeout: 3000 });

    await countBadge.click();
    await expect(page.locator("text=First")).toBeVisible();
    await expect(page.locator("text=Second")).toBeVisible();

    // Use page.evaluate to programmatically click since the panel has z-index/pointer issues
    await page.evaluate(() => {
      const clearButton = document.querySelector('button[title="Clear all"]') as HTMLButtonElement;
      if (clearButton) clearButton.click();
    });
    await expect(page.locator("text=No notifications")).toBeVisible();
    await expect(page.locator("text=You're all caught up!")).toBeVisible();
    await expect(countBadge).toContainText("0", { timeout: 3000 });
  });
});
