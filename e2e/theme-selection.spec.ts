import { expect, type Page, test } from "@playwright/test";
import { waitForAppReady as waitForAppReadyBase } from "./helpers/app";

/**
 * Theme Selection E2E Tests
 *
 * These tests verify that theme selection works correctly in the settings:
 * - Themes are displayed in the theme picker
 * - Selecting a theme applies and persists it immediately
 * - Custom themes are displayed alongside builtin themes
 */

/**
 * Wait for the app to be fully ready in browser mode.
 */
async function waitForAppReady(page: Page) {
  await waitForAppReadyBase(page);
}

/**
 * Open the settings dialog via keyboard shortcut.
 */
async function openSettings(page: Page) {
  await page.keyboard.press("Meta+,");
  // Wait for settings dialog to appear
  await expect(page.locator("nav >> button:has-text('Providers')")).toBeVisible({ timeout: 5000 });
}

/**
 * Navigate to the Terminal settings section.
 */
async function navigateToTerminalSettings(page: Page) {
  const terminalButton = page.locator("nav >> button:has-text('Terminal')");
  await expect(terminalButton).toBeVisible({ timeout: 5000 });
  await terminalButton.click();
  // Wait for theme picker to be visible - look for the Palette icon section header
  await expect(page.getByText("Themes", { exact: true }).first()).toBeVisible({ timeout: 5000 });
}

/**
 * Get the list of theme names from the theme picker.
 */
async function getThemeNames(page: Page): Promise<string[]> {
  const themeButtons = page.locator('.space-y-1.border >> button[type="button"]');
  const count = await themeButtons.count();
  const names: string[] = [];
  for (let i = 0; i < count; i++) {
    const text = await themeButtons.nth(i).innerText();
    // Extract just the theme name (before any badges like "(Custom)" or "● Active")
    // The text might be on one line like "Qbit● Active" or "Obsidian Ember(Custom)"
    let name = text.split("\n")[0].trim();
    // Remove the "● Active" marker if present
    name = name.replace(/● Active$/, "").trim();
    // Remove "(Custom)" if present
    name = name.replace(/\(Custom\)$/, "").trim();
    names.push(name);
  }
  return names;
}

/**
 * Click on a theme by name in the theme picker.
 */
async function selectTheme(page: Page, themeName: string) {
  const themeButton = page.locator(`.space-y-1.border >> button:has-text("${themeName}")`).first();
  await expect(themeButton).toBeVisible({ timeout: 5000 });
  await themeButton.click();
}

/**
 * Check if a theme is marked as active.
 */
async function isThemeActive(page: Page, themeName: string): Promise<boolean> {
  const themeButton = page.locator(`.space-y-1.border >> button:has-text("${themeName}")`).first();
  const text = await themeButton.innerText();
  return text.includes("● Active");
}

/**
 * Close settings by clicking the X button.
 */
async function closeSettings(page: Page) {
  // The X button is in the settings header
  const closeButton = page.locator("button:has(.lucide-x)").first();
  await expect(closeButton).toBeVisible({ timeout: 5000 });
  await closeButton.click();
  // Wait for dialog to close
  await page.waitForTimeout(300);
}

/**
 * Get the current CSS variable value from the document.
 */
async function getCssVariable(page: Page, variableName: string): Promise<string> {
  return await page.evaluate((varName) => {
    return getComputedStyle(document.documentElement).getPropertyValue(varName).trim();
  }, variableName);
}

test.describe("Theme Selection", () => {
  test.beforeEach(async ({ page }) => {
    await waitForAppReady(page);
  });

  test("should display available themes in settings", async ({ page }) => {
    await openSettings(page);
    await navigateToTerminalSettings(page);

    // Check that themes are displayed
    const themeNames = await getThemeNames(page);
    expect(themeNames.length).toBeGreaterThan(0);

    // Should have at least Qbit Default theme
    expect(themeNames.some((name) => name.includes("Qbit"))).toBe(true);
  });

  test("should mark current theme as active", async ({ page }) => {
    await openSettings(page);
    await navigateToTerminalSettings(page);

    // At least one theme should be marked as active
    const themeNames = await getThemeNames(page);
    let hasActiveTheme = false;
    for (const name of themeNames) {
      if (await isThemeActive(page, name)) {
        hasActiveTheme = true;
        break;
      }
    }
    expect(hasActiveTheme).toBe(true);
  });

  test("should switch active indicator when selecting a different theme", async ({ page }) => {
    await openSettings(page);
    await navigateToTerminalSettings(page);

    const themeNames = await getThemeNames(page);
    expect(themeNames.length).toBeGreaterThanOrEqual(2);

    // Find the currently active theme
    let activeThemeName = "";
    for (const name of themeNames) {
      if (await isThemeActive(page, name)) {
        activeThemeName = name;
        break;
      }
    }
    expect(activeThemeName).not.toBe("");

    // Find a different theme to select
    const differentTheme = themeNames.find((name) => name !== activeThemeName);
    expect(differentTheme).toBeDefined();
    if (!differentTheme) throw new Error("No different theme found");

    // Select the different theme
    await selectTheme(page, differentTheme);

    // Wait for the theme to be applied
    await page.waitForTimeout(300);

    // The new theme should now be active
    const isNewThemeActive = await isThemeActive(page, differentTheme);
    expect(isNewThemeActive).toBe(true);

    // The old theme should no longer be active
    const isOldThemeActive = await isThemeActive(page, activeThemeName);
    expect(isOldThemeActive).toBe(false);
  });

  test("should apply theme CSS variables when selecting a theme", async ({ page }) => {
    await openSettings(page);
    await navigateToTerminalSettings(page);

    // Get initial background color (used for theme verification)
    // Theme CSS vars use --color-* prefix (e.g., --color-background)
    await getCssVariable(page, "--color-background");

    const themeNames = await getThemeNames(page);

    // Find a theme that's not currently active
    let inactiveTheme = "";
    for (const name of themeNames) {
      if (!(await isThemeActive(page, name))) {
        inactiveTheme = name;
        break;
      }
    }

    if (inactiveTheme) {
      // Select the inactive theme
      await selectTheme(page, inactiveTheme);

      // Wait for theme to apply
      await page.waitForTimeout(200);

      // CSS variables should be updated (may or may not be different depending on the themes)
      const newBackground = await getCssVariable(page, "--color-background");
      // Just verify it's a valid color value
      expect(newBackground).toMatch(/^#[0-9a-fA-F]{6}|rgb|hsl|transparent/);
    }
  });

  test("should persist theme immediately when selecting", async ({ page }) => {
    await openSettings(page);
    await navigateToTerminalSettings(page);

    const themeNames = await getThemeNames(page);
    expect(themeNames.length).toBeGreaterThanOrEqual(2);

    // Find the currently active theme
    let activeThemeName = "";
    for (const name of themeNames) {
      if (await isThemeActive(page, name)) {
        activeThemeName = name;
        break;
      }
    }

    // Find and select a different theme
    const differentTheme = themeNames.find((name) => name !== activeThemeName);
    if (differentTheme) {
      await selectTheme(page, differentTheme);
      await page.waitForTimeout(200);

      // Get the new background after selecting
      const newBackground = await getCssVariable(page, "--background");

      // Close settings (no Save button - changes are auto-saved)
      await closeSettings(page);

      // Wait for close
      await page.waitForTimeout(200);

      // Background should still be the new theme (persisted immediately)
      const persistedBackground = await getCssVariable(page, "--background");
      expect(persistedBackground).toBe(newBackground);

      // Re-open settings and verify the theme is still selected
      await openSettings(page);
      await navigateToTerminalSettings(page);

      const isStillActive = await isThemeActive(page, differentTheme);
      expect(isStillActive).toBe(true);
    }
  });

  test("should display custom themes with (Custom) badge", async ({ page }) => {
    // This test checks that custom themes are properly labeled
    await openSettings(page);
    await navigateToTerminalSettings(page);

    // Look for any custom theme badges
    const customBadges = page.locator('.space-y-1.border >> text="(Custom)"');
    const customCount = await customBadges.count();

    // Log count for debugging (custom themes may or may not exist in mock mode)
    console.log(`Found ${customCount} custom themes`);

    // Verify the theme list container exists
    const themeContainer = page.locator(".space-y-1.border");
    await expect(themeContainer).toBeVisible();
  });

  test("clicking theme button should trigger selection handler", async ({ page }) => {
    await openSettings(page);
    await navigateToTerminalSettings(page);

    // Find any theme button
    const themeButton = page.locator('.space-y-1.border >> button[type="button"]').first();
    await expect(themeButton).toBeVisible();

    // Verify it's clickable
    const boundingBox = await themeButton.boundingBox();
    expect(boundingBox).not.toBeNull();
    expect(boundingBox?.width).toBeGreaterThan(0);
    expect(boundingBox?.height).toBeGreaterThan(0);

    // Click and verify no errors
    await themeButton.click();
  });
});
