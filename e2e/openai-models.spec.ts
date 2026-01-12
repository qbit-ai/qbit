import { expect, type Page, test } from "@playwright/test";

async function waitForAppReady(page: Page) {
  await page.goto("/");
  await page.waitForLoadState("domcontentloaded");
  await page.waitForFunction(
    () => (window as unknown as { __MOCK_BROWSER_MODE__?: boolean }).__MOCK_BROWSER_MODE__ === true,
    { timeout: 15000 }
  );
  await expect(page.locator('[data-testid="status-bar"]')).toBeVisible({ timeout: 10000 });
}

async function switchToAgentMode(page: Page) {
  const switchToAi = page.getByRole("button", { name: "Switch to AI mode" });
  const isSwitchToAiVisible = await switchToAi.isVisible().catch(() => false);
  if (isSwitchToAiVisible) {
    await switchToAi.click();
    return;
  }

  await expect(page.getByRole("button", { name: "Switch to Terminal mode" })).toBeVisible({
    timeout: 5000,
  });
}

async function openModelSelector(page: Page) {
  const modelSelector = page
    .locator("button")
    .filter({ hasText: /Claude|Devstral|GPT|Llama|o1|o3|o4|Codex/ });
  await expect(modelSelector.first()).toBeVisible({ timeout: 5000 });
  await modelSelector.first().click();
  await expect(page.locator("[role='menu']")).toBeVisible({ timeout: 3000 });
}

async function openSubMenu(page: Page, triggerText: string) {
  const trigger = page
    .locator('[data-slot="dropdown-menu-sub-trigger"], [role="menuitem"]')
    .filter({ hasText: triggerText })
    .first();
  await expect(trigger).toBeVisible({ timeout: 3000 });
  await trigger.hover();
  await page.waitForTimeout(150);
  await trigger.click();
  await page.waitForTimeout(200);
}

test.describe("OpenAI Models - Model Selector", () => {
  test.beforeEach(async ({ page }) => {
    await waitForAppReady(page);
  });

  test("OpenAI provider appears in the model selector", async ({ page }) => {
    await switchToAgentMode(page);
    await openModelSelector(page);
    await expect(page.locator("[role='menu'] >> text=OpenAI")).toBeVisible();
    await page.keyboard.press("Escape");
  });

  test("can select Codex Mini from nested menu", async ({ page }) => {
    await switchToAgentMode(page);
    await openModelSelector(page);
    await openSubMenu(page, "Codex");

    const menuItem = page.getByRole("menuitem").filter({ hasText: "Codex Mini" });
    await menuItem.first().click();

    const modelSelector = page.locator("button").filter({ hasText: /Codex Mini/ });
    await expect(modelSelector.first()).toBeVisible({ timeout: 3000 });
  });
});
