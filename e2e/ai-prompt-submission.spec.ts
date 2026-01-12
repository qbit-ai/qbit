import { expect, type Page, test } from "@playwright/test";

async function waitForAppReady(page: Page) {
  await page.goto("/");
  await page.waitForLoadState("domcontentloaded");
  await page.waitForFunction(
    () => (window as unknown as { __MOCK_BROWSER_MODE__?: boolean }).__MOCK_BROWSER_MODE__ === true,
    { timeout: 15000 }
  );
  await expect(page.locator('[data-testid="status-bar"]')).toBeVisible({ timeout: 10000 });
  await expect(page.locator("textarea:not(.xterm-helper-textarea)")).toBeVisible({ timeout: 5000 });
}

function getInputTextarea(page: Page) {
  return page.locator("textarea:not(.xterm-helper-textarea)");
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
    await expect(textarea).toHaveAttribute("placeholder", "Ask the AI...", { timeout: 3000 });

    await textarea.fill("Test prompt for error checking");
    await page.keyboard.press("Enter");

    await page.waitForTimeout(1000);

    const errorNotification = page.locator('text="Agent error"');
    await expect(errorNotification).not.toBeVisible();
  });
});
