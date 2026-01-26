import { expect, type Locator, type Page, test } from "@playwright/test";

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

  // The UI used to expose a status-bar test id; that has been removed.
  // Use stable, user-visible controls as the readiness signal instead.
  const terminalMode = page.getByRole("button", { name: "Switch to Terminal mode" });
  const aiMode = page.getByRole("button", { name: "Switch to AI mode" });
  await expect(async () => {
    const terminalVisible = await terminalMode.isVisible().catch(() => false);
    const aiVisible = await aiMode.isVisible().catch(() => false);
    expect(terminalVisible || aiVisible).toBe(true);
  }).toPass({ timeout: 15000 });

  // Some builds render the Command Palette by default; close it so it doesn't
  // capture keyboard shortcuts used by the tests (e.g. Meta+, for Settings).
  const commandPaletteHeading = page.getByRole("heading", { name: "Command Palette" });
  if (await commandPaletteHeading.isVisible().catch(() => false)) {
    await page.keyboard.press("Escape");

    // Even if the palette remains visible (some builds keep it docked), ensure
    // focus is returned to the main UI so keyboard shortcuts work.
    const unifiedInput = page.locator('[data-testid="unified-input"]');
    if (await unifiedInput.isVisible().catch(() => false)) {
      await unifiedInput.click();
    } else {
      await page.locator("body").click({ position: { x: 10, y: 10 } });
    }
  }
}

/**
 * Open the settings dialog via keyboard shortcut.
 */
async function openSettings(page: Page) {
  await page.keyboard.press("Meta+,");
  // Wait for settings tab to appear - look for the Providers nav button
  await expect(page.locator("nav >> button:has-text('Providers')")).toBeVisible({ timeout: 5000 });
  // Wait for settings to load - the Providers section should be visible
  await expect(page.locator("text=Default Model")).toBeVisible({ timeout: 5000 });
}

/**
 * Expand a provider's accordion in the Providers settings.
 */
async function expandProvider(page: Page, providerName: string): Promise<Locator> {
  // Find the provider accordion trigger by accessible name.
  // It includes provider name + configuration state (e.g., "Vertex AI ... Configured").
  const providerButton = page
    .getByRole("button", {
      name: new RegExp(`${providerName}.*(Configured|Not configured)`, "i"),
    })
    .first();

  // Wait for the button to be visible and stable
  await expect(providerButton).toBeVisible({ timeout: 10000 });

  // Scroll into view in case it's below the fold
  await providerButton.scrollIntoViewIfNeeded();

  // Click to expand
  await providerButton.click();

  // In some runs the first click is swallowed (e.g. focus/overlay), leaving the trigger closed.
  // Retry once to avoid flakes.
  if ((await providerButton.getAttribute("data-state")) !== "open") {
    await page.waitForTimeout(100);
    await providerButton.click();
  }

  // Radix Collapsible sets data-state on the trigger; wait for it to be open.
  await expect(providerButton).toHaveAttribute("data-state", "open", { timeout: 10000 });

  // Scope to the provider card container that wraps the trigger + collapsible content.
  // The trigger button sits inside a bordered wrapper div which also contains CollapsibleContent.
  const providerCard = providerButton.locator(
    'xpath=ancestor::div[contains(@class, "overflow-hidden")][1]'
  );

  // Wait for the collapsible content to appear (switch rendered next to label)
  await expect(providerCard.getByRole("switch").first()).toBeVisible({ timeout: 10000 });

  return providerCard;
}

/**
 * Get the visibility toggle switch for the currently expanded provider.
 */
function getVisibilityToggle(providerCard: Locator) {
  return providerCard.getByRole("switch").first();
}

/**
 * The Settings UI now auto-saves changes and dispatches a DOM-level `settings-updated` event.
 * Wrap an action that triggers a save and wait until the event fires.
 */
async function withNextSettingsUpdated(page: Page, action: () => Promise<void>) {
  const waitForUpdated = page.evaluate(
    () =>
      new Promise<void>((resolve) => {
        const handler = () => resolve();
        window.addEventListener("settings-updated", handler, { once: true });
      })
  );

  await action();
  await waitForUpdated;

  // Give React a tick to re-render any dependent UI.
  await page.waitForTimeout(50);
}

async function toggleAndWaitForSave(page: Page, toggle: Locator) {
  await withNextSettingsUpdated(page, async () => {
    await toggle.click();
  });
}

/**
 * Close settings tab by clicking the X button on the Settings tab.
 */
async function closeSettings(page: Page) {
  // Find the Settings tab and click its close button
  // The tab has "Settings" text and a sibling close button with title="Close tab"
  const settingsTab = page.locator('[role="tablist"]').getByText("Settings");
  await expect(settingsTab).toBeVisible({ timeout: 3000 });

  // The close button is a sibling of the tab trigger, within the same parent div
  const tabContainer = settingsTab.locator("../..");
  const closeButton = tabContainer.locator('button[title="Close tab"]');
  await closeButton.click();

  // Wait for the settings tab to be closed (Providers nav should not be visible)
  await expect(page.locator("nav >> button:has-text('Providers')")).not.toBeVisible({
    timeout: 3000,
  });
}

/**
 * Switch to the AI mode in the status bar.
 */
async function switchToAgentMode(page: Page) {
  // Use the status bar toggle button's aria-label to avoid ambiguous icon matches.
  const switchToAi = page.getByRole("button", { name: "Switch to AI mode" });
  const isSwitchToAiVisible = await switchToAi.isVisible().catch(() => false);
  if (isSwitchToAiVisible) {
    await switchToAi.click();
    return;
  }

  // If we're already in AI mode, the "Switch to Terminal mode" button should be present.
  await expect(page.getByRole("button", { name: "Switch to Terminal mode" })).toBeVisible({
    timeout: 5000,
  });
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
    // Use negative lookbehind to avoid matching "Z.AI (Anthropic)"
    await expect(
      page.getByRole("button", { name: /(?<!\()Anthropic.*Not configured/ })
    ).toBeVisible();

    // Close settings
    await closeSettings(page);
  });

  test("Vertex AI provider settings show visibility toggle when expanded", async ({ page }) => {
    // Open settings
    await openSettings(page);

    // Expand the Vertex AI provider accordion
    const vertexProvider = await expandProvider(page, "Vertex AI");

    // Check that the "Show in model selector" toggle is present
    await expect(page.locator("text=Show in model selector").first()).toBeVisible();

    // The switch should be visible
    const vertexToggle = getVisibilityToggle(vertexProvider);
    await expect(vertexToggle).toBeVisible();

    // Close settings
    await closeSettings(page);
  });

  test("OpenRouter provider settings show visibility toggle when expanded", async ({ page }) => {
    // Open settings
    await openSettings(page);

    // Expand the OpenRouter provider accordion
    const openRouterProvider = await expandProvider(page, "OpenRouter");

    // Check that the "Show in model selector" toggle is present
    await expect(page.locator("text=Show in model selector").first()).toBeVisible();

    // The switch should be visible
    const openRouterToggle = getVisibilityToggle(openRouterProvider);
    await expect(openRouterToggle).toBeVisible();

    // Close settings
    await closeSettings(page);
  });

  test("provider visibility toggle can be clicked and saves state", async ({ page }) => {
    // Open settings
    await openSettings(page);

    // Expand the Vertex AI provider accordion
    const vertexProvider = await expandProvider(page, "Vertex AI");

    // Get the toggle and check its initial state (should be checked/true by default in mocks)
    const vertexToggle = getVisibilityToggle(vertexProvider);
    const initialState = await vertexToggle.getAttribute("data-state");
    expect(initialState).toBe("checked");

    // Click to toggle off
    await toggleAndWaitForSave(page, vertexToggle);

    // Verify it's now unchecked
    await expect(vertexToggle).toHaveAttribute("data-state", "unchecked");

    // Auto-saved; state should persist

    // Re-open settings to verify persistence
    await openSettings(page);
    const vertexProvider2 = await expandProvider(page, "Vertex AI");

    // Verify the toggle state persisted
    const persistedToggle = getVisibilityToggle(vertexProvider2);
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
    const openRouterProvider = await expandProvider(page, "OpenRouter");

    // Toggle off OpenRouter visibility
    const openRouterToggle = getVisibilityToggle(openRouterProvider);
    await toggleAndWaitForSave(page, openRouterToggle);
    await expect(openRouterToggle).toHaveAttribute("data-state", "unchecked");

    // Close settings to avoid ambiguous button matches
    await closeSettings(page);

    // Switch to agent mode
    await switchToAgentMode(page);

    // Wait for the model selector (Ollama should still be available)
    const modelSelector = page.locator("button").filter({ hasText: /Claude|Devstral|GPT|Llama/ });
    await expect(modelSelector.first()).toBeVisible({ timeout: 5000 });

    // Click the model selector to open dropdown
    await modelSelector.first().click();

    // Ollama should be visible, but OpenRouter should NOT be visible
    await expect(page.locator("[role='menu'] >> text=Ollama")).toBeVisible({ timeout: 10000 });
    await expect(page.locator("[role='menu'] >> text=OpenRouter")).not.toBeVisible();

    // Close dropdown
    await page.keyboard.press("Escape");
  });

  test("disabling all providers shows 'Enable a provider' message", async ({ page }) => {
    // Disable all providers by expanding each and toggling them off
    // This is a sequential operation - expand, toggle off, save, repeat for each provider

    // Disable Vertex AI
    await openSettings(page);
    const vertexProvider = await expandProvider(page, "Vertex AI");
    await toggleAndWaitForSave(page, getVisibilityToggle(vertexProvider));
    await closeSettings(page);

    // Disable OpenRouter
    await openSettings(page);
    const openRouterProvider = await expandProvider(page, "OpenRouter");
    await toggleAndWaitForSave(page, getVisibilityToggle(openRouterProvider));
    await closeSettings(page);

    // Disable Ollama (doesn't require API key, so it's enabled by default)
    await openSettings(page);
    const ollamaProvider = await expandProvider(page, "Ollama");
    await toggleAndWaitForSave(page, getVisibilityToggle(ollamaProvider));
    await closeSettings(page);

    // Disable the rest of the providers that might be visible
    const otherProviders = ["Anthropic", "Gemini", "Groq", "OpenAI", "xAI"];
    for (const provider of otherProviders) {
      await openSettings(page);
      const providerCard = await expandProvider(page, provider);
      await toggleAndWaitForSave(page, getVisibilityToggle(providerCard));
      await closeSettings(page);
    }

    // Switch to agent mode
    await switchToAgentMode(page);

    // Should see the "Enable a provider in settings" message instead of model selector
    await expect(page.locator("text=Enable a provider in settings")).toBeVisible({
      timeout: 10000,
    });
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
      const providerCard = await expandProvider(page, provider);
      await toggleAndWaitForSave(page, getVisibilityToggle(providerCard));
      await closeSettings(page);
    }

    // Verify message is shown
    await switchToAgentMode(page);
    await expect(page.locator("text=Enable a provider in settings")).toBeVisible({
      timeout: 10000,
    });

    // Now re-enable Ollama (which doesn't require an API key in mock mode)
    await openSettings(page);
    const ollamaProvider = await expandProvider(page, "Ollama");
    await toggleAndWaitForSave(page, getVisibilityToggle(ollamaProvider)); // Toggle back on

    // The "Enable a provider" message should no longer be visible
    // because at least one provider is now enabled
    await expect(page.locator("text=Enable a provider in settings")).not.toBeVisible({
      timeout: 5000,
    });
  });
});

test.describe("Provider Visibility Toggle - Settings Persistence", () => {
  test.beforeEach(async ({ page }) => {
    await waitForAppReady(page);
  });

  test("settings changes trigger settings-updated event", async ({ page }) => {
    await openSettings(page);
    const vertexProvider = await expandProvider(page, "Vertex AI");

    const vertexToggle = getVisibilityToggle(vertexProvider);
    const initialState = await vertexToggle.getAttribute("data-state");

    // Toggle the visibility
    await toggleAndWaitForSave(page, vertexToggle);

    // Close and re-open to verify persistence
    await closeSettings(page);
    await openSettings(page);

    const vertexProvider2 = await expandProvider(page, "Vertex AI");
    const vertexToggle2 = getVisibilityToggle(vertexProvider2);

    const persistedState = await vertexToggle2.getAttribute("data-state");
    expect(persistedState).not.toBe(initialState);

    await closeSettings(page);
  });

  test("closing settings persists changes (auto-save)", async ({ page }) => {
    // Open settings and verify initial state
    await openSettings(page);
    const vertexProvider = await expandProvider(page, "Vertex AI");

    const vertexToggle = getVisibilityToggle(vertexProvider);
    const initialState = await vertexToggle.getAttribute("data-state");
    expect(initialState).toBe("checked");

    // Toggle the setting
    await toggleAndWaitForSave(page, vertexToggle);
    await expect(vertexToggle).toHaveAttribute("data-state", "unchecked");

    // Close settings tab (auto-save already occurred)
    await closeSettings(page);

    // Re-open settings and verify the change WAS persisted
    await openSettings(page);
    const vertexProvider2 = await expandProvider(page, "Vertex AI");
    const toggleAfterClose = getVisibilityToggle(vertexProvider2);
    await expect(toggleAfterClose).toHaveAttribute("data-state", "unchecked");

    await closeSettings(page);
  });
});
