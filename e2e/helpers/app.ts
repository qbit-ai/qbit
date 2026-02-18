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

  // Wait for the mode toggle buttons to appear (indicates the UI is rendered)
  // Use stable title attributes that don't change with mode state
  const terminalBtn = page.locator('button[title="Terminal"]');
  const aiBtn = page.locator('button[title="AI"]');
  await expect(async () => {
    const terminalVisible = await terminalBtn.isVisible().catch(() => false);
    const aiVisible = await aiBtn.isVisible().catch(() => false);
    expect(terminalVisible || aiVisible).toBe(true);
  }).toPass({ timeout: 15000 });

  // Wait longer for the app to stabilize (Home tab + Terminal tab creation causes re-renders)
  await page.waitForTimeout(500);

  // Wait for the unified input to be visible and stable
  // Use polling to ensure the DOM has settled
  await expect(async () => {
    const input = page.locator('[data-testid="unified-input"]:visible').first();
    await expect(input).toBeVisible();
    await expect(input).toBeEnabled();
  }).toPass({ timeout: 10000, intervals: [100, 200, 500] });

  // In some builds the Command Palette can be open/docked by default.
  // Dismiss any active overlay with Escape via evaluate (to avoid locator stability issues).
  await page.evaluate(() => {
    const heading = document.querySelector('h2, h3, [role="heading"]');
    if (heading?.textContent?.includes("Command Palette")) {
      document.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    }
  });
  await page.waitForTimeout(300);
}

/**
 * Get the UnifiedInput textarea from the active pane.
 * Uses :visible to find the textarea in the currently active tab.
 */
export function getActiveInput(page: Page) {
  return page.locator('[data-testid="unified-input"]:visible').first();
}
