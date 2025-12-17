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
  // Wait for settings to load - the Providers section should be visible (default section)
  await expect(page.locator("text=Default Model")).toBeVisible({ timeout: 5000 });
}

/**
 * Expand a provider's accordion in the Providers settings.
 */
async function expandProvider(page: Page, providerName: string) {
  // Click on the "Providers" nav item in the sidebar to ensure we're in the right section
  // and to close any open dropdowns (clicking outside the dropdown closes it)
  const providersNavItem = page.locator("nav >> button:has-text('Providers')").first();
  await providersNavItem.click();
  await page.waitForTimeout(300); // Wait for any dropdown animation to complete

  // Find the provider accordion button by its name within the main content area
  // Provider buttons show: emoji + name + [Default badge] + status (e.g., "🔷 Vertex AI Default Configured")
  // Exclude the navigation sidebar buttons by looking in the content area
  const contentArea = page.locator("[role='dialog']").first();
  const providerButton = contentArea
    .locator(`button`)
    .filter({ hasText: new RegExp(`${providerName}`, "i") })
    .filter({ hasText: /Configured|Not configured/i })
    .first();

  // Wait for the button to be visible and stable
  await expect(providerButton).toBeVisible({ timeout: 3000 });

  // Scroll into view in case it's below the fold
  await providerButton.scrollIntoViewIfNeeded();

  // Click to expand
  await providerButton.click();

  // Wait for the collapsible content to appear (Show in model selector toggle)
  await expect(page.locator("text=Show in model selector").first()).toBeVisible({ timeout: 3000 });
}

/**
 * Get the visibility toggle switch for the currently expanded provider.
 */
function getVisibilityToggle(page: Page) {
  // The switch is inside the expanded collapsible content, next to "Show in model selector" text
  return page.locator("[role='switch']").first();
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

  test("settings dialog shows Providers section with provider visibility toggles", async ({
    page,
  }) => {
    // Open settings
    await openSettings(page);

    // Providers section should be visible and selected by default
    await expect(page.locator("text=Providers").first()).toBeVisible();
    await expect(page.locator("text=Default Model")).toBeVisible();

    // Verify at least one provider is listed (e.g., Anthropic)
    await expect(page.locator("text=Anthropic")).toBeVisible();

    // Close settings
    await closeSettings(page);
  });

  test("Vertex AI provider settings show visibility toggle when expanded", async ({ page }) => {
    // Open settings
    await openSettings(page);

    // Expand the Vertex AI provider accordion
    await expandProvider(page, "Vertex AI");

    // Check that the "Show in model selector" toggle is present
    await expect(page.locator("text=Show in model selector").first()).toBeVisible();

    // The switch should be visible
    const vertexToggle = getVisibilityToggle(page);
    await expect(vertexToggle).toBeVisible();

    // Close settings
    await closeSettings(page);
  });

  test("OpenRouter provider settings show visibility toggle when expanded", async ({ page }) => {
    // Open settings
    await openSettings(page);

    // Expand the OpenRouter provider accordion
    await expandProvider(page, "OpenRouter");

    // Check that the "Show in model selector" toggle is present
    await expect(page.locator("text=Show in model selector").first()).toBeVisible();

    // The switch should be visible
    const openRouterToggle = getVisibilityToggle(page);
    await expect(openRouterToggle).toBeVisible();

    // Close settings
    await closeSettings(page);
  });

  test("provider visibility toggle can be clicked and saves state", async ({ page }) => {
    // Open settings
    await openSettings(page);

    // Expand the Vertex AI provider accordion
    await expandProvider(page, "Vertex AI");

    // Get the toggle and check its initial state (should be checked/true by default in mocks)
    const vertexToggle = getVisibilityToggle(page);
    const initialState = await vertexToggle.getAttribute("data-state");
    expect(initialState).toBe("checked");

    // Click to toggle off
    await vertexToggle.click();

    // Verify it's now unchecked
    await expect(vertexToggle).toHaveAttribute("data-state", "unchecked");

    // Save settings
    await saveSettings(page);

    // Re-open settings to verify persistence
    await openSettings(page);
    await expandProvider(page, "Vertex AI");

    // Verify the toggle state persisted
    const persistedToggle = getVisibilityToggle(page);
    await expect(persistedToggle).toHaveAttribute("data-state", "unchecked");

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
    // In mock browser mode, OpenRouter and Ollama are available (Vertex AI requires AI initialization)
    const modelSelector = page.locator("button").filter({ hasText: /Claude|Devstral|GPT|Llama/ });
    await expect(modelSelector.first()).toBeVisible({ timeout: 5000 });

    // Click the model selector to open dropdown
    await modelSelector.first().click();

    // OpenRouter and Ollama sections should be visible in mock mode
    // (Vertex AI requires vertexConfig which is only set after AI initialization)
    await expect(page.locator("[role='menu'] >> text=OpenRouter")).toBeVisible();
    await expect(page.locator("[role='menu'] >> text=Ollama")).toBeVisible();

    // Click outside to close dropdown
    await page.keyboard.press("Escape");
  });

  test("disabling a provider hides it from the model selector", async ({ page }) => {
    // First, open settings and disable OpenRouter
    await openSettings(page);
    await expandProvider(page, "OpenRouter");

    // Toggle off OpenRouter visibility
    const openRouterToggle = getVisibilityToggle(page);
    await openRouterToggle.click();
    await expect(openRouterToggle).toHaveAttribute("data-state", "unchecked");

    // Save settings
    await saveSettings(page);

    // Switch to agent mode
    await switchToAgentMode(page);

    // Wait for the model selector (Ollama should still be available)
    const modelSelector = page.locator("button").filter({ hasText: /Claude|Devstral|GPT|Llama/ });
    await expect(modelSelector.first()).toBeVisible({ timeout: 5000 });

    // Click the model selector to open dropdown
    await modelSelector.first().click();

    // Ollama should be visible, but OpenRouter should NOT be visible
    await expect(page.locator("[role='menu'] >> text=Ollama")).toBeVisible();
    await expect(page.locator("[role='menu'] >> text=OpenRouter")).not.toBeVisible();

    // Close dropdown
    await page.keyboard.press("Escape");
  });

  test("disabling all providers shows 'Enable a provider' message", async ({ page }) => {
    // Disable all providers by expanding each and toggling them off
    // This is a sequential operation - expand, toggle off, save, repeat for each provider

    // Disable Vertex AI
    await openSettings(page);
    await expandProvider(page, "Vertex AI");
    await getVisibilityToggle(page).click();
    await saveSettings(page);

    // Disable OpenRouter
    await openSettings(page);
    await expandProvider(page, "OpenRouter");
    await getVisibilityToggle(page).click();
    await saveSettings(page);

    // Disable Ollama (doesn't require API key, so it's enabled by default)
    await openSettings(page);
    await expandProvider(page, "Ollama");
    await getVisibilityToggle(page).click();
    await saveSettings(page);

    // Disable the rest of the providers that might be visible
    const otherProviders = ["Anthropic", "Gemini", "Groq", "OpenAI", "xAI"];
    for (const provider of otherProviders) {
      await openSettings(page);
      await expandProvider(page, provider);
      await getVisibilityToggle(page).click();
      await saveSettings(page);
    }

    // Switch to agent mode
    await switchToAgentMode(page);

    // Should see the "Enable a provider in settings" message instead of model selector
    await expect(page.locator("text=Enable a provider in settings")).toBeVisible({ timeout: 5000 });
  });

  test("re-enabling a provider makes it visible in model selector again", async ({ page }) => {
    // Disable all providers
    const allProviders = [
      "Vertex AI",
      "OpenRouter",
      "Ollama",
      "Anthropic",
      "Gemini",
      "Groq",
      "OpenAI",
      "xAI",
    ];
    for (const provider of allProviders) {
      await openSettings(page);
      await expandProvider(page, provider);
      await getVisibilityToggle(page).click();
      await saveSettings(page);
    }

    // Verify message is shown
    await switchToAgentMode(page);
    await expect(page.locator("text=Enable a provider in settings")).toBeVisible({ timeout: 5000 });

    // Now re-enable OpenRouter (which is configured in mock mode)
    await openSettings(page);
    await expandProvider(page, "OpenRouter");
    await getVisibilityToggle(page).click(); // Toggle back on
    await saveSettings(page);

    // The "Enable a provider" message should no longer be visible
    await expect(page.locator("text=Enable a provider in settings")).not.toBeVisible({ timeout: 5000 });

    // A model selector should now be visible (could show any available model)
    // Look for the model selector button which contains model name and chevron
    const modelSelector = page.locator("button").filter({ hasText: /Claude|Devstral|GPT|Llama/ });
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
    await expandProvider(page, "Vertex AI");

    // Toggle the visibility
    await getVisibilityToggle(page).click();

    // Save settings (this should trigger the settings-updated event)
    await saveSettings(page);

    // Verify the event was triggered
    const eventTriggered = await settingsUpdatedPromise;
    expect(eventTriggered).toBe(true);
  });

  test("cancel button discards changes", async ({ page }) => {
    // Open settings and verify initial state
    await openSettings(page);
    await expandProvider(page, "Vertex AI");

    const vertexToggle = getVisibilityToggle(page);
    const initialState = await vertexToggle.getAttribute("data-state");
    expect(initialState).toBe("checked");

    // Toggle the setting
    await vertexToggle.click();
    await expect(vertexToggle).toHaveAttribute("data-state", "unchecked");

    // Cancel instead of save
    await closeSettings(page);

    // Re-open settings and verify the change was NOT persisted
    await openSettings(page);
    await expandProvider(page, "Vertex AI");

    // Should still be checked (change was discarded)
    const toggleAfterCancel = getVisibilityToggle(page);
    await expect(toggleAfterCancel).toHaveAttribute("data-state", "checked");

    await closeSettings(page);
  });
});
