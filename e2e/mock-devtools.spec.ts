import { expect, type Page, test } from "@playwright/test";

/**
 * MockDevTools E2E Tests
 *
 * These tests verify that mock events properly update the UI,
 * not just that events are dispatched. Each test triggers an action
 * and verifies that the expected content appears on screen.
 */

/**
 * Wait for the app to be fully ready in browser mode.
 * This includes waiting for mocks to be initialized and React to render.
 */
async function waitForAppReady(page: Page) {
  await page.goto("/");
  await page.waitForLoadState("domcontentloaded");

  // Wait for the mock browser mode flag to be set (mocks initialized)
  await page.waitForFunction(
    () => (window as unknown as { __MOCK_BROWSER_MODE__?: boolean }).__MOCK_BROWSER_MODE__ === true,
    { timeout: 15000 }
  );

  // Wait for the MockDevTools toggle button to be visible (React rendered)
  const toggleButton = page.locator('button[title="Toggle Mock Dev Tools"]');
  await expect(toggleButton).toBeVisible({ timeout: 10000 });
}

test.describe("MockDevTools - Preset UI Verification", () => {
  test.beforeEach(async ({ page }) => {
    // Navigate and wait for app to fully load
    await waitForAppReady(page);
  });

  test("Fresh Start preset completes without error", async ({ page }) => {
    // Open MockDevTools
    await page.locator('button[title="Toggle Mock Dev Tools"]').click();
    await expect(page.locator("text=Mock Dev Tools")).toBeVisible();

    // Click Fresh Start preset
    await page.locator("text=Fresh Start").click();

    // Note: Fresh Start only emits terminal_output without a command_start event,
    // so the output doesn't appear in the timeline (by design - the app only shows
    // output that is part of command blocks). Verify the preset completes by
    // checking the action log shows completion.
    // Assertion auto-retries until element appears or timeout
    await expect(page.locator("text=Fresh start complete")).toBeVisible({ timeout: 10000 });
  });

  test("Active Conversation preset displays terminal output and AI response", async ({ page }) => {
    // Open MockDevTools
    await page.locator('button[title="Toggle Mock Dev Tools"]').click();

    // Click Active Conversation preset
    await page.locator("text=Active Conversation").click();

    // Verify terminal output from "cat src/main.rs" command is visible
    // Assertion auto-retries until streaming completes
    await expect(page.locator("text=Hello, world!")).toBeVisible({ timeout: 15000 });

    // Verify AI response text appears
    await expect(page.locator("text=basic Rust project")).toBeVisible({ timeout: 10000 });
  });

  test("Tool Execution preset displays tool request and result", async ({ page }) => {
    // Open MockDevTools
    await page.locator('button[title="Toggle Mock Dev Tools"]').click();

    // Click Tool Execution preset
    await page.locator("text=Tool Execution").click();

    // Verify AI text appears - assertion auto-retries until preset completes
    await expect(page.locator("text=read the configuration file")).toBeVisible({ timeout: 15000 });

    // Verify tool request shows (tool name in UI)
    await expect(page.locator("text=read_file")).toBeVisible({ timeout: 5000 });

    // Verify tool result content is shown
    await expect(page.locator("text=Rust 2021 edition")).toBeVisible({ timeout: 5000 });
  });

  test("Error State preset displays error message", async ({ page }) => {
    // Open MockDevTools
    await page.locator('button[title="Toggle Mock Dev Tools"]').click();

    // Click Error State preset
    await page.locator("text=Error State").click();

    // Verify error message appears - assertion auto-retries
    await expect(page.locator("text=Rate limit exceeded")).toBeVisible({ timeout: 10000 });
  });

  test("Command History preset displays multiple command outputs", async ({ page }) => {
    // Open MockDevTools
    await page.locator('button[title="Toggle Mock Dev Tools"]').click();

    // Click Command History preset
    await page.locator("text=Command History").click();

    // Verify git status output - wait for first command to complete
    await expect(page.locator("text=On branch main")).toBeVisible({ timeout: 15000 });

    // Verify cargo build output
    await expect(page.locator("text=Compiling my-app")).toBeVisible({ timeout: 10000 });

    // Verify cargo test output (last command, needs more time)
    await expect(page.locator("text=test result: ok. 3 passed")).toBeVisible({ timeout: 10000 });
  });

  test("Build Failure preset displays compiler error and AI help", async ({ page }) => {
    // Open MockDevTools
    await page.locator('button[title="Toggle Mock Dev Tools"]').click();

    // Click Build Failure preset
    await page.locator("text=Build Failure").click();

    // Verify compiler error is shown - assertion auto-retries until preset completes
    await expect(page.locator("text=borrow of moved value")).toBeVisible({ timeout: 15000 });

    // Verify AI help response appears
    await expect(page.locator("text=borrow checker error")).toBeVisible({ timeout: 10000 });
  });

  test("Code Review preset displays code and review comments", async ({ page }) => {
    // Open MockDevTools
    await page.locator('button[title="Toggle Mock Dev Tools"]').click();

    // Click Code Review preset
    await page.locator("text=Code Review").click();

    // Verify the code being reviewed is shown - assertion auto-retries until preset completes
    await expect(page.locator("text=cat src/handlers.rs").first()).toBeVisible({ timeout: 15000 });

    // Verify review comments appear (check for a specific review point)
    await expect(page.locator("text=anti-pattern")).toBeVisible({ timeout: 10000 });
  });

  test("Long Output preset displays extensive test output", async ({ page }) => {
    // Open MockDevTools
    await page.locator('button[title="Toggle Mock Dev Tools"]').click();

    // Click Long Output preset
    await page.locator("text=Long Output").click();

    // Verify test output header appears - assertion auto-retries
    await expect(page.locator("text=running 50 tests")).toBeVisible({ timeout: 10000 });

    // Verify doc test output also appears
    await expect(page.locator("text=Doc-tests my-app")).toBeVisible({ timeout: 5000 });
  });
});

test.describe("MockDevTools - Terminal Tab UI Verification", () => {
  test.beforeEach(async ({ page }) => {
    await waitForAppReady(page);

    // Open MockDevTools and switch to Terminal tab
    await page.locator('button[title="Toggle Mock Dev Tools"]').click();
    await expect(page.locator("text=Mock Dev Tools")).toBeVisible();
    await page.getByRole("button", { name: "Terminal", exact: true }).click();
  });

  test("Emit Output button displays terminal output in timeline", async ({ page }) => {
    // Set custom output text in the MockDevTools panel
    const customOutput = "Custom terminal output for testing\n";
    const panel = page.locator('[data-testid="mock-devtools-panel"]');
    await panel.locator("textarea").first().fill(customOutput);

    // Click Emit Output
    await panel.locator("button:has-text('Emit Output')").click();

    // Verify the output appears in the UI - assertion auto-retries
    await expect(page.locator("text=Custom terminal output for testing")).toBeVisible({
      timeout: 10000,
    });
  });

  test("Emit Command Block displays command with output in timeline", async ({ page }) => {
    // Set custom command and output within the MockDevTools panel
    const panel = page.locator('[data-testid="mock-devtools-panel"]');
    const commandInput = panel.locator('input[type="text"]').nth(1); // Command input
    await commandInput.fill("echo 'test command'");

    const outputTextarea = panel.locator("textarea").last();
    await outputTextarea.fill("Command output result");

    // Click Emit Command Block
    await panel.locator("button:has-text('Emit Command Block')").click();

    // Verify command appears in timeline - assertion auto-retries
    await expect(page.locator("text=echo 'test command'").first()).toBeVisible({ timeout: 10000 });

    // Verify output appears in timeline
    await expect(page.locator("text=Command output result").first()).toBeVisible({ timeout: 5000 });
  });
});

test.describe("MockDevTools - AI Tab UI Verification", () => {
  test.beforeEach(async ({ page }) => {
    await waitForAppReady(page);

    // Open MockDevTools and switch to AI tab
    await page.locator('button[title="Toggle Mock Dev Tools"]').click();
    await expect(page.locator("text=Mock Dev Tools")).toBeVisible();
    await page.getByRole("button", { name: "AI", exact: true }).click();
  });

  test("Simulate Response displays streamed AI text in timeline", async ({ page }) => {
    // Set custom AI response within the MockDevTools panel
    const panel = page.locator('[data-testid="mock-devtools-panel"]');
    const customResponse = "This is a custom AI response for testing the streaming feature.";
    await panel.locator("textarea").first().fill(customResponse);

    // Set a fast stream delay
    await panel.locator('input[type="number"]').first().fill("10");

    // Click Simulate Response
    await panel.locator("button:has-text('Simulate Response')").click();

    // Verify the AI response text appears in the UI - assertion auto-retries during streaming
    await expect(page.locator("text=custom AI response for testing")).toBeVisible({
      timeout: 15000,
    });
  });

  test("Emit Tool Request displays tool card in timeline", async ({ page }) => {
    // Set tool name and args within the MockDevTools panel
    const panel = page.locator('[data-testid="mock-devtools-panel"]');
    const toolNameInput = panel.locator('input[type="text"]').first();
    await toolNameInput.fill("write_file");

    const toolArgsTextarea = panel.locator("textarea").last();
    await toolArgsTextarea.fill('{"path": "/test/file.txt", "content": "hello"}');

    // Click Emit Tool Request
    await panel.locator("button:has-text('Emit Tool Request')").click();

    // Verify tool request card appears with tool name - assertion auto-retries
    await expect(page.locator("text=write_file").first()).toBeVisible({ timeout: 10000 });
  });

  test("Emit Error displays error message in timeline", async ({ page }) => {
    // Click Emit Error
    await page.locator("button:has-text('Emit Error')").click();

    // Verify error message appears - assertion auto-retries
    await expect(page.locator("text=Mock error for testing")).toBeVisible({ timeout: 10000 });
  });
});

test.describe("MockDevTools - Session Tab Functionality", () => {
  test.beforeEach(async ({ page }) => {
    await waitForAppReady(page);

    // Open MockDevTools and switch to Session tab
    await page.locator('button[title="Toggle Mock Dev Tools"]').click();
    await expect(page.locator("text=Mock Dev Tools")).toBeVisible();
    await page.locator("button:has-text('Session')").click();
  });

  test("New Session ID button generates a new session ID", async ({ page }) => {
    // Get the initial session ID
    const sessionInput = page.locator('input[type="text"]').first();
    const initialValue = await sessionInput.inputValue();

    // Click New Session ID button
    await page.locator("button:has-text('New Session ID')").click();

    // Wait for session ID to change using polling assertion
    await expect
      .poll(async () => sessionInput.inputValue(), { timeout: 5000 })
      .not.toBe(initialValue);

    // Verify the new session ID format
    const newValue = await sessionInput.inputValue();
    expect(newValue).toContain("mock-session-");
  });

  test("Session ID input can be changed manually", async ({ page }) => {
    const sessionInput = page.locator('input[type="text"]').first();
    const customSessionId = "custom-session-123";

    // Clear and fill with custom session ID
    await sessionInput.fill(customSessionId);

    // Verify the value is set
    const value = await sessionInput.inputValue();
    expect(value).toBe(customSessionId);
  });
});

test.describe("MockDevTools - Combined Preset Workflow", () => {
  test("Run Active Conversation, Tool Execution, and Error State in sequence", async ({ page }) => {
    // 1. Load the page fresh
    await waitForAppReady(page);

    // 2. Open MockDevTools panel
    await page.locator('button[title="Toggle Mock Dev Tools"]').click();
    await expect(page.locator("text=Mock Dev Tools")).toBeVisible();

    // 3. Run Active Conversation preset
    await page.locator("text=Active Conversation").click();

    // Verify Active Conversation content in timeline - assertion auto-retries
    await expect(page.locator("text=Hello, world!")).toBeVisible({ timeout: 15000 });
    await expect(page.locator("text=basic Rust project")).toBeVisible({ timeout: 10000 });

    // 4. Run Tool Execution preset
    await page.locator("text=Tool Execution").click();

    // Verify Tool Execution content in timeline
    await expect(page.locator("text=read the configuration file")).toBeVisible({ timeout: 15000 });
    await expect(page.locator("text=read_file")).toBeVisible({ timeout: 5000 });

    // 5. Run Error State preset
    await page.locator("text=Error State").click();

    // Verify Error State content in timeline
    await expect(page.locator("text=Rate limit exceeded")).toBeVisible({ timeout: 10000 });

    // 6. Verify all previous items are still visible (cumulative timeline)
    // Active Conversation items should still be visible
    await expect(page.locator("text=Hello, world!")).toBeVisible();
    await expect(page.locator("text=basic Rust project")).toBeVisible();

    // Tool Execution items should still be visible
    await expect(page.locator("text=read_file")).toBeVisible();

    // Close the MockDevTools panel
    await page.locator('button[title="Toggle Mock Dev Tools"]').click();
    await expect(page.locator("text=Mock Dev Tools")).not.toBeVisible();
  });
});

test.describe("MockDevTools - Panel Interaction", () => {
  test("Toggle button opens and closes the panel", async ({ page }) => {
    await waitForAppReady(page);

    // Panel should be closed initially
    await expect(page.locator("text=Mock Dev Tools")).not.toBeVisible();

    // Click toggle button to open
    await page.locator('button[title="Toggle Mock Dev Tools"]').click();
    await expect(page.locator("text=Mock Dev Tools")).toBeVisible();

    // Click toggle button to close
    await page.locator('button[title="Toggle Mock Dev Tools"]').click();
    await expect(page.locator("text=Mock Dev Tools")).not.toBeVisible();
  });

  test("Tab navigation works correctly", async ({ page }) => {
    await waitForAppReady(page);

    // Open panel
    await page.locator('button[title="Toggle Mock Dev Tools"]').click();

    // Verify Presets tab is active by default (shows Scenarios section)
    await expect(page.locator("text=Scenarios")).toBeVisible();

    // Switch to Terminal tab
    await page.getByRole("button", { name: "Terminal", exact: true }).click();
    await expect(page.locator("text=Terminal Output")).toBeVisible();

    // Switch to AI tab
    await page.getByRole("button", { name: "AI", exact: true }).click();
    await expect(page.locator("text=Streaming Response")).toBeVisible();

    // Switch to Session tab
    await page.getByRole("button", { name: "Session", exact: true }).click();
    await expect(page.locator("text=Session Management")).toBeVisible();

    // Switch back to Presets
    await page.getByRole("button", { name: "Presets", exact: true }).click();
    await expect(page.locator("text=Scenarios")).toBeVisible();
  });

  test("All preset cards are visible in the Presets tab", async ({ page }) => {
    await waitForAppReady(page);

    // Open panel
    await page.locator('button[title="Toggle Mock Dev Tools"]').click();

    // Verify all 8 presets are listed
    await expect(page.locator("text=Fresh Start")).toBeVisible();
    await expect(page.locator("text=Active Conversation")).toBeVisible();
    await expect(page.locator("text=Tool Execution")).toBeVisible();
    await expect(page.locator("text=Error State")).toBeVisible();
    await expect(page.locator("text=Command History")).toBeVisible();
    await expect(page.locator("text=Build Failure")).toBeVisible();
    await expect(page.locator("text=Code Review")).toBeVisible();
    await expect(page.locator("text=Long Output")).toBeVisible();
  });
});
