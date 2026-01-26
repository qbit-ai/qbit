import { expect, type Page, test } from "@playwright/test";
import { waitForAppReady as waitForAppReadyBase } from "./helpers/app";

async function waitForAppReady(page: Page) {
  await waitForAppReadyBase(page);
  await expect(page.locator('[data-testid="unified-input"]')).toBeVisible({ timeout: 5000 });
}

function getInputTextarea(page: Page) {
  return page.locator('[data-testid="unified-input"]');
}

function getAgentModeButton(page: Page) {
  return page.getByRole("button", { name: "Switch to AI mode" });
}

test.describe("AI Prompt Submission", () => {
  test.beforeEach(async ({ page }) => {
    await waitForAppReady(page);
  });

  test("should not show AI error notification on prompt submission", async ({ page }) => {
    const agentButton = getAgentModeButton(page);
    await agentButton.click();

    const textarea = getInputTextarea(page);
    await expect(textarea).toHaveAttribute("data-mode", "agent", { timeout: 3000 });

    await textarea.fill("Test prompt for error checking");
    await page.keyboard.press("Enter");

    await page.waitForTimeout(1000);

    const errorNotification = page.locator('text="Agent error"');
    await expect(errorNotification).not.toBeVisible();
  });
});
