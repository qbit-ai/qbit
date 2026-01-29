import { expect, type Page, test } from "@playwright/test";
import { waitForAppReady as waitForAppReadyBase } from "./helpers/app";

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
  await waitForAppReadyBase(page);

  // Wait for the unified input textarea to be visible in the active tab
  // Use :visible to find the textarea in the currently active tab
  await expect(page.locator('[data-testid="unified-input"]:visible').first()).toBeVisible({
    timeout: 10000,
  });
}

/**
 * Get the UnifiedInput textarea element.
 * Uses :visible to find the textarea in the currently active tab.
 */
function getInputTextarea(page: Page) {
  return page.locator('[data-testid="unified-input"]:visible').first();
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
    await expect(textarea).toHaveAttribute("data-mode", "terminal");

    // Remove focus from the textarea using blur
    await textarea.evaluate((el: HTMLTextAreaElement) => el.blur());
    await expect(textarea).not.toBeFocused();

    // Now switch to agent mode
    const agentButton = getAgentModeButton(page);
    await agentButton.click();

    // Verify we're in agent mode by checking placeholder text
    await expect(textarea).toHaveAttribute("data-mode", "agent");

    // The textarea should be automatically focused
    await expect(textarea).toBeFocused();
  });

  test("input should auto-focus when toggling from agent to terminal mode", async ({ page }) => {
    // Start by switching to agent mode (default is terminal mode)
    const agentButton = getAgentModeButton(page);
    await agentButton.click();

    const textarea = getInputTextarea(page);

    // Verify we're in agent mode
    await expect(textarea).toHaveAttribute("data-mode", "agent", { timeout: 3000 });

    // Remove focus from the textarea using blur
    await textarea.evaluate((el: HTMLTextAreaElement) => el.blur());
    await expect(textarea).not.toBeFocused();

    // Switch to terminal mode
    const terminalButton = getTerminalModeButton(page);
    await terminalButton.click();

    // Verify we're in terminal mode by checking placeholder text
    await expect(textarea).toHaveAttribute("data-mode", "terminal", { timeout: 3000 });

    // The textarea should be automatically focused
    await expect(textarea).toBeFocused();
  });

  test("input should auto-focus when toggling multiple times", async ({ page }) => {
    const textarea = getInputTextarea(page);
    const terminalButton = getTerminalModeButton(page);
    const agentButton = getAgentModeButton(page);

    // Start in terminal mode (default)
    await expect(textarea).toHaveAttribute("data-mode", "terminal");

    // Unfocus the input
    await textarea.evaluate((el: HTMLTextAreaElement) => el.blur());
    await expect(textarea).not.toBeFocused();

    // Toggle to agent mode
    await agentButton.click();
    await expect(textarea).toHaveAttribute("data-mode", "agent", { timeout: 3000 });
    await expect(textarea).toBeFocused();

    // Unfocus again
    await textarea.evaluate((el: HTMLTextAreaElement) => el.blur());
    await expect(textarea).not.toBeFocused();

    // Toggle back to terminal
    await terminalButton.click();
    await expect(textarea).toHaveAttribute("data-mode", "terminal", { timeout: 3000 });
    await expect(textarea).toBeFocused();

    // Unfocus again
    await textarea.evaluate((el: HTMLTextAreaElement) => el.blur());
    await expect(textarea).not.toBeFocused();

    // Toggle to agent again
    await agentButton.click();
    await expect(textarea).toHaveAttribute("data-mode", "agent", { timeout: 3000 });
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
    await expect(textarea).toHaveAttribute("data-mode", "agent", { timeout: 3000 });

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
    await expect(textarea).toHaveAttribute("data-mode", "terminal", { timeout: 3000 });

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

    // Initially we have Home + Terminal tabs, and Terminal's textarea is focused
    const textarea = getInputTextarea(page);
    await expect(textarea).toBeFocused();

    // Create a third tab (becomes active)
    await newTabButton.click();

    const thirdTab = page.getByRole("tab").nth(2);
    await expect(thirdTab).toBeVisible();

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

    // Verify we have 3 tabs (Home + Terminal + new tab)
    const tabs = page.getByRole("tab");
    await expect(tabs).toHaveCount(3);
  });
});
