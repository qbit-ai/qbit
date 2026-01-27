import { expect, type Page } from "@playwright/test";

/**
 * Open the settings tab by using the keyboard shortcut Meta+, (Cmd+,).
 * This triggers the app's native settings handler.
 *
 * Note: Settings-related E2E tests are currently flaky due to React re-rendering
 * issues in browser mock mode. The Settings tab content causes continuous
 * re-renders that detach elements from the DOM.
 */
export async function openSettings(page: Page) {
  // Dismiss command palette if open
  const commandPaletteHeading = page.getByRole("heading", { name: "Command Palette" });
  if (await commandPaletteHeading.isVisible().catch(() => false)) {
    await page.keyboard.press("Escape");
    await page.waitForTimeout(100);
  }

  // Focus the body to ensure keyboard shortcuts work
  await page.locator("body").click({ position: { x: 10, y: 10 } });
  await page.waitForTimeout(50);

  // Use the keyboard shortcut to open settings
  await page.keyboard.press("Meta+,");

  // Wait for settings tab to appear - look for the Providers nav button
  await expect(page.locator("nav >> button:has-text('Providers')")).toBeVisible({ timeout: 10000 });
}

/**
 * Wait for the app to be fully ready in mock browser mode.
 *
 * NOTE: The UI no longer exposes `data-testid="status-bar"`, so use stable
 * user-visible controls as the readiness signal instead.
 */
export async function waitForAppReady(page: Page) {
  await page.goto("/");
  await page.waitForLoadState("domcontentloaded");

  await page.waitForFunction(
    () => (window as unknown as { __MOCK_BROWSER_MODE__?: boolean }).__MOCK_BROWSER_MODE__ === true,
    { timeout: 15000 }
  );

  const terminalMode = page.getByRole("button", { name: "Switch to Terminal mode" });
  const aiMode = page.getByRole("button", { name: "Switch to AI mode" });
  await expect(async () => {
    const terminalVisible = await terminalMode.isVisible().catch(() => false);
    const aiVisible = await aiMode.isVisible().catch(() => false);
    expect(terminalVisible || aiVisible).toBe(true);
  }).toPass({ timeout: 15000 });

  // In some builds the Command Palette can be open/docked by default.
  // Dismiss any active overlay and restore focus so keyboard shortcuts work.
  const commandPaletteHeading = page.getByRole("heading", { name: "Command Palette" });
  if (await commandPaletteHeading.isVisible().catch(() => false)) {
    await page.keyboard.press("Escape");
    const unifiedInput = page.locator('[data-testid="unified-input"]');
    if (await unifiedInput.isVisible().catch(() => false)) {
      await unifiedInput.click();
    } else {
      await page.locator("body").click({ position: { x: 10, y: 10 } });
    }
  }
}
