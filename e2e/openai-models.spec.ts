import { expect, type Page, test } from "@playwright/test";
import { waitForAppReady } from "./helpers/app";

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
    // Navigate: Codex -> GPT 5.1 Codex Mini -> Low
    await openSubMenu(page, "Codex");
    await openSubMenu(page, "GPT 5.1 Codex Mini");

    const menuItem = page.getByRole("menuitem").filter({ hasText: "Low" });
    await menuItem.first().click();

    const modelSelector = page
      .locator("button")
      .filter({ hasText: /GPT 5\.1 Codex Mini.*Low/ });
    await expect(modelSelector.first()).toBeVisible({ timeout: 3000 });
  });
});
