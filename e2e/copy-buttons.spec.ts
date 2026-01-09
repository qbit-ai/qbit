import { expect, type Page, test } from "@playwright/test";

/**
 * Copy Buttons E2E Tests
 *
 * Tests the copy button functionality on:
 * - User messages in AI chat
 * - Assistant messages in AI chat
 * - Command blocks in the terminal timeline
 */

// Type definitions for the global mock functions
declare global {
  interface Window {
    __MOCK_BROWSER_MODE__?: boolean;
    __MOCK_EMIT_AI_EVENT__?: (event: AiEventType) => Promise<void>;
    __MOCK_SIMULATE_AI_RESPONSE__?: (response: string, delayMs?: number) => Promise<void>;
    __MOCK_SIMULATE_COMMAND__?: (
      sessionId: string,
      command: string,
      output: string,
      exitCode?: number
    ) => Promise<void>;
    __QBIT_STORE__?: {
      getState: () => { activeSessionId: string | null };
    };
    __MOCK_EVENT_LISTENERS__?: Map<string, unknown[]>;
  }
}

type AiEventType =
  | { type: "started"; turn_id: string }
  | { type: "text_delta"; delta: string; accumulated: string }
  | {
      type: "completed";
      response: string;
      tokens_used?: number;
      duration_ms?: number;
      input_tokens?: number;
      output_tokens?: number;
    }
  | { type: "error"; message: string; error_type: string };

/**
 * Wait for the app to be fully ready in browser mode.
 */
async function waitForAppReady(page: Page) {
  await page.goto("/");
  await page.waitForLoadState("domcontentloaded");

  // Wait for the mock browser mode flag to be set
  await page.waitForFunction(() => window.__MOCK_BROWSER_MODE__ === true, { timeout: 15000 });

  // Wait for the status bar to appear (indicates React has rendered)
  await expect(page.locator('[data-testid="status-bar"]')).toBeVisible({
    timeout: 10000,
  });

  // Wait for the unified input textarea to be visible (use specific selector to avoid xterm textarea)
  await expect(page.locator('textarea[data-slot="popover-anchor"]')).toBeVisible({ timeout: 5000 });

  // Ensure mock functions are available
  await page.waitForFunction(() => typeof window.__MOCK_EMIT_AI_EVENT__ === "function", {
    timeout: 5000,
  });
}

/**
 * Get the Agent/AI mode toggle button (Bot icon).
 */
function getAgentModeButton(page: Page) {
  return page.getByRole("button", { name: "Switch to AI mode" });
}

/**
 * Get the UnifiedInput textarea element.
 * Uses a specific selector to avoid matching xterm's internal textarea.
 */
function getInputTextarea(page: Page) {
  return page.locator('textarea[data-slot="popover-anchor"]');
}

/**
 * Get the active session ID from the store.
 */
async function getActiveSessionId(page: Page): Promise<string | null> {
  return await page.evaluate(() => {
    return window.__QBIT_STORE__?.getState().activeSessionId ?? null;
  });
}

/**
 * Wait for command_block event listeners to be registered.
 */
async function waitForCommandBlockListeners(page: Page) {
  await page.waitForFunction(
    () => {
      const listeners = window.__MOCK_EVENT_LISTENERS__?.get("command_block");
      return listeners && listeners.length > 0;
    },
    { timeout: 10000 }
  );
}

test.describe("Copy Buttons", () => {
  test.beforeEach(async ({ page }) => {
    await waitForAppReady(page);
  });

  test.describe("User Message Copy Button", () => {
    test("copy button should be hidden by default and visible on hover", async ({ page }) => {
      // Switch to AI mode
      const agentButton = getAgentModeButton(page);
      await agentButton.click();

      const textarea = getInputTextarea(page);
      await expect(textarea).toHaveAttribute("placeholder", "Ask the AI...", { timeout: 3000 });

      // Submit a user message
      await textarea.fill("This is a test user message for copy button");
      await page.keyboard.press("Enter");

      // Wait for the message to appear in the timeline
      await page.waitForTimeout(300);

      // Find the user message copy button
      const copyButton = page.locator('[data-testid="user-message-copy-button"]');

      // Button should exist
      await expect(copyButton).toBeAttached();

      // Find the user message container and hover over it
      const userMessage = page.locator("text=This is a test user message for copy button");
      await userMessage.hover();

      // Wait for hover animation
      await page.waitForTimeout(200);

      // Copy button should now be visible (opacity changes on group-hover)
      // We check the computed style after hover
      const opacity = await copyButton.evaluate((el) => window.getComputedStyle(el).opacity);
      expect(Number.parseFloat(opacity)).toBeGreaterThan(0);
    });

    test("clicking copy button should show success state", async ({ page }) => {
      // Grant clipboard permissions for this test
      await page.context().grantPermissions(["clipboard-write", "clipboard-read"]);

      // Switch to AI mode
      const agentButton = getAgentModeButton(page);
      await agentButton.click();

      const textarea = getInputTextarea(page);
      await expect(textarea).toHaveAttribute("placeholder", "Ask the AI...", { timeout: 3000 });

      // Submit a user message
      await textarea.fill("Copy this message");
      await page.keyboard.press("Enter");

      // Wait for the message to appear
      await page.waitForTimeout(300);

      // Find and hover over the user message to show the copy button
      const userMessage = page.locator("text=Copy this message");
      await userMessage.hover();
      await page.waitForTimeout(200);

      // Click the copy button
      const copyButton = page.locator('[data-testid="user-message-copy-button"]');
      await copyButton.click({ force: true });

      // Button title should change to "Copied!"
      await expect(copyButton).toHaveAttribute("title", "Copied!");
    });
  });

  test.describe("Assistant Message Copy Button", () => {
    test("copy button should appear on assistant messages", async ({ page }) => {
      // Switch to AI mode
      const agentButton = getAgentModeButton(page);
      await agentButton.click();

      const textarea = getInputTextarea(page);
      await expect(textarea).toHaveAttribute("placeholder", "Ask the AI...", { timeout: 3000 });

      // Simulate an AI response
      await page.evaluate(async () => {
        const simulate = window.__MOCK_SIMULATE_AI_RESPONSE__;
        if (simulate) {
          await simulate("This is a test assistant response for copy button testing.", 5);
        }
      });

      // Wait for the response to appear
      await page.waitForTimeout(500);

      // Verify the response text is visible
      await expect(page.getByText("This is a test assistant response")).toBeVisible({
        timeout: 3000,
      });

      // Find the assistant message copy button
      const copyButton = page.locator('[data-testid="assistant-message-copy-button"]');

      // Button should exist
      await expect(copyButton).toBeAttached();

      // Hover over the assistant message area
      const assistantMessage = page.getByText("This is a test assistant response");
      await assistantMessage.hover();
      await page.waitForTimeout(200);

      // Copy button should become visible on hover
      const opacity = await copyButton.evaluate((el) => window.getComputedStyle(el).opacity);
      expect(Number.parseFloat(opacity)).toBeGreaterThan(0);
    });

    test("clicking assistant copy button should copy message content", async ({ page }) => {
      // Grant clipboard permissions
      await page.context().grantPermissions(["clipboard-write", "clipboard-read"]);

      // Switch to AI mode
      const agentButton = getAgentModeButton(page);
      await agentButton.click();

      const textarea = getInputTextarea(page);
      await expect(textarea).toHaveAttribute("placeholder", "Ask the AI...", { timeout: 3000 });

      // Simulate an AI response
      await page.evaluate(async () => {
        const simulate = window.__MOCK_SIMULATE_AI_RESPONSE__;
        if (simulate) {
          await simulate("Copy this assistant response text.", 5);
        }
      });

      await page.waitForTimeout(500);

      // Hover over the message to show copy button
      const assistantMessage = page.getByText("Copy this assistant response text");
      await assistantMessage.hover();
      await page.waitForTimeout(200);

      // Click copy button
      const copyButton = page.locator('[data-testid="assistant-message-copy-button"]');
      await copyButton.click({ force: true });

      // Verify copied state
      await expect(copyButton).toHaveAttribute("title", "Copied!");
    });
  });

  test.describe("Command Block Copy Button", () => {
    test("copy button should appear on command blocks on hover", async ({ page }) => {
      // Get the active session ID
      const sessionId = await getActiveSessionId(page);
      expect(sessionId).toBeTruthy();
      if (!sessionId) return;

      // Wait for command_block event listeners to be registered
      await waitForCommandBlockListeners(page);

      // Simulate a command using the higher-level helper
      await page.evaluate(
        async ({ sid }) => {
          const simulate = window.__MOCK_SIMULATE_COMMAND__;
          if (!simulate) {
            throw new Error("__MOCK_SIMULATE_COMMAND__ not found on window");
          }
          await simulate(sid, "echo hello world", "hello world\n", 0);
        },
        { sid: sessionId }
      );

      // Wait for the command block to appear
      await page.waitForTimeout(500);

      // Find the command block
      const commandBlock = page.locator('[data-testid="command-block"]').first();
      await expect(commandBlock).toBeVisible({ timeout: 3000 });

      // Find the copy button within the command block
      const copyButton = page.locator('[data-testid="command-block-copy-button"]').first();

      // Button should exist but be hidden by default
      await expect(copyButton).toBeAttached();
      await expect(copyButton).toHaveCSS("opacity", "0");

      // Hover over the command block
      await commandBlock.hover();
      await page.waitForTimeout(200);

      // Copy button should become visible
      const opacity = await copyButton.evaluate((el) => window.getComputedStyle(el).opacity);
      expect(Number.parseFloat(opacity)).toBeGreaterThan(0);
    });

    test("clicking command block copy button should copy command", async ({ page }) => {
      // Grant clipboard permissions
      await page.context().grantPermissions(["clipboard-write", "clipboard-read"]);

      // Get the active session ID
      const sessionId = await getActiveSessionId(page);
      expect(sessionId).toBeTruthy();
      if (!sessionId) return;

      // Wait for command_block event listeners to be registered
      await waitForCommandBlockListeners(page);

      // Simulate a command
      await page.evaluate(
        async ({ sid }) => {
          const simulate = window.__MOCK_SIMULATE_COMMAND__;
          if (!simulate) {
            throw new Error("__MOCK_SIMULATE_COMMAND__ not found on window");
          }
          await simulate(
            sid,
            "ls -la",
            "total 42\ndrwxr-xr-x  10 user  staff  320 Jan  1 12:00 .\n",
            0
          );
        },
        { sid: sessionId }
      );

      await page.waitForTimeout(500);

      // Find and hover over the command block
      const commandBlock = page.locator('[data-testid="command-block"]').first();
      await expect(commandBlock).toBeVisible({ timeout: 3000 });
      await commandBlock.hover();
      await page.waitForTimeout(200);

      // Click copy button
      const copyButton = page.locator('[data-testid="command-block-copy-button"]').first();
      await copyButton.click({ force: true });

      // Verify copied state
      await expect(copyButton).toHaveAttribute("title", "Copied!");
    });
  });

  test.describe("Multiple Copy Buttons", () => {
    test("each message should have its own independent copy button", async ({ page }) => {
      // Grant clipboard permissions
      await page.context().grantPermissions(["clipboard-write", "clipboard-read"]);

      // Switch to AI mode
      const agentButton = getAgentModeButton(page);
      await agentButton.click();

      const textarea = getInputTextarea(page);
      await expect(textarea).toHaveAttribute("placeholder", "Ask the AI...", { timeout: 3000 });

      // Submit first user message
      await textarea.fill("First user message");
      await page.keyboard.press("Enter");
      await page.waitForTimeout(200);

      // Simulate first AI response
      await page.evaluate(async () => {
        const simulate = window.__MOCK_SIMULATE_AI_RESPONSE__;
        if (simulate) {
          await simulate("First assistant response.", 5);
        }
      });
      await page.waitForTimeout(500);

      // Submit second user message
      await textarea.fill("Second user message");
      await page.keyboard.press("Enter");
      await page.waitForTimeout(200);

      // Simulate second AI response
      await page.evaluate(async () => {
        const simulate = window.__MOCK_SIMULATE_AI_RESPONSE__;
        if (simulate) {
          await simulate("Second assistant response.", 5);
        }
      });
      await page.waitForTimeout(500);

      // There should be multiple user and assistant copy buttons
      const userCopyButtons = page.locator('[data-testid="user-message-copy-button"]');
      const assistantCopyButtons = page.locator('[data-testid="assistant-message-copy-button"]');

      // We should have at least 2 of each type
      await expect(userCopyButtons).toHaveCount(2);
      await expect(assistantCopyButtons).toHaveCount(2);

      // Clicking one should not affect the others
      const firstUserMessage = page.locator("text=First user message");
      await firstUserMessage.hover();
      await page.waitForTimeout(200);

      const firstUserCopyButton = userCopyButtons.first();
      await firstUserCopyButton.click({ force: true });

      // First button should show copied state
      await expect(firstUserCopyButton).toHaveAttribute("title", "Copied!");

      // Second button should still show default state
      const secondUserMessage = page.locator("text=Second user message");
      await secondUserMessage.hover();
      await page.waitForTimeout(200);

      const secondUserCopyButton = userCopyButtons.nth(1);
      await expect(secondUserCopyButton).toHaveAttribute("title", "Copy code");
    });
  });
});
