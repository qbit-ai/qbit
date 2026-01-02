import { expect, type Page, test } from "@playwright/test";

/**
 * Input Mode Focus E2E Tests
 *
 * These tests verify that the UnifiedInput automatically receives focus
 * when toggling between AI mode and Terminal mode from the StatusBar.
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
 * Get the Terminal mode toggle button (Terminal icon).
 */
function getTerminalModeButton(page: Page) {
  return page.getByRole("button", { name: "Switch to Terminal mode" });
}

/**
 * Get the Agent/AI mode toggle button (Bot icon).
 */
function getAgentModeButton(page: Page) {
  return page.getByRole("button", { name: "Switch to AI mode" });
}

function getNewTabButton(page: Page) {
  return page.getByRole("button", { name: "New tab" });
}

test.describe("Input Mode Focus", () => {
  test.beforeEach(async ({ page }) => {
    await waitForAppReady(page);
  });

  test("unified input should be focused on initial load", async ({ page }) => {
    const textarea = getInputTextarea(page);

    // The textarea should be visible
    await expect(textarea).toBeVisible();

    // The textarea should be focused
    await expect(textarea).toBeFocused();
  });

  test("input should auto-focus when toggling from terminal to agent mode", async ({ page }) => {
    // Start by switching to terminal mode
    const terminalButton = getTerminalModeButton(page);
    await terminalButton.click();

    // Verify we're in terminal mode by checking placeholder text
    const textarea = getInputTextarea(page);
    await expect(textarea).toHaveAttribute("placeholder", "Enter command...");

    // Remove focus from the textarea using blur
    await textarea.evaluate((el: HTMLTextAreaElement) => el.blur());
    await expect(textarea).not.toBeFocused();

    // Now switch to agent mode
    const agentButton = getAgentModeButton(page);
    await agentButton.click();

    // Verify we're in agent mode by checking placeholder text
    await expect(textarea).toHaveAttribute("placeholder", "Ask the AI...");

    // The textarea should be automatically focused
    await expect(textarea).toBeFocused();
  });

  test("input should auto-focus when toggling from agent to terminal mode", async ({ page }) => {
    // Start by switching to agent mode (default is terminal mode)
    const agentButton = getAgentModeButton(page);
    await agentButton.click();

    const textarea = getInputTextarea(page);

    // Verify we're in agent mode
    await expect(textarea).toHaveAttribute("placeholder", "Ask the AI...", { timeout: 3000 });

    // Remove focus from the textarea using blur
    await textarea.evaluate((el: HTMLTextAreaElement) => el.blur());
    await expect(textarea).not.toBeFocused();

    // Switch to terminal mode
    const terminalButton = getTerminalModeButton(page);
    await terminalButton.click();

    // Verify we're in terminal mode by checking placeholder text
    await expect(textarea).toHaveAttribute("placeholder", "Enter command...", { timeout: 3000 });

    // The textarea should be automatically focused
    await expect(textarea).toBeFocused();
  });

  test("input should auto-focus when toggling multiple times", async ({ page }) => {
    const textarea = getInputTextarea(page);
    const terminalButton = getTerminalModeButton(page);
    const agentButton = getAgentModeButton(page);

    // Start in terminal mode (default)
    await expect(textarea).toHaveAttribute("placeholder", "Enter command...");

    // Unfocus the input
    await textarea.evaluate((el: HTMLTextAreaElement) => el.blur());
    await expect(textarea).not.toBeFocused();

    // Toggle to agent mode
    await agentButton.click();
    await expect(textarea).toHaveAttribute("placeholder", "Ask the AI...", { timeout: 3000 });
    await expect(textarea).toBeFocused();

    // Unfocus again
    await textarea.evaluate((el: HTMLTextAreaElement) => el.blur());
    await expect(textarea).not.toBeFocused();

    // Toggle back to terminal
    await terminalButton.click();
    await expect(textarea).toHaveAttribute("placeholder", "Enter command...", { timeout: 3000 });
    await expect(textarea).toBeFocused();

    // Unfocus again
    await textarea.evaluate((el: HTMLTextAreaElement) => el.blur());
    await expect(textarea).not.toBeFocused();

    // Toggle to agent again
    await agentButton.click();
    await expect(textarea).toHaveAttribute("placeholder", "Ask the AI...", { timeout: 3000 });
    await expect(textarea).toBeFocused();
  });

  test("user can start typing immediately after toggling mode", async ({ page }) => {
    const textarea = getInputTextarea(page);
    const terminalButton = getTerminalModeButton(page);
    const agentButton = getAgentModeButton(page);

    // Start in terminal mode (default), unfocus
    await textarea.evaluate((el: HTMLTextAreaElement) => el.blur());

    // Toggle to agent mode
    await agentButton.click();

    // Verify we're in agent mode
    await expect(textarea).toHaveAttribute("placeholder", "Ask the AI...", { timeout: 3000 });

    // Type immediately without manually focusing
    await page.keyboard.type("Hello AI");

    // Verify the text was entered
    await expect(textarea).toHaveValue("Hello AI");

    // Clear the input
    await textarea.clear();

    // Unfocus
    await textarea.evaluate((el: HTMLTextAreaElement) => el.blur());

    // Toggle to terminal mode
    await terminalButton.click();

    // Verify we're in terminal mode
    await expect(textarea).toHaveAttribute("placeholder", "Enter command...", { timeout: 3000 });

    // Type immediately without manually focusing
    await page.keyboard.type("ls -la");

    // Verify the text was entered
    await expect(textarea).toHaveValue("ls -la");
  });

  test("input should auto-focus when creating new tab", async ({ page }) => {
    const newTabButton = getNewTabButton(page);

    // Helper to check if any UnifiedInput textarea is focused
    async function isUnifiedInputFocused(): Promise<boolean> {
      return page.evaluate(() => {
        const activeElement = document.activeElement;
        if (!activeElement || activeElement.tagName !== "TEXTAREA") return false;
        // Check it's not the xterm helper textarea
        return !activeElement.classList.contains("xterm-helper-textarea");
      });
    }

    // With only one tab, there's only one textarea
    const textarea = getInputTextarea(page);
    await expect(textarea).toBeFocused();

    // Create a second tab (becomes active)
    await newTabButton.click();

    const secondTab = page.getByRole("tab").nth(1);
    await expect(secondTab).toBeVisible();

    // After creating a new tab, a UnifiedInput textarea should be focused
    await page.waitForFunction(
      () => {
        const activeElement = document.activeElement;
        if (!activeElement || activeElement.tagName !== "TEXTAREA") return false;
        return !activeElement.classList.contains("xterm-helper-textarea");
      },
      { timeout: 3000 }
    );
    expect(await isUnifiedInputFocused()).toBe(true);

    // Verify we have 2 tabs
    const tabs = page.getByRole("tab");
    await expect(tabs).toHaveCount(2);
  });
});
