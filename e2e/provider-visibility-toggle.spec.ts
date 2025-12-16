import { expect, type Page, test } from "@playwright/test";

/**
 * Provider Visibility Toggle E2E Tests
 *
 * These tests verify that the provider visibility toggle feature works correctly:
 * - Provider toggles are visible in settings
 * - Toggling visibility updates the model selector in the status bar
 * - When all providers are disabled, an appropriate message is shown
 * - Settings changes are persisted
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
  await expect(page.locator('[class*="StatusBar"]').or(page.locator("text=Terminal"))).toBeVisible({
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
  // Wait for settings to load (the loading spinner should disappear)
  await expect(page.locator("text=AI & Providers")).toBeVisible({ timeout: 5000 });
}

/**
 * Close the settings dialog by clicking Cancel.
 */
async function closeSettings(page: Page) {
  await page.locator("button:has-text('Cancel')").click();
  // Wait for dialog to close (use role='dialog' to avoid matching multiple h2 elements)
  await expect(page.getByRole("dialog")).not.toBeVisible({ timeout: 3000 });
}

/**
 * Save settings by clicking Save Changes button.
 */
async function saveSettings(page: Page) {
  await page.locator("button:has-text('Save Changes')").click();
  // Wait for dialog to close after save (use role='dialog' to avoid matching multiple h2 elements)
  await expect(page.getByRole("dialog")).not.toBeVisible({ timeout: 5000 });
}

/**
 * Switch to the AI mode in the status bar.
 */
async function switchToAgentMode(page: Page) {
  // Find and click the Bot icon button (agent mode toggle)
  const agentModeButton = page.locator("button").filter({ has: page.locator("svg.lucide-bot") });
  await agentModeButton.click();
}

test.describe("Provider Visibility Toggle - Settings UI", () => {
  test.beforeEach(async ({ page }) => {
    await waitForAppReady(page);
  });

  test("settings dialog shows AI & Providers section with provider visibility toggles", async ({
    page,
  }) => {
    // Open settings
    await openSettings(page);

    // AI & Providers should be visible and selected by default
    await expect(page.locator("text=AI & Providers")).toBeVisible();
    await expect(page.locator("text=Default Provider")).toBeVisible();

    // Close settings
    await closeSettings(page);
  });

  test("Vertex AI provider settings show visibility toggle when selected", async ({ page }) => {
    // Open settings
    await openSettings(page);

    // Select Vertex AI as the default provider
    await page.locator("#ai-default-provider").selectOption("vertex_ai");

    // Wait for the Vertex AI configuration section to appear
    await expect(page.locator("text=Vertex AI Configuration")).toBeVisible({ timeout: 3000 });

    // Check that the "Show in model selector" toggle is present
    await expect(page.locator("text=Show in model selector").first()).toBeVisible();

    // The switch should be visible
    const vertexToggle = page.locator("#vertex-show-in-selector");
    await expect(vertexToggle).toBeVisible();

    // Close settings
    await closeSettings(page);
  });

  test("OpenRouter provider settings show visibility toggle when selected", async ({ page }) => {
    // Open settings
    await openSettings(page);

    // Select OpenRouter as the default provider
    await page.locator("#ai-default-provider").selectOption("openrouter");

    // Wait for the OpenRouter configuration section to appear
    await expect(page.locator("text=OpenRouter Configuration")).toBeVisible({ timeout: 3000 });

    // Check that the "Show in model selector" toggle is present
    await expect(page.locator("text=Show in model selector").first()).toBeVisible();

    // The switch should be visible
    const openRouterToggle = page.locator("#openrouter-show-in-selector");
    await expect(openRouterToggle).toBeVisible();

    // Close settings
    await closeSettings(page);
  });

  test("provider visibility toggle can be clicked and saves state", async ({ page }) => {
    // Open settings
    await openSettings(page);

    // Select Vertex AI as the default provider
    await page.locator("#ai-default-provider").selectOption("vertex_ai");

    // Wait for the Vertex AI configuration section
    await expect(page.locator("text=Vertex AI Configuration")).toBeVisible({ timeout: 3000 });

    // Get the toggle and check its initial state (should be checked/true by default in mocks)
    const vertexToggle = page.locator("#vertex-show-in-selector");
    const initialState = await vertexToggle.isChecked();
    expect(initialState).toBe(true);

    // Click to toggle off
    await vertexToggle.click();

    // Verify it's now unchecked
    await expect(vertexToggle).not.toBeChecked();

    // Save settings
    await saveSettings(page);

    // Re-open settings to verify persistence
    await openSettings(page);
    await page.locator("#ai-default-provider").selectOption("vertex_ai");
    await expect(page.locator("text=Vertex AI Configuration")).toBeVisible({ timeout: 3000 });

    // Verify the toggle state persisted
    const persistedToggle = page.locator("#vertex-show-in-selector");
    await expect(persistedToggle).not.toBeChecked();

    // Close settings
    await closeSettings(page);
  });
});

test.describe("Provider Visibility Toggle - Model Selector", () => {
  test.beforeEach(async ({ page }) => {
    await waitForAppReady(page);
  });

  test("model selector shows providers based on visibility settings", async ({ page }) => {
    // Switch to agent mode to see the model selector
    await switchToAgentMode(page);

    // Wait for the model selector button to be visible (with model name)
    // In mock mode with both providers visible, we should see a model selector
    const modelSelector = page.locator("button").filter({ hasText: /Claude|Devstral|GPT/ });
    await expect(modelSelector.first()).toBeVisible({ timeout: 5000 });

    // Click the model selector to open dropdown
    await modelSelector.first().click();

    // Both Vertex AI and OpenRouter sections should be visible
    await expect(page.locator("text=Vertex AI")).toBeVisible();
    await expect(page.locator("text=OpenRouter")).toBeVisible();

    // Click outside to close dropdown
    await page.keyboard.press("Escape");
  });

  test("disabling a provider hides it from the model selector", async ({ page }) => {
    // First, open settings and disable OpenRouter
    await openSettings(page);
    await page.locator("#ai-default-provider").selectOption("openrouter");
    await expect(page.locator("text=OpenRouter Configuration")).toBeVisible({ timeout: 3000 });

    // Toggle off OpenRouter visibility
    const openRouterToggle = page.locator("#openrouter-show-in-selector");
    await openRouterToggle.click();
    await expect(openRouterToggle).not.toBeChecked();

    // Save settings
    await saveSettings(page);

    // Switch to agent mode
    await switchToAgentMode(page);

    // Wait for the model selector
    const modelSelector = page.locator("button").filter({ hasText: /Claude|Devstral|GPT/ });
    await expect(modelSelector.first()).toBeVisible({ timeout: 5000 });

    // Click the model selector to open dropdown
    await modelSelector.first().click();

    // Vertex AI should be visible, but OpenRouter should NOT be visible
    await expect(page.locator("[role='menu'] >> text=Vertex AI")).toBeVisible();
    await expect(page.locator("[role='menu'] >> text=OpenRouter")).not.toBeVisible();

    // Close dropdown
    await page.keyboard.press("Escape");
  });

  test("disabling all providers shows 'Enable a provider' message", async ({ page }) => {
    // First, disable Vertex AI
    await openSettings(page);
    await page.locator("#ai-default-provider").selectOption("vertex_ai");
    await expect(page.locator("text=Vertex AI Configuration")).toBeVisible({ timeout: 3000 });
    await page.locator("#vertex-show-in-selector").click();
    await saveSettings(page);

    // Then, disable OpenRouter
    await openSettings(page);
    await page.locator("#ai-default-provider").selectOption("openrouter");
    await expect(page.locator("text=OpenRouter Configuration")).toBeVisible({ timeout: 3000 });
    await page.locator("#openrouter-show-in-selector").click();
    await saveSettings(page);

    // Switch to agent mode
    await switchToAgentMode(page);

    // Should see the "Enable a provider in settings" message instead of model selector
    await expect(page.locator("text=Enable a provider in settings")).toBeVisible({ timeout: 5000 });
  });

  test("re-enabling a provider makes it visible in model selector again", async ({ page }) => {
    // First, disable both providers
    await openSettings(page);
    await page.locator("#ai-default-provider").selectOption("vertex_ai");
    await expect(page.locator("text=Vertex AI Configuration")).toBeVisible({ timeout: 3000 });
    await page.locator("#vertex-show-in-selector").click();
    await saveSettings(page);

    await openSettings(page);
    await page.locator("#ai-default-provider").selectOption("openrouter");
    await expect(page.locator("text=OpenRouter Configuration")).toBeVisible({ timeout: 3000 });
    await page.locator("#openrouter-show-in-selector").click();
    await saveSettings(page);

    // Verify message is shown
    await switchToAgentMode(page);
    await expect(page.locator("text=Enable a provider in settings")).toBeVisible({ timeout: 5000 });

    // Now re-enable Vertex AI
    await openSettings(page);
    await page.locator("#ai-default-provider").selectOption("vertex_ai");
    await expect(page.locator("text=Vertex AI Configuration")).toBeVisible({ timeout: 3000 });
    await page.locator("#vertex-show-in-selector").click(); // Toggle back on
    await saveSettings(page);

    // The model selector should now be visible again
    const modelSelector = page.locator("button").filter({ hasText: /Claude/ });
    await expect(modelSelector.first()).toBeVisible({ timeout: 5000 });
  });
});

test.describe("Provider Visibility Toggle - Settings Persistence", () => {
  test.beforeEach(async ({ page }) => {
    await waitForAppReady(page);
  });

  test("settings changes trigger settings-updated event", async ({ page }) => {
    // Set up event listener to capture settings-updated events
    const settingsUpdatedPromise = page.evaluate(() => {
      return new Promise<boolean>((resolve) => {
        const handler = () => {
          window.removeEventListener("settings-updated", handler);
          resolve(true);
        };
        window.addEventListener("settings-updated", handler);
        // Timeout after 10 seconds
        setTimeout(() => resolve(false), 10000);
      });
    });

    // Open settings and make a change
    await openSettings(page);
    await page.locator("#ai-default-provider").selectOption("vertex_ai");
    await expect(page.locator("text=Vertex AI Configuration")).toBeVisible({ timeout: 3000 });

    // Toggle the visibility
    await page.locator("#vertex-show-in-selector").click();

    // Save settings (this should trigger the settings-updated event)
    await saveSettings(page);

    // Verify the event was triggered
    const eventTriggered = await settingsUpdatedPromise;
    expect(eventTriggered).toBe(true);
  });

  test("cancel button discards changes", async ({ page }) => {
    // Open settings and verify initial state
    await openSettings(page);
    await page.locator("#ai-default-provider").selectOption("vertex_ai");
    await expect(page.locator("text=Vertex AI Configuration")).toBeVisible({ timeout: 3000 });

    const vertexToggle = page.locator("#vertex-show-in-selector");
    const initialState = await vertexToggle.isChecked();
    expect(initialState).toBe(true);

    // Toggle the setting
    await vertexToggle.click();
    await expect(vertexToggle).not.toBeChecked();

    // Cancel instead of save
    await closeSettings(page);

    // Re-open settings and verify the change was NOT persisted
    await openSettings(page);
    await page.locator("#ai-default-provider").selectOption("vertex_ai");
    await expect(page.locator("text=Vertex AI Configuration")).toBeVisible({ timeout: 3000 });

    // Should still be checked (change was discarded)
    const toggleAfterCancel = page.locator("#vertex-show-in-selector");
    await expect(toggleAfterCancel).toBeChecked();

    await closeSettings(page);
  });
});
