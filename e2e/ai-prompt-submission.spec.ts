import { expect, type Page, test } from "@playwright/test";

/**
 * AI Prompt Submission E2E Tests
 *
 * These tests verify that submitting prompts to the AI agent works correctly
 * and catches parameter validation errors that would occur with the real Tauri backend.
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
 * Get the Agent/AI mode toggle button (Bot icon).
 */
function getAgentModeButton(page: Page) {
  return page.getByRole("button", { name: "Switch to AI mode" });
}

/**
 * Collect console errors during a test.
 */
function setupErrorCollection(page: Page): string[] {
  const errors: string[] = [];

  page.on("console", (msg) => {
    if (msg.type() === "error") {
      errors.push(msg.text());
    }
  });

  page.on("pageerror", (error) => {
    errors.push(`Page error: ${error.message}`);
  });

  return errors;
}

test.describe("AI Prompt Submission", () => {
  test.beforeEach(async ({ page }) => {
    await waitForAppReady(page);
  });

  test("should submit AI prompt without parameter validation errors", async ({ page }) => {
    const errors = setupErrorCollection(page);

    // Switch to AI mode
    const agentButton = getAgentModeButton(page);
    await agentButton.click();

    const textarea = getInputTextarea(page);

    // Verify we're in agent mode
    await expect(textarea).toHaveAttribute("placeholder", "Ask the AI...", { timeout: 3000 });

    // Type a prompt
    await textarea.fill("Hello, this is a test prompt");

    // Submit the prompt (press Enter)
    await page.keyboard.press("Enter");

    // Wait a moment for any errors to surface
    await page.waitForTimeout(500);

    // Check for parameter validation errors (the exact error format from our mock)
    const parameterErrors = errors.filter(
      (e) => e.includes("invalid args") || e.includes("missing required key")
    );

    expect(parameterErrors).toHaveLength(0);
  });

  test("should handle AI session initialization without errors", async ({ page }) => {
    const errors = setupErrorCollection(page);

    // The app should initialize AI session on startup/tab creation
    // Wait for any initialization to complete
    await page.waitForTimeout(1000);

    // Check for any initialization-related errors
    const initErrors = errors.filter(
      (e) =>
        e.includes("init_ai_session") ||
        e.includes("invalid args") ||
        e.includes("missing required key")
    );

    expect(initErrors).toHaveLength(0);
  });

  test("should log mock IPC calls with correct parameters", async ({ page }) => {
    const mockIpcLogs: string[] = [];

    // Capture mock IPC logs
    page.on("console", (msg) => {
      const text = msg.text();
      if (text.includes("[Mock IPC]")) {
        mockIpcLogs.push(text);
      }
    });

    const errors = setupErrorCollection(page);

    // Switch to AI mode
    const agentButton = getAgentModeButton(page);
    await agentButton.click();

    const textarea = getInputTextarea(page);
    await expect(textarea).toHaveAttribute("placeholder", "Ask the AI...", { timeout: 3000 });

    // Submit a prompt
    await textarea.fill("Test prompt for IPC logging");
    await page.keyboard.press("Enter");

    // Wait for the IPC call to be logged
    await page.waitForTimeout(500);

    // Verify the send_ai_prompt_session call was made
    const sendPromptLog = mockIpcLogs.find((log) => log.includes("send_ai_prompt_session"));
    expect(sendPromptLog).toBeDefined();

    // Verify no parameter validation errors occurred
    const parameterErrors = errors.filter(
      (e) => e.includes("invalid args") || e.includes("missing required key")
    );
    expect(parameterErrors).toHaveLength(0);
  });

  test("should not show AI error notification on prompt submission", async ({ page }) => {
    // Switch to AI mode
    const agentButton = getAgentModeButton(page);
    await agentButton.click();

    const textarea = getInputTextarea(page);
    await expect(textarea).toHaveAttribute("placeholder", "Ask the AI...", { timeout: 3000 });

    // Type and submit a prompt
    await textarea.fill("Test prompt for error checking");
    await page.keyboard.press("Enter");

    // Wait for potential error notification to appear
    await page.waitForTimeout(1000);

    // Check that no error notification appeared
    // The notification would contain "Agent error" based on UnifiedInput.tsx line 183
    const errorNotification = page.locator('text="Agent error"');
    await expect(errorNotification).not.toBeVisible();
  });

  test("mock should validate sessionId parameter correctly", async ({ page }) => {
    // This test verifies the mock's parameter validation is working
    // by checking that valid calls succeed (no errors)

    const errors = setupErrorCollection(page);

    // Switch to AI mode and submit a prompt (which calls send_ai_prompt_session)
    const agentButton = getAgentModeButton(page);
    await agentButton.click();

    const textarea = getInputTextarea(page);
    await expect(textarea).toHaveAttribute("placeholder", "Ask the AI...", { timeout: 3000 });

    await textarea.fill("Testing parameter validation");
    await page.keyboard.press("Enter");

    await page.waitForTimeout(500);

    // If the mock's validation caught a missing sessionId, it would log an error
    // containing "missing required key sessionId"
    const sessionIdErrors = errors.filter((e) => e.includes("sessionId"));

    // With our fix (using camelCase sessionId), there should be no errors
    expect(sessionIdErrors).toHaveLength(0);
  });
});
