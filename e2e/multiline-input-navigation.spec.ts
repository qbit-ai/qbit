import { expect, type Page, test } from "@playwright/test";

/**
 * Multiline Input Navigation E2E Tests
 *
 * These tests verify that arrow key navigation works correctly in multiline
 * terminal input:
 * - ArrowUp/ArrowDown navigate command history when cursor is on first/last line
 * - ArrowUp/ArrowDown move cursor within text when NOT on first/last line
 * - Draft text is preserved when navigating history and returning
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
 * Get the UnifiedInput textarea element.
 * We use :not(.xterm-helper-textarea) to exclude the xterm.js hidden textarea
 * which is always present due to the terminal portal architecture.
 */
function getInputTextarea(page: Page) {
  return page.locator("textarea:not(.xterm-helper-textarea)");
}

/**
 * Get the Terminal mode toggle button.
 */
function getTerminalModeButton(page: Page) {
  return page.getByRole("button", { name: "Switch to Terminal mode" });
}

/**
 * Ensure we're in terminal mode (required for command history).
 */
async function ensureTerminalMode(page: Page) {
  const textarea = getInputTextarea(page);
  const placeholder = await textarea.getAttribute("placeholder");

  if (placeholder !== "Enter command...") {
    const terminalButton = getTerminalModeButton(page);
    await terminalButton.click();
    await expect(textarea).toHaveAttribute("placeholder", "Enter command...");
  }
}

test.describe("Multiline Input Navigation", () => {
  test.beforeEach(async ({ page }) => {
    await waitForAppReady(page);
    await ensureTerminalMode(page);
  });

  test("ArrowUp navigates history in single-line input", async ({ page }) => {
    const textarea = getInputTextarea(page);

    // Add first command to history
    await page.keyboard.type("first command");
    await page.keyboard.press("Enter");

    // Wait for the textarea to be ready again after submit
    await page.waitForTimeout(100);

    // Add second command to history
    await page.keyboard.type("second command");
    await page.keyboard.press("Enter");

    // Wait for the textarea to be ready again after submit
    await page.waitForTimeout(100);

    // Now press ArrowUp to navigate to previous command
    await page.keyboard.press("ArrowUp");

    // Should show the second command (most recent)
    await expect(textarea).toHaveValue("second command");

    // Press ArrowUp again to get the first command
    await page.keyboard.press("ArrowUp");

    // Should show the first command
    await expect(textarea).toHaveValue("first command");
  });

  test("ArrowUp navigates history when cursor is on first line of multiline input", async ({ page }) => {
    const textarea = getInputTextarea(page);

    // Add a command to history first
    await page.keyboard.type("history entry");
    await page.keyboard.press("Enter");
    await page.waitForTimeout(100);

    // Clear and type multiline text
    await textarea.clear();
    await page.keyboard.type("line1");
    await page.keyboard.press("Shift+Enter");
    await page.keyboard.type("line2");
    await page.keyboard.press("Shift+Enter");
    await page.keyboard.type("line3");

    // Verify we have the multiline text
    await expect(textarea).toHaveValue("line1\nline2\nline3");

    // Move cursor to the beginning (first line) using JavaScript
    await textarea.evaluate((el: HTMLTextAreaElement) => {
      el.selectionStart = 0;
      el.selectionEnd = 0;
    });

    // Press ArrowUp - should navigate to history, not move cursor
    await page.keyboard.press("ArrowUp");

    // Should show the history entry, not the multiline text
    await expect(textarea).toHaveValue("history entry");
  });

  test("ArrowUp moves cursor when NOT on first line of multiline input", async ({ page }) => {
    const textarea = getInputTextarea(page);

    // Type multiline text
    await page.keyboard.type("line1");
    await page.keyboard.press("Shift+Enter");
    await page.keyboard.type("line2");

    // Cursor is at the end (on line2)
    await expect(textarea).toHaveValue("line1\nline2");

    // Press ArrowUp - should move cursor up within the text, not trigger history
    await page.keyboard.press("ArrowUp");

    // The text should remain unchanged
    await expect(textarea).toHaveValue("line1\nline2");

    // Note: We can't easily verify cursor position in e2e tests,
    // but the fact that the text is unchanged proves history wasn't triggered
  });

  test("ArrowDown navigates history when cursor is on last line of multiline input", async ({ page }) => {
    const textarea = getInputTextarea(page);

    // Add two commands to history
    await page.keyboard.type("first");
    await page.keyboard.press("Enter");
    await page.waitForTimeout(100);

    await page.keyboard.type("second");
    await page.keyboard.press("Enter");
    await page.waitForTimeout(100);

    // Navigate up in history
    await page.keyboard.press("ArrowUp");
    await expect(textarea).toHaveValue("second");

    // Create a multiline entry
    await textarea.clear();
    await page.keyboard.type("line1");
    await page.keyboard.press("Shift+Enter");
    await page.keyboard.type("line2");

    // Cursor is on the last line (line2)
    await expect(textarea).toHaveValue("line1\nline2");

    // Press ArrowDown - should navigate forward in history (to empty/draft)
    await page.keyboard.press("ArrowDown");

    // Should clear or return to draft (empty in this case)
    await expect(textarea).toHaveValue("");
  });

  test("ArrowDown moves cursor when NOT on last line of multiline input", async ({ page }) => {
    const textarea = getInputTextarea(page);

    // Type multiline text
    await page.keyboard.type("line1");
    await page.keyboard.press("Shift+Enter");
    await page.keyboard.type("line2");

    // Move cursor to the beginning (on line1) using JavaScript
    await textarea.evaluate((el: HTMLTextAreaElement) => {
      el.selectionStart = 0;
      el.selectionEnd = 0;
    });

    // Press ArrowDown - should move cursor down within the text
    await page.keyboard.press("ArrowDown");

    // The text should remain unchanged (cursor moved, history not triggered)
    await expect(textarea).toHaveValue("line1\nline2");
  });

  test("Draft is preserved when navigating history and returning", async ({ page }) => {
    const textarea = getInputTextarea(page);

    // Build history first
    await page.keyboard.type("old command");
    await page.keyboard.press("Enter");
    await page.waitForTimeout(100);

    // Type a draft
    await page.keyboard.type("my draft text");
    await expect(textarea).toHaveValue("my draft text");

    // Press ArrowUp to navigate to history (should save draft)
    await page.keyboard.press("ArrowUp");
    await expect(textarea).toHaveValue("old command");

    // Press ArrowDown to return (should restore draft)
    await page.keyboard.press("ArrowDown");
    await expect(textarea).toHaveValue("my draft text");
  });

  test("Draft preservation works with multiline input", async ({ page }) => {
    const textarea = getInputTextarea(page);

    // Build history first
    await page.keyboard.type("history command");
    await page.keyboard.press("Enter");
    await page.waitForTimeout(100);

    // Type a multiline draft
    await page.keyboard.type("line1");
    await page.keyboard.press("Shift+Enter");
    await page.keyboard.type("line2");
    await expect(textarea).toHaveValue("line1\nline2");

    // Move cursor to first line using JavaScript
    await textarea.evaluate((el: HTMLTextAreaElement) => {
      el.selectionStart = 0;
      el.selectionEnd = 0;
    });

    // Press ArrowUp to navigate to history (should save multiline draft)
    await page.keyboard.press("ArrowUp");
    await expect(textarea).toHaveValue("history command");

    // Press ArrowDown to return (should restore multiline draft)
    await page.keyboard.press("ArrowDown");
    await expect(textarea).toHaveValue("line1\nline2");
  });

  test("Multiple history navigation cycles preserve draft correctly", async ({ page }) => {
    const textarea = getInputTextarea(page);

    // Build history with multiple commands
    await page.keyboard.type("command1");
    await page.keyboard.press("Enter");
    await page.waitForTimeout(100);

    await page.keyboard.type("command2");
    await page.keyboard.press("Enter");
    await page.waitForTimeout(100);

    await page.keyboard.type("command3");
    await page.keyboard.press("Enter");
    await page.waitForTimeout(100);

    // Type a draft
    await page.keyboard.type("my new draft");
    await expect(textarea).toHaveValue("my new draft");

    // Navigate up through history
    await page.keyboard.press("ArrowUp");
    await expect(textarea).toHaveValue("command3");

    await page.keyboard.press("ArrowUp");
    await expect(textarea).toHaveValue("command2");

    // Navigate back down
    await page.keyboard.press("ArrowDown");
    await expect(textarea).toHaveValue("command3");

    // Continue down to restore draft
    await page.keyboard.press("ArrowDown");
    await expect(textarea).toHaveValue("my new draft");

    // Navigate up again
    await page.keyboard.press("ArrowUp");
    await expect(textarea).toHaveValue("command3");

    // And back down to draft again
    await page.keyboard.press("ArrowDown");
    await expect(textarea).toHaveValue("my new draft");
  });
});