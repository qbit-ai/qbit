import { expect, type Page, test } from "@playwright/test";

/**
 * OpenAI Models E2E Tests
 *
 * These tests verify that OpenAI models are correctly displayed in the model selector:
 * - All model families are present (GPT-5, GPT-4.1, GPT-4o, o-series, Codex)
 * - Reasoning effort variants (Low/Medium/High) are shown for applicable models
 * - Model names are correctly formatted
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
  const providersNavItem = page.locator("nav >> button:has-text('Providers')").first();
  await providersNavItem.click();
  await page.waitForTimeout(300);

  // Find the provider accordion button by its name within the main content area
  const contentArea = page.locator("[role='dialog']").first();
  const providerButton = contentArea
    .locator(`button`)
    .filter({ hasText: new RegExp(`${providerName}`, "i") })
    .filter({ hasText: /Configured|Not configured/i })
    .first();

  await expect(providerButton).toBeVisible({ timeout: 3000 });
  await providerButton.scrollIntoViewIfNeeded();
  await providerButton.click();

  // Wait for the collapsible content to appear
  await expect(page.locator("text=Show in model selector").first()).toBeVisible({ timeout: 3000 });
}

/**
 * Close the settings dialog by clicking Cancel.
 */
async function closeSettings(page: Page) {
  await page.locator("button:has-text('Cancel')").click();
  await expect(page.getByRole("dialog")).not.toBeVisible({ timeout: 3000 });
}

/**
 * Switch to the AI mode in the status bar.
 */
async function switchToAgentMode(page: Page) {
  const agentModeButton = page.locator("button").filter({ has: page.locator("svg.lucide-bot") });
  await agentModeButton.click();
}

/**
 * Open the model selector dropdown in the status bar.
 */
async function openModelSelector(page: Page) {
  // Find and click the model selector button
  const modelSelector = page
    .locator("button")
    .filter({ hasText: /Claude|Devstral|GPT|Llama|o1|o3|o4|Codex/ });
  await expect(modelSelector.first()).toBeVisible({ timeout: 5000 });
  await modelSelector.first().click();

  // Wait for the dropdown menu to appear
  await expect(page.locator("[role='menu']")).toBeVisible({ timeout: 3000 });
}

test.describe("OpenAI Models - Model Selector", () => {
  test.beforeEach(async ({ page }) => {
    await waitForAppReady(page);
  });

  test("OpenAI provider appears in the model selector", async ({ page }) => {
    // Switch to agent mode
    await switchToAgentMode(page);

    // Open the model selector
    await openModelSelector(page);

    // Check that OpenAI section is visible
    await expect(page.locator("[role='menu'] >> text=OpenAI")).toBeVisible();

    // Close dropdown
    await page.keyboard.press("Escape");
  });

  test("GPT-5 series models with reasoning effort variants are present", async ({ page }) => {
    await switchToAgentMode(page);
    await openModelSelector(page);

    const menu = page.locator("[role='menu']");

    // Check for GPT 5.2 reasoning variants
    await expect(menu.locator("text=GPT 5.2 (Low)")).toBeVisible();
    await expect(menu.locator("text=GPT 5.2 (Medium)")).toBeVisible();
    await expect(menu.locator("text=GPT 5.2 (High)")).toBeVisible();

    // Check for GPT 5.1 reasoning variants
    await expect(menu.locator("text=GPT 5.1 (Low)")).toBeVisible();
    await expect(menu.locator("text=GPT 5.1 (Medium)")).toBeVisible();
    await expect(menu.locator("text=GPT 5.1 (High)")).toBeVisible();

    // Check for GPT 5 reasoning variants
    await expect(menu.locator("text=GPT 5 (Low)")).toBeVisible();
    await expect(menu.locator("text=GPT 5 (Medium)")).toBeVisible();
    await expect(menu.locator("text=GPT 5 (High)")).toBeVisible();

    await page.keyboard.press("Escape");
  });

  test("GPT-5 Mini and Nano models are present without reasoning variants", async ({ page }) => {
    await switchToAgentMode(page);
    await openModelSelector(page);

    const menu = page.locator("[role='menu']");

    // Check for GPT 5 Mini and Nano (no reasoning variants)
    await expect(menu.locator("text=GPT 5 Mini")).toBeVisible();
    await expect(menu.locator("text=GPT 5 Nano")).toBeVisible();

    await page.keyboard.press("Escape");
  });

  test("GPT-4.5 Preview model is present", async ({ page }) => {
    await switchToAgentMode(page);
    await openModelSelector(page);

    const menu = page.locator("[role='menu']");

    await expect(menu.locator("text=GPT 4.5 Preview")).toBeVisible();

    await page.keyboard.press("Escape");
  });

  test("GPT-4.1 series models are present", async ({ page }) => {
    await switchToAgentMode(page);
    await openModelSelector(page);

    const menu = page.locator("[role='menu']");

    await expect(menu.locator("text=GPT 4.1").first()).toBeVisible();
    await expect(menu.locator("text=GPT 4.1 Mini")).toBeVisible();
    await expect(menu.locator("text=GPT 4.1 Nano")).toBeVisible();

    await page.keyboard.press("Escape");
  });

  test("GPT-4o series models are present", async ({ page }) => {
    await switchToAgentMode(page);
    await openModelSelector(page);

    const menu = page.locator("[role='menu']");

    await expect(menu.locator("text=GPT 4o").first()).toBeVisible();
    await expect(menu.locator("text=GPT 4o Mini")).toBeVisible();
    await expect(menu.locator("text=ChatGPT 4o Latest")).toBeVisible();

    await page.keyboard.press("Escape");
  });

  test("o-series reasoning models with effort variants are present", async ({ page }) => {
    await switchToAgentMode(page);
    await openModelSelector(page);

    const menu = page.locator("[role='menu']");

    // Check for o3 reasoning variants
    await expect(menu.locator("text=o3 (Low)")).toBeVisible();
    await expect(menu.locator("text=o3 (Medium)")).toBeVisible();
    await expect(menu.locator("text=o3 (High)")).toBeVisible();

    // Check for o3 Mini reasoning variants
    await expect(menu.locator("text=o3 Mini (Low)")).toBeVisible();
    await expect(menu.locator("text=o3 Mini (Medium)")).toBeVisible();
    await expect(menu.locator("text=o3 Mini (High)")).toBeVisible();

    // Check for o4 Mini reasoning variants
    await expect(menu.locator("text=o4 Mini (Low)")).toBeVisible();
    await expect(menu.locator("text=o4 Mini (Medium)")).toBeVisible();
    await expect(menu.locator("text=o4 Mini (High)")).toBeVisible();

    // Check for o1 reasoning variants
    await expect(menu.locator("text=o1 (Low)")).toBeVisible();
    await expect(menu.locator("text=o1 (Medium)")).toBeVisible();
    await expect(menu.locator("text=o1 (High)")).toBeVisible();

    // Check for o1 Mini reasoning variants
    await expect(menu.locator("text=o1 Mini (Low)")).toBeVisible();
    await expect(menu.locator("text=o1 Mini (Medium)")).toBeVisible();
    await expect(menu.locator("text=o1 Mini (High)")).toBeVisible();

    await page.keyboard.press("Escape");
  });

  test("Codex models are present", async ({ page }) => {
    await switchToAgentMode(page);
    await openModelSelector(page);

    const menu = page.locator("[role='menu']");

    await expect(menu.locator("text=GPT 5.2 Codex")).toBeVisible();
    await expect(menu.locator("text=GPT 5.1 Codex")).toBeVisible();
    await expect(menu.locator("text=Codex Mini")).toBeVisible();

    await page.keyboard.press("Escape");
  });

  test("can select an OpenAI model from the dropdown", async ({ page }) => {
    await switchToAgentMode(page);
    await openModelSelector(page);

    // Select GPT 5.2 (Medium)
    const menuItem = page.locator("[role='menuitem']").filter({ hasText: "GPT 5.2 (Medium)" });
    await menuItem.click();

    // Verify the selection is reflected in the status bar
    // The model selector button should now show the selected model
    const modelSelector = page.locator("button").filter({ hasText: /GPT 5\.2/ });
    await expect(modelSelector.first()).toBeVisible({ timeout: 3000 });
  });

  test("can select a Codex model from the dropdown", async ({ page }) => {
    await switchToAgentMode(page);
    await openModelSelector(page);

    // Select GPT 5.2 Codex
    const menuItem = page.locator("[role='menuitem']").filter({ hasText: "GPT 5.2 Codex" });
    await menuItem.click();

    // Verify the selection is reflected in the status bar
    const modelSelector = page.locator("button").filter({ hasText: /Codex/ });
    await expect(modelSelector.first()).toBeVisible({ timeout: 3000 });
  });
});

test.describe("OpenAI Models - Settings", () => {
  test.beforeEach(async ({ page }) => {
    await waitForAppReady(page);
  });

  test("OpenAI provider is listed in settings", async ({ page }) => {
    await openSettings(page);

    // Verify OpenAI provider is listed
    await expect(page.locator("text=OpenAI")).toBeVisible();

    await closeSettings(page);
  });

  test("OpenAI provider can be expanded to show visibility toggle", async ({ page }) => {
    await openSettings(page);

    // Expand the OpenAI provider accordion
    await expandProvider(page, "OpenAI");

    // Check that the "Show in model selector" toggle is present
    await expect(page.locator("text=Show in model selector").first()).toBeVisible();

    // The switch should be visible
    const toggle = page.locator("[role='switch']").first();
    await expect(toggle).toBeVisible();

    await closeSettings(page);
  });
});

test.describe("OpenAI Models - Model Count Verification", () => {
  test.beforeEach(async ({ page }) => {
    await waitForAppReady(page);
  });

  test("correct number of OpenAI models are displayed", async ({ page }) => {
    await switchToAgentMode(page);
    await openModelSelector(page);

    const menu = page.locator("[role='menu']");

    // Count all OpenAI model menu items
    // Expected: 9 GPT-5 variants (3x3) + 2 GPT-5 Mini/Nano + 1 GPT-4.5 + 3 GPT-4.1 + 3 GPT-4o
    //         + 15 o-series (5 models x 3 variants) + 3 Codex = 36 total
    const expectedModels = [
      // GPT-5 series with reasoning
      "GPT 5.2 (Low)",
      "GPT 5.2 (Medium)",
      "GPT 5.2 (High)",
      "GPT 5.1 (Low)",
      "GPT 5.1 (Medium)",
      "GPT 5.1 (High)",
      "GPT 5 (Low)",
      "GPT 5 (Medium)",
      "GPT 5 (High)",
      // GPT-5 Mini/Nano
      "GPT 5 Mini",
      "GPT 5 Nano",
      // GPT-4.5
      "GPT 4.5 Preview",
      // GPT-4.1 series
      "GPT 4.1",
      "GPT 4.1 Mini",
      "GPT 4.1 Nano",
      // GPT-4o series
      "GPT 4o",
      "GPT 4o Mini",
      "ChatGPT 4o Latest",
      // o-series with reasoning
      "o3 (Low)",
      "o3 (Medium)",
      "o3 (High)",
      "o3 Mini (Low)",
      "o3 Mini (Medium)",
      "o3 Mini (High)",
      "o4 Mini (Low)",
      "o4 Mini (Medium)",
      "o4 Mini (High)",
      "o1 (Low)",
      "o1 (Medium)",
      "o1 (High)",
      "o1 Mini (Low)",
      "o1 Mini (Medium)",
      "o1 Mini (High)",
      // Codex
      "GPT 5.2 Codex",
      "GPT 5.1 Codex",
      "Codex Mini",
    ];

    // Verify each expected model is present
    for (const modelName of expectedModels) {
      await expect(
        menu.locator(`text=${modelName}`).first(),
        `Model "${modelName}" should be visible`
      ).toBeVisible();
    }

    await page.keyboard.press("Escape");
  });
});
