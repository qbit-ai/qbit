import { expect, type Page, test } from "@playwright/test";

/**
 * Notification Widget E2E Tests
 *
 * These tests verify the notification widget functionality including:
 * - Displaying notifications in the footer
 * - Preview text appearing for new notifications
 * - Expanded overlay with notification list
 * - Notification actions (mark read, clear, etc.)
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

  // Wait for the notification widget to be visible
  await page.waitForSelector('[data-testid="notification-widget"]', { timeout: 10000 });

  // Clear any notifications added during app initialization (e.g., indexer notifications)
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

  // Wait a tick for the UI to update
  await page.waitForTimeout(100);
}

/**
 * Add a notification via the Zustand store
 */
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

test.describe("Notification Widget - Basic Functionality", () => {
  test.beforeEach(async ({ page }) => {
    await waitForAppReady(page);
  });

  test("notification widget shows zero count initially", async ({ page }) => {
    // Check that the notification button shows 0
    const countBadge = page.locator('[data-testid="notification-widget"] button');
    await expect(countBadge).toContainText("0");
  });

  test("clicking notification button opens the overlay panel", async ({ page }) => {
    // Click the notification button
    await page.locator('[data-testid="notification-widget"] button').click();

    // Verify overlay is visible
    await expect(page.getByText("Notifications", { exact: true })).toBeVisible({ timeout: 3000 });
    await expect(page.locator("text=No notifications")).toBeVisible();
    await expect(page.locator("text=You're all caught up!")).toBeVisible();
  });

  test("clicking outside closes the overlay", async ({ page }) => {
    // Open the overlay
    await page.locator('[data-testid="notification-widget"] button').click();
    await expect(page.getByText("Notifications", { exact: true })).toBeVisible();

    // Click outside (on the main content area)
    await page.locator("body").click({ position: { x: 100, y: 100 } });

    // Verify overlay is closed
    await expect(page.locator("text=No notifications")).not.toBeVisible();
  });

  test("pressing Escape closes the overlay", async ({ page }) => {
    // Open the overlay
    await page.locator('[data-testid="notification-widget"] button').click();
    await expect(page.getByText("Notifications", { exact: true })).toBeVisible();

    // Press Escape
    await page.keyboard.press("Escape");

    // Verify overlay is closed
    await expect(page.locator("text=No notifications")).not.toBeVisible();
  });
});

test.describe("Notification Widget - Adding Notifications", () => {
  test.beforeEach(async ({ page }) => {
    await waitForAppReady(page);
  });

  test("adding a notification updates the count badge", async ({ page }) => {
    // Add a notification
    await addNotification(page, "success", "Test Notification");

    // Check count updates to 1
    const countBadge = page.locator('[data-testid="notification-widget"] button');
    await expect(countBadge).toContainText("1", { timeout: 3000 });
  });

  test("adding multiple notifications shows correct count", async ({ page }) => {
    // Add multiple notifications
    await addNotification(page, "success", "Notification 1");
    await addNotification(page, "info", "Notification 2");
    await addNotification(page, "warning", "Notification 3");

    // Check count updates to 3
    const countBadge = page.locator('[data-testid="notification-widget"] button');
    await expect(countBadge).toContainText("3", { timeout: 3000 });
  });

  test("notifications appear in the overlay list", async ({ page }) => {
    // Add notifications
    await addNotification(page, "success", "Build Complete", "Your project built successfully");
    await addNotification(page, "error", "Connection Failed", "Unable to reach server");

    // Open the overlay
    await page.locator('[data-testid="notification-widget"] button').click();

    // Verify notifications are listed
    await expect(page.locator("text=Build Complete")).toBeVisible({ timeout: 3000 });
    await expect(page.locator("text=Connection Failed")).toBeVisible();
  });
});

test.describe("Notification Widget - Preview Feature", () => {
  test.beforeEach(async ({ page }) => {
    await waitForAppReady(page);
  });

  test("new notification shows preview text in footer", async ({ page }) => {
    // Add a notification
    await addNotification(page, "success", "Task Completed");

    // Verify preview text appears (should show the title)
    await expect(page.locator('[data-testid="notification-preview"]')).toContainText(
      "Task Completed",
      {
        timeout: 3000,
      }
    );
  });

  test("preview text disappears after 5 seconds", async ({ page }) => {
    // Add a notification
    await addNotification(page, "info", "Processing...");

    // Verify preview is visible
    await expect(page.locator('[data-testid="notification-preview"]')).toBeVisible({
      timeout: 3000,
    });

    // Wait for preview to disappear (5 seconds + buffer)
    await page.waitForTimeout(6000);

    // Verify preview is gone
    await expect(page.locator('[data-testid="notification-preview"]')).not.toBeVisible();
  });

  test("opening overlay clears the preview", async ({ page }) => {
    // Add a notification
    await addNotification(page, "warning", "Low Storage");

    // Verify preview appears
    await expect(page.locator('[data-testid="notification-preview"]')).toBeVisible({
      timeout: 3000,
    });

    // Open the overlay
    await page.locator('[data-testid="notification-widget"] button').click();

    // Verify preview is cleared (overlay content visible instead)
    await expect(page.getByText("Notifications", { exact: true })).toBeVisible();
    await expect(page.locator('[data-testid="notification-preview"]')).not.toBeVisible();
  });
});

test.describe("Notification Widget - Actions", () => {
  test.beforeEach(async ({ page }) => {
    await waitForAppReady(page);
  });

  test("clicking a notification marks it as read", async ({ page }) => {
    // Add an unread notification
    await addNotification(page, "info", "Unread Message");

    // Open overlay and click the notification
    await page.locator('[data-testid="notification-widget"] button').click();
    await page.locator("text=Unread Message").click();

    // The count should go to 0 (no unread)
    await page.keyboard.press("Escape");
    const countBadge = page.locator('[data-testid="notification-widget"] button');
    await expect(countBadge).toContainText("0", { timeout: 3000 });
  });

  test("mark all as read button clears unread count", async ({ page }) => {
    // Add multiple notifications
    await addNotification(page, "success", "Notification 1");
    await addNotification(page, "info", "Notification 2");
    await addNotification(page, "warning", "Notification 3");

    // Open overlay
    await page.locator('[data-testid="notification-widget"] button').click();

    // Click mark all as read button (checkmark icon)
    await page.locator('button[title="Mark all as read"]').click();

    // Close and verify count is 0
    await page.keyboard.press("Escape");
    const countBadge = page.locator('[data-testid="notification-widget"] button');
    await expect(countBadge).toContainText("0", { timeout: 3000 });
  });

  test("clear all button removes all notifications", async ({ page }) => {
    // Add notifications
    await addNotification(page, "success", "Test 1");
    await addNotification(page, "error", "Test 2");

    // Open overlay
    await page.locator('[data-testid="notification-widget"] button').click();
    await expect(page.locator("text=Test 1")).toBeVisible();

    // Click clear all button (trash icon)
    await page.locator('button[title="Clear all"]').click();

    // Verify empty state
    await expect(page.locator("text=No notifications")).toBeVisible();
    await expect(page.locator("text=You're all caught up!")).toBeVisible();
  });

  test("remove button on notification deletes single notification", async ({ page }) => {
    // Add two notifications
    await addNotification(page, "success", "Keep This");
    await addNotification(page, "error", "Delete This");

    // Open overlay
    await page.locator('[data-testid="notification-widget"] button').click();

    // Hover over the notification to reveal the X button, then click it
    const notification = page.locator('[data-testid^="notification-item"]', {
      hasText: "Delete This",
    });
    await notification.hover();
    await notification.locator("button").last().click();

    // Verify only one notification remains
    await expect(page.locator("text=Delete This")).not.toBeVisible();
    await expect(page.locator("text=Keep This")).toBeVisible();
  });
});

test.describe("Notification Widget - Notification Types", () => {
  test.beforeEach(async ({ page }) => {
    await waitForAppReady(page);
  });

  test("success notifications display with green styling", async ({ page }) => {
    await addNotification(page, "success", "Success Message");
    await page.locator('[data-testid="notification-widget"] button').click();

    // Verify the notification has success styling (green color)
    const successIcon = page.locator('[data-testid="notification-item-success"]');
    await expect(successIcon).toBeVisible({ timeout: 3000 });
  });

  test("error notifications display with red styling", async ({ page }) => {
    await addNotification(page, "error", "Error Message");
    await page.locator('[data-testid="notification-widget"] button').click();

    const errorIcon = page.locator('[data-testid="notification-item-error"]');
    await expect(errorIcon).toBeVisible({ timeout: 3000 });
  });

  test("warning notifications display with yellow styling", async ({ page }) => {
    await addNotification(page, "warning", "Warning Message");
    await page.locator('[data-testid="notification-widget"] button').click();

    const warningIcon = page.locator('[data-testid="notification-item-warning"]');
    await expect(warningIcon).toBeVisible({ timeout: 3000 });
  });

  test("info notifications display with blue styling", async ({ page }) => {
    await addNotification(page, "info", "Info Message");
    await page.locator('[data-testid="notification-widget"] button').click();

    const infoIcon = page.locator('[data-testid="notification-item-info"]');
    await expect(infoIcon).toBeVisible({ timeout: 3000 });
  });
});

test.describe("Notification Widget - MockDevTools Integration", () => {
  test.beforeEach(async ({ page }) => {
    await waitForAppReady(page);

    // Open MockDevTools
    await page.locator('button[title="Toggle Mock Dev Tools"]').click();
    await expect(page.locator("text=Mock Dev Tools")).toBeVisible();

    // Switch to Notifications tab
    await page.getByRole("button", { name: "Notifications", exact: true }).click();
  });

  test("Emit Success Notification button adds a success notification", async ({ page }) => {
    await page.locator("button:has-text('Emit Success')").click();

    // Verify notification appears
    const countBadge = page.locator('[data-testid="notification-widget"] button');
    await expect(countBadge).toContainText("1", { timeout: 3000 });
  });

  test("Emit Error Notification button adds an error notification", async ({ page }) => {
    await page.locator("button:has-text('Emit Error')").click();

    const countBadge = page.locator('[data-testid="notification-widget"] button');
    await expect(countBadge).toContainText("1", { timeout: 3000 });
  });

  test("Emit Info Notification button adds an info notification", async ({ page }) => {
    await page.locator("button:has-text('Emit Info')").click();

    const countBadge = page.locator('[data-testid="notification-widget"] button');
    await expect(countBadge).toContainText("1", { timeout: 3000 });
  });

  test("Emit Warning Notification button adds a warning notification", async ({ page }) => {
    await page.locator("button:has-text('Emit Warning')").click();

    const countBadge = page.locator('[data-testid="notification-widget"] button');
    await expect(countBadge).toContainText("1", { timeout: 3000 });
  });

  test("Custom notification with title and message", async ({ page }) => {
    // Fill in custom title and message
    const titleInput = page.locator('input[placeholder*="title"]');
    await titleInput.waitFor({ state: "visible" });
    await titleInput.fill("Custom Title");

    const messageInput = page.locator('input[placeholder*="message"]');
    await messageInput.fill("Custom message content");

    // Emit the notification
    await page.locator("button:has-text('Emit Custom')").click();

    // Wait for notification to be registered
    const countBadge = page.locator('[data-testid="notification-widget"] button');
    await expect(countBadge).toContainText("1", { timeout: 3000 });

    // Open notification overlay and verify
    await countBadge.click();
    await expect(page.locator("text=Custom Title")).toBeVisible({ timeout: 3000 });
    await expect(page.locator("text=Custom message content")).toBeVisible({ timeout: 3000 });
  });

  test("Clear All Notifications button removes all notifications", async ({ page }) => {
    // Add some notifications first
    await page.locator("button:has-text('Emit Success')").click();
    await page.locator("button:has-text('Emit Error')").click();
    await page.locator("button:has-text('Emit Info')").click();

    // Verify count is 3
    const countBadge = page.locator('[data-testid="notification-widget"] button');
    await expect(countBadge).toContainText("3", { timeout: 3000 });

    // Click Clear All
    await page.locator("button:has-text('Clear All Notifications')").click();

    // Verify count is 0
    await expect(countBadge).toContainText("0", { timeout: 3000 });
  });
});
