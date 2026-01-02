import { expect, type Page, test } from "@playwright/test";

/**
 * OpenAI Models E2E Tests
 *
 * These tests verify that OpenAI models are correctly displayed in the model selector:
 * - All model families are present (GPT-5, GPT-4.1, GPT-4o, o-series, Codex)
 * - Reasoning effort variants (Low/Medium/High) are shown for applicable models
 * - Model names are correctly formatted
 *
 * Note: The model selector uses a 3-level nested sub-menu structure:
 * - Level 1: Model family (e.g., "GPT-5 Series", "o-Series", "Codex")
 * - Level 2: Model variant (e.g., "GPT 5.2", "o3", "o4 Mini")
 * - Level 3: Reasoning effort (e.g., "Low", "Medium", "High")
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

/**
 * Open a nested sub-menu by hovering on the trigger.
 * Waits for the sub-menu content to appear before returning.
 */
async function openSubMenu(page: Page, triggerText: string) {
  // Find the sub-menu trigger using data-slot attribute (Radix UI) or role
  const trigger = page
    .locator('[data-slot="dropdown-menu-sub-trigger"], [role="menuitem"]')
    .filter({ hasText: triggerText })
    .first();
  await expect(trigger).toBeVisible({ timeout: 3000 });

  // Hover and click to ensure sub-menu opens reliably
  await trigger.hover();
  await page.waitForTimeout(150);

  // Some sub-menus need a click to open reliably
  // If sub-menu didn't open from hover, try clicking
  await trigger.click();
  await page.waitForTimeout(200);
}

/**
 * Check if a flat model exists in the menu (no nesting).
 */
async function verifyFlatModel(page: Page, modelName: string) {
  const menu = page.locator("[role='menu']");
  await expect(menu.getByText(modelName, { exact: false }).first()).toBeVisible({ timeout: 3000 });
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

  test("GPT-5 Series sub-menu is present with model variants", async ({ page }) => {
    await switchToAgentMode(page);
    await openModelSelector(page);

    // Verify GPT-5 Series exists as a top-level sub-menu
    await verifyFlatModel(page, "GPT-5 Series");

    // Open GPT-5 Series and check for GPT 5.2, 5.1, 5
    await openSubMenu(page, "GPT-5 Series");

    // Check for model variants in the sub-menu
    await expect(page.getByText("GPT 5.2").first()).toBeVisible({ timeout: 3000 });
    await expect(page.getByText("GPT 5.1").first()).toBeVisible({ timeout: 3000 });
    await expect(page.getByText("GPT 5 Mini").first()).toBeVisible({ timeout: 3000 });
    await expect(page.getByText("GPT 5 Nano").first()).toBeVisible({ timeout: 3000 });

    await page.keyboard.press("Escape");
  });

  test("GPT 5.2 sub-menu trigger is accessible from GPT-5 Series", async ({ page }) => {
    await switchToAgentMode(page);
    await openModelSelector(page);

    // Navigate: GPT-5 Series > verify GPT 5.2 is visible as a sub-menu trigger
    await openSubMenu(page, "GPT-5 Series");

    // Verify GPT 5.2 appears as a sub-menu trigger (has chevron icon)
    // This confirms the 3-level nesting structure exists
    const gpt52Trigger = page
      .locator('[data-slot="dropdown-menu-sub-trigger"]')
      .filter({ hasText: "GPT 5.2" });
    await expect(gpt52Trigger.first()).toBeVisible({ timeout: 3000 });

    await page.keyboard.press("Escape");
  });

  test("GPT-4 Series sub-menu contains GPT 4.1 and 4o models", async ({ page }) => {
    await switchToAgentMode(page);
    await openModelSelector(page);

    // Verify GPT-4 Series exists
    await verifyFlatModel(page, "GPT-4 Series");

    // Open GPT-4 Series
    await openSubMenu(page, "GPT-4 Series");

    // Check for GPT-4.1 models
    await expect(page.getByText("GPT 4.1").first()).toBeVisible({ timeout: 3000 });
    await expect(page.getByText("GPT 4.1 Mini").first()).toBeVisible({ timeout: 3000 });
    await expect(page.getByText("GPT 4.1 Nano").first()).toBeVisible({ timeout: 3000 });

    // Check for GPT-4o models
    await expect(page.getByText("GPT 4o").first()).toBeVisible({ timeout: 3000 });
    await expect(page.getByText("GPT 4o Mini").first()).toBeVisible({ timeout: 3000 });
    await expect(page.getByText("ChatGPT 4o Latest").first()).toBeVisible({ timeout: 3000 });

    await page.keyboard.press("Escape");
  });

  test("o-Series sub-menu contains o3, o3 Mini, o4 Mini, and o1", async ({ page }) => {
    await switchToAgentMode(page);
    await openModelSelector(page);

    // Verify o-Series exists
    await verifyFlatModel(page, "o-Series");

    // Open o-Series
    await openSubMenu(page, "o-Series");

    // Check for o-series models
    await expect(page.getByText("o4 Mini").first()).toBeVisible({ timeout: 3000 });
    await expect(page.getByText("o3").first()).toBeVisible({ timeout: 3000 });
    await expect(page.getByText("o3 Mini").first()).toBeVisible({ timeout: 3000 });
    await expect(page.getByText("o1").first()).toBeVisible({ timeout: 3000 });

    await page.keyboard.press("Escape");
  });

  test("o3 sub-menu trigger is accessible from o-Series", async ({ page }) => {
    await switchToAgentMode(page);
    await openModelSelector(page);

    // Navigate: o-Series > verify o3 is visible as a sub-menu trigger
    await openSubMenu(page, "o-Series");

    // Verify o3 appears as a sub-menu trigger (has chevron icon)
    // This confirms reasoning effort variants exist in a nested sub-menu
    const o3Trigger = page
      .locator('[data-slot="dropdown-menu-sub-trigger"]')
      .filter({ hasText: /^o3$/ });
    await expect(o3Trigger.first()).toBeVisible({ timeout: 3000 });

    await page.keyboard.press("Escape");
  });

  test("Codex sub-menu contains GPT 5.1 Codex models", async ({ page }) => {
    await switchToAgentMode(page);
    await openModelSelector(page);

    // Verify Codex exists
    await verifyFlatModel(page, "Codex");

    // Open Codex
    await openSubMenu(page, "Codex");

    // Check for Codex models
    await expect(page.getByText("GPT 5.1 Codex").first()).toBeVisible({ timeout: 3000 });
    await expect(page.getByText("GPT 5.1 Codex Max").first()).toBeVisible({ timeout: 3000 });
    await expect(page.getByText("Codex Mini").first()).toBeVisible({ timeout: 3000 });

    await page.keyboard.press("Escape");
  });

  test("can select GPT 4.1 from GPT-4 Series menu", async ({ page }) => {
    await switchToAgentMode(page);
    await openModelSelector(page);

    // Navigate: GPT-4 Series > GPT 4.1 (2-level navigation, no reasoning effort)
    await openSubMenu(page, "GPT-4 Series");

    // Click GPT 4.1
    const menuItem = page.getByRole("menuitem").filter({ hasText: "GPT 4.1" });
    await menuItem.first().click();

    // Verify the selection is reflected in the status bar
    await expect(
      page
        .locator("button")
        .filter({ hasText: /GPT 4\.1/ })
        .first()
    ).toBeVisible({ timeout: 3000 });
  });

  test("can select Codex Mini from nested menu", async ({ page }) => {
    await switchToAgentMode(page);
    await openModelSelector(page);

    // Navigate: Codex > Codex Mini
    await openSubMenu(page, "Codex");

    // Click Codex Mini
    const menuItem = page.getByRole("menuitem").filter({ hasText: "Codex Mini" });
    await menuItem.first().click();

    // Verify the selection is reflected in the status bar
    const modelSelector = page.locator("button").filter({ hasText: /Codex Mini/ });
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

test.describe("OpenAI Models - Menu Structure Verification", () => {
  test.beforeEach(async ({ page }) => {
    await waitForAppReady(page);
  });

  test("all top-level OpenAI model groups are present", async ({ page }) => {
    await switchToAgentMode(page);
    await openModelSelector(page);

    const menu = page.locator("[role='menu']");

    // Verify all top-level groups exist
    const expectedGroups = ["GPT-5 Series", "GPT-4 Series", "o-Series", "Codex"];

    for (const groupName of expectedGroups) {
      await expect(
        menu.getByText(groupName, { exact: false }).first(),
        `Group "${groupName}" should be visible`
      ).toBeVisible({ timeout: 3000 });
    }

    await page.keyboard.press("Escape");
  });

  test("GPT-5 Series contains all expected variants", async ({ page }) => {
    await switchToAgentMode(page);
    await openModelSelector(page);

    // Open GPT-5 Series
    await openSubMenu(page, "GPT-5 Series");

    // Expected items in GPT-5 Series
    const expectedModels = ["GPT 5.2", "GPT 5.1", "GPT 5", "GPT 5 Mini", "GPT 5 Nano"];

    for (const modelName of expectedModels) {
      await expect(
        page.getByText(modelName, { exact: false }).first(),
        `Model "${modelName}" should be visible in GPT-5 Series`
      ).toBeVisible({ timeout: 3000 });
    }

    await page.keyboard.press("Escape");
  });

  test("o-Series contains all expected reasoning models", async ({ page }) => {
    await switchToAgentMode(page);
    await openModelSelector(page);

    // Open o-Series
    await openSubMenu(page, "o-Series");

    // Expected items in o-Series (no o1 Mini)
    const expectedModels = ["o4 Mini", "o3", "o3 Mini", "o1"];

    for (const modelName of expectedModels) {
      await expect(
        page.getByText(modelName, { exact: false }).first(),
        `Model "${modelName}" should be visible in o-Series`
      ).toBeVisible({ timeout: 3000 });
    }

    await page.keyboard.press("Escape");
  });
});
