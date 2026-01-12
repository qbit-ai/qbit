import { expect, type Page, test } from "@playwright/test";

async function waitForAppReady(page: Page) {
  await page.goto("/");
  await page.waitForLoadState("domcontentloaded");
  await page.waitForFunction(
    () => (window as unknown as { __MOCK_BROWSER_MODE__?: boolean }).__MOCK_BROWSER_MODE__ === true,
    { timeout: 15000 }
  );
  const toggleButton = page.locator('button[title="Toggle Mock Dev Tools"]');
  await expect(toggleButton).toBeVisible({ timeout: 10000 });
}

test.describe("MockDevTools - Preset UI Verification", () => {
  test.beforeEach(async ({ page }) => {
    await waitForAppReady(page);
  });

  test("Tool Execution preset displays tool request and result", async ({ page }) => {
    await page.locator('button[title="Toggle Mock Dev Tools"]').click();
    await page.locator("text=Tool Execution").click();
    await expect(page.locator("text=read the configuration file")).toBeVisible({ timeout: 15000 });
    await expect(page.locator("text=read_file")).toBeVisible({ timeout: 5000 });
    await expect(page.locator("text=Rust 2021 edition")).toBeVisible({ timeout: 5000 });
  });
});

test.describe("MockDevTools - Terminal Tab UI Verification", () => {
  test.beforeEach(async ({ page }) => {
    await waitForAppReady(page);
    await page.locator('button[title="Toggle Mock Dev Tools"]').click();
    await expect(page.getByRole("heading", { name: "Mock Dev Tools" })).toBeVisible();
    await page.getByRole("button", { name: "Terminal", exact: true }).click();
  });

  test("Emit Command Block displays command with output in timeline", async ({ page }) => {
    const panel = page.locator('[data-testid="mock-devtools-panel"]');
    const commandInput = panel.locator('input[type="text"]').nth(1);
    await commandInput.fill("echo 'test command'");
    const outputTextarea = panel.locator("textarea").last();
    await outputTextarea.fill("Command output result");
    await panel.locator("button:has-text('Emit Command Block')").click();
    await expect(page.locator("text=echo 'test command'").first()).toBeVisible({ timeout: 10000 });
    await expect(page.locator("text=Command output result").first()).toBeVisible({ timeout: 5000 });
  });
});

test.describe("MockDevTools - AI Tab UI Verification", () => {
  test.beforeEach(async ({ page }) => {
    await waitForAppReady(page);
    await page.locator('button[title="Toggle Mock Dev Tools"]').click();
    await expect(page.getByRole("heading", { name: "Mock Dev Tools" })).toBeVisible();
    await page.getByRole("button", { name: "AI", exact: true }).click();
  });

  test("Simulate Response displays streamed AI text in timeline", async ({ page }) => {
    const panel = page.locator('[data-testid="mock-devtools-panel"]');
    const customResponse = "This is a custom AI response for testing the streaming feature.";
    await panel.locator("textarea").first().fill(customResponse);
    await panel.locator('input[type="number"]').first().fill("10");
    await panel.locator("button:has-text('Simulate Response')").click();
    await expect(page.locator("text=custom AI response for testing")).toBeVisible({
      timeout: 15000,
    });
  });
});
