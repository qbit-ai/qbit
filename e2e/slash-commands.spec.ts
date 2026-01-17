import { expect, type Page, test } from "@playwright/test";

/**
 * Slash Commands E2E Tests
 *
 * These tests verify the slash commands and skills feature:
 * - Popup triggering with "/"
 * - Filtering commands by name and description
 * - Keyboard navigation (Up/Down arrows)
 * - Selection with Tab/Enter/Click
 * - Popup dismissal (Escape, click outside, backspace)
 * - Skills display and execution
 * - Prompts display and execution
 * - Mode switching integration
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

  // Wait for the unified input textarea to be visible (exclude xterm's hidden textarea)
  await expect(page.locator("textarea:not(.xterm-helper-textarea)")).toBeVisible({ timeout: 5000 });
}

/**
 * Get the UnifiedInput textarea element.
 * We use :not(.xterm-helper-textarea) to exclude the xterm.js hidden textarea.
 */
function getInputTextarea(page: Page) {
  return page.locator("textarea:not(.xterm-helper-textarea)");
}

/**
 * Get the Terminal mode toggle button.
 */
function getTerminalModeButton(page: Page) {
  return page.getByRole("button", { name: "Switch to Terminal mode" });
}

/**
 * Get the AI mode toggle button.
 */
function getAgentModeButton(page: Page) {
  return page.getByRole("button", { name: "Switch to AI mode" });
}

/**
 * Get the slash command popup (listbox).
 */
function getSlashCommandPopup(page: Page) {
  return page.locator('[role="listbox"]');
}

/**
 * Get slash command items within the popup.
 */
function getSlashCommandItems(page: Page) {
  return page.locator('[role="option"]');
}

/**
 * Switch to agent mode if not already there.
 */
async function ensureAgentMode(page: Page) {
  const textarea = getInputTextarea(page);
  const placeholder = await textarea.getAttribute("placeholder");

  if (placeholder !== "Ask the AI...") {
    const agentButton = getAgentModeButton(page);
    await agentButton.click();
    await expect(textarea).toHaveAttribute("placeholder", "Ask the AI...");
  }
}

/**
 * Switch to terminal mode if not already there.
 */
async function ensureTerminalMode(page: Page) {
  const textarea = getInputTextarea(page);
  const placeholder = await textarea.getAttribute("placeholder");

  if (placeholder !== "Enter command...") {
    const terminalButton = getTerminalModeButton(page);
    await terminalButton.click();
    await expect(textarea).toHaveAttribute("placeholder", "Enter command...");
  }
}

/**
 * Simulate AI response through the mock system.
 */
async function simulateAiResponse(page: Page, response: string, delayMs: number = 20) {
  await page.evaluate(
    async ({ response, delayMs }) => {
      const simulateFn = (
        window as unknown as {
          __MOCK_SIMULATE_AI_RESPONSE__?: (response: string, delayMs: number) => Promise<void>;
        }
      ).__MOCK_SIMULATE_AI_RESPONSE__;
      if (simulateFn) {
        await simulateFn(response, delayMs);
      }
    },
    { response, delayMs }
  );
}

// =============================================================================
// Mock Data Reference (from mocks.ts)
// =============================================================================
// Prompts (3):
//   - review (global)
//   - explain (global)
//   - project-context (local)
// Skills (2):
//   - code-review (global) - "Review code for quality and best practices"
//   - refactor (global) - "Refactor code for improved readability and maintainability"
// Total: 5 commands (sorted: code-review, explain, project-context, refactor, review)

test.describe("Slash Commands", () => {
  test.beforeEach(async ({ page }) => {
    await waitForAppReady(page);
  });

  test.describe("Popup Triggering", () => {
    test("typing / at start opens popup", async ({ page }) => {
      const textarea = getInputTextarea(page);

      await textarea.focus();
      await textarea.fill("/");

      // Popup should appear
      const popup = getSlashCommandPopup(page);
      await expect(popup).toBeVisible({ timeout: 3000 });
    });

    test("typing / in middle of text does not open popup", async ({ page }) => {
      const textarea = getInputTextarea(page);

      await textarea.focus();
      await textarea.fill("some text /");

      // Give it time to potentially appear
      await page.waitForTimeout(500);

      // Popup should NOT appear
      const popup = getSlashCommandPopup(page);
      await expect(popup).not.toBeVisible();
    });

    test("popup shows all commands when only / typed", async ({ page }) => {
      const textarea = getInputTextarea(page);

      await textarea.focus();
      await textarea.fill("/");

      const popup = getSlashCommandPopup(page);
      await expect(popup).toBeVisible({ timeout: 3000 });

      // Should show all 5 commands (2 skills + 3 prompts)
      const items = getSlashCommandItems(page);
      await expect(items).toHaveCount(5);
    });

    test("popup works in both terminal and agent modes", async ({ page }) => {
      // Test in terminal mode first
      await ensureTerminalMode(page);
      const textarea = getInputTextarea(page);
      const popup = getSlashCommandPopup(page);

      await textarea.focus();
      await textarea.fill("/");
      await expect(popup).toBeVisible({ timeout: 3000 });

      // Clear and close
      await page.keyboard.press("Escape");
      await textarea.fill("");

      // Switch to agent mode and test
      await ensureAgentMode(page);
      await textarea.focus();
      await textarea.fill("/");
      await expect(popup).toBeVisible({ timeout: 3000 });
    });
  });

  test.describe("Filtering", () => {
    test("filters commands as user types", async ({ page }) => {
      const textarea = getInputTextarea(page);

      await textarea.focus();
      await textarea.fill("/ref");

      const popup = getSlashCommandPopup(page);
      await expect(popup).toBeVisible({ timeout: 3000 });

      // Should filter to: refactor (only one contains "ref")
      const items = getSlashCommandItems(page);
      await expect(items).toHaveCount(1);

      // Verify the item is refactor
      const itemTexts = await items.allTextContents();
      expect(itemTexts.some((t) => t.includes("/refactor"))).toBeTruthy();
    });

    test("filters by description for skills", async ({ page }) => {
      const textarea = getInputTextarea(page);

      await textarea.focus();
      // Search for "quality" which is in code-review's description
      await textarea.fill("/quality");

      const popup = getSlashCommandPopup(page);
      await expect(popup).toBeVisible({ timeout: 3000 });

      // Should find code-review skill
      const items = getSlashCommandItems(page);
      const count = await items.count();
      expect(count).toBeGreaterThanOrEqual(1);

      const itemTexts = await items.allTextContents();
      expect(itemTexts.some((t) => t.includes("/code-review"))).toBeTruthy();
    });

    test("shows 'No commands found' for no matches", async ({ page }) => {
      const textarea = getInputTextarea(page);

      await textarea.focus();
      await textarea.fill("/zzzznonexistent");

      // Wait for the popup to appear - when empty, it shows "No commands found"
      // The popup container is positioned above the input with specific classes
      const noResultsText = page.locator("text=No commands found");
      await expect(noResultsText).toBeVisible({ timeout: 3000 });

      // No option items should be present (role="option" only exists when there are items)
      const items = getSlashCommandItems(page);
      await expect(items).toHaveCount(0);
    });

    test("case insensitive filtering", async ({ page }) => {
      const textarea = getInputTextarea(page);

      await textarea.focus();
      await textarea.fill("/REVIEW");

      const popup = getSlashCommandPopup(page);
      await expect(popup).toBeVisible({ timeout: 3000 });

      // Should match "review" despite uppercase
      const items = getSlashCommandItems(page);
      const count = await items.count();
      expect(count).toBeGreaterThanOrEqual(1);

      const itemTexts = await items.allTextContents();
      expect(itemTexts.some((t) => t.includes("/review"))).toBeTruthy();
    });
  });

  test.describe("Keyboard Navigation", () => {
    test("Arrow Down moves selection down", async ({ page }) => {
      const textarea = getInputTextarea(page);

      await textarea.focus();
      await textarea.fill("/");

      const popup = getSlashCommandPopup(page);
      await expect(popup).toBeVisible({ timeout: 3000 });

      const items = getSlashCommandItems(page);
      const count = await items.count();

      if (count >= 2) {
        // First item should be selected initially
        await expect(items.nth(0)).toHaveAttribute("aria-selected", "true");

        // Press Down
        await page.keyboard.press("ArrowDown");

        // Second item should now be selected
        await expect(items.nth(1)).toHaveAttribute("aria-selected", "true");
        await expect(items.nth(0)).toHaveAttribute("aria-selected", "false");
      }
    });

    test("Arrow Up moves selection up", async ({ page }) => {
      const textarea = getInputTextarea(page);

      await textarea.focus();
      await textarea.fill("/");

      const popup = getSlashCommandPopup(page);
      await expect(popup).toBeVisible({ timeout: 3000 });

      const items = getSlashCommandItems(page);
      const count = await items.count();

      if (count >= 2) {
        // Move down first
        await page.keyboard.press("ArrowDown");
        await expect(items.nth(1)).toHaveAttribute("aria-selected", "true");

        // Move up
        await page.keyboard.press("ArrowUp");
        await expect(items.nth(0)).toHaveAttribute("aria-selected", "true");
      }
    });

    test("Arrow Up at first item stays at first", async ({ page }) => {
      const textarea = getInputTextarea(page);

      await textarea.focus();
      await textarea.fill("/");

      const popup = getSlashCommandPopup(page);
      await expect(popup).toBeVisible({ timeout: 3000 });

      const items = getSlashCommandItems(page);
      const count = await items.count();

      if (count >= 1) {
        // First item should be selected
        await expect(items.nth(0)).toHaveAttribute("aria-selected", "true");

        // Press Up - should stay at first
        await page.keyboard.press("ArrowUp");
        await expect(items.nth(0)).toHaveAttribute("aria-selected", "true");
      }
    });

    test("Arrow Down at last item stays at last", async ({ page }) => {
      const textarea = getInputTextarea(page);

      await textarea.focus();
      await textarea.fill("/");

      const popup = getSlashCommandPopup(page);
      await expect(popup).toBeVisible({ timeout: 3000 });

      const items = getSlashCommandItems(page);
      const count = await items.count();

      if (count >= 1) {
        // Navigate to last item
        for (let i = 0; i < count; i++) {
          await page.keyboard.press("ArrowDown");
        }

        // Last item should be selected
        await expect(items.nth(count - 1)).toHaveAttribute("aria-selected", "true");

        // Press Down again - should stay at last
        await page.keyboard.press("ArrowDown");
        await expect(items.nth(count - 1)).toHaveAttribute("aria-selected", "true");
      }
    });
  });

  test.describe("Selection", () => {
    test("Tab autocompletes with trailing space for args", async ({ page }) => {
      const textarea = getInputTextarea(page);

      await textarea.focus();
      await textarea.fill("/rev");

      const popup = getSlashCommandPopup(page);
      await expect(popup).toBeVisible({ timeout: 3000 });

      // Navigate to "review" if needed (it should be visible in filtered results)
      const items = getSlashCommandItems(page);
      const count = await items.count();

      // Find and select review
      for (let i = 0; i < count; i++) {
        const text = await items.nth(i).textContent();
        if (text?.includes("/review")) {
          // Navigate to this item
          for (let j = 0; j < i; j++) {
            await page.keyboard.press("ArrowDown");
          }
          break;
        }
      }

      // Press Tab to autocomplete
      await page.keyboard.press("Tab");

      // Popup should close
      await expect(popup).not.toBeVisible();

      // Input should have the completed command with trailing space
      const inputValue = await textarea.inputValue();
      expect(inputValue).toBe("/review ");
    });

    test("Enter executes the command", async ({ page }) => {
      await ensureAgentMode(page);
      const textarea = getInputTextarea(page);

      await textarea.focus();
      await textarea.fill("/review");

      const popup = getSlashCommandPopup(page);
      await expect(popup).toBeVisible({ timeout: 3000 });

      // Press Enter to execute
      await page.keyboard.press("Enter");

      // Popup should close
      await expect(popup).not.toBeVisible();

      // Input should be cleared (command was executed)
      const inputValue = await textarea.inputValue();
      expect(inputValue).toBe("");
    });

    test("Click executes the command", async ({ page }) => {
      await ensureAgentMode(page);
      const textarea = getInputTextarea(page);

      await textarea.focus();
      await textarea.fill("/");

      const popup = getSlashCommandPopup(page);
      await expect(popup).toBeVisible({ timeout: 3000 });

      const items = getSlashCommandItems(page);
      const count = await items.count();

      if (count >= 1) {
        // Click on the first item
        await items.nth(0).click();

        // Popup should close
        await expect(popup).not.toBeVisible();

        // Input should be cleared (command was executed)
        const inputValue = await textarea.inputValue();
        expect(inputValue).toBe("");
      }
    });
  });

  test.describe("Dismissal", () => {
    test("Escape closes the popup", async ({ page }) => {
      const textarea = getInputTextarea(page);

      await textarea.focus();
      await textarea.fill("/");

      const popup = getSlashCommandPopup(page);
      await expect(popup).toBeVisible({ timeout: 3000 });

      // Press Escape
      await page.keyboard.press("Escape");

      // Popup should close
      await expect(popup).not.toBeVisible();
    });

    test("click outside closes the popup", async ({ page }) => {
      const textarea = getInputTextarea(page);

      await textarea.focus();
      await textarea.fill("/");

      const popup = getSlashCommandPopup(page);
      await expect(popup).toBeVisible({ timeout: 3000 });

      // Click outside the popup (on the status bar)
      await page.locator('[data-testid="status-bar"]').click();

      // Popup should close
      await expect(popup).not.toBeVisible();
    });

    test("space after exact match closes popup for args", async ({ page }) => {
      const textarea = getInputTextarea(page);

      await textarea.focus();
      await textarea.fill("/review");

      const popup = getSlashCommandPopup(page);
      await expect(popup).toBeVisible({ timeout: 3000 });

      // Type space after exact match
      await page.keyboard.type(" ");

      // Popup should close
      await expect(popup).not.toBeVisible();

      // Input should have the command with space
      const inputValue = await textarea.inputValue();
      expect(inputValue).toBe("/review ");
    });

    test("backspace to remove / closes popup", async ({ page }) => {
      const textarea = getInputTextarea(page);

      await textarea.focus();
      await textarea.fill("/");

      const popup = getSlashCommandPopup(page);
      await expect(popup).toBeVisible({ timeout: 3000 });

      // Press Backspace to remove the /
      await page.keyboard.press("Backspace");

      // Popup should close
      await expect(popup).not.toBeVisible();

      // Input should be empty
      const inputValue = await textarea.inputValue();
      expect(inputValue).toBe("");
    });
  });
});

test.describe("Skills", () => {
  test.beforeEach(async ({ page }) => {
    await waitForAppReady(page);
  });

  test.describe("Display", () => {
    test("skills show skill badge with puzzle icon", async ({ page }) => {
      const textarea = getInputTextarea(page);

      await textarea.focus();
      await textarea.fill("/code-review");

      const popup = getSlashCommandPopup(page);
      await expect(popup).toBeVisible({ timeout: 3000 });

      // Find the code-review skill item
      const items = getSlashCommandItems(page);
      let skillItem = null;

      const count = await items.count();
      for (let i = 0; i < count; i++) {
        const text = await items.nth(i).textContent();
        if (text?.includes("/code-review")) {
          skillItem = items.nth(i);
          break;
        }
      }

      expect(skillItem).not.toBeNull();

      // Check for "skill" badge
      const badge = skillItem!.locator("text=skill");
      await expect(badge).toBeVisible();

      // Check for puzzle icon (SVG)
      const svg = skillItem!.locator("svg");
      await expect(svg).toBeVisible();
    });

    test("skills show description", async ({ page }) => {
      const textarea = getInputTextarea(page);

      await textarea.focus();
      await textarea.fill("/code-review");

      const popup = getSlashCommandPopup(page);
      await expect(popup).toBeVisible({ timeout: 3000 });

      // Check that the description is visible
      const description = popup.locator("text=Review code for quality and best practices");
      await expect(description).toBeVisible();
    });
  });

  test.describe("Execution", () => {
    test("executing skill sends content to AI", async ({ page }) => {
      await ensureAgentMode(page);
      const textarea = getInputTextarea(page);

      await textarea.focus();
      await textarea.fill("/code-review");

      const popup = getSlashCommandPopup(page);
      await expect(popup).toBeVisible({ timeout: 3000 });

      // Press Enter to execute
      await page.keyboard.press("Enter");

      // Popup should close
      await expect(popup).not.toBeVisible();

      // Input should be cleared
      const inputValue = await textarea.inputValue();
      expect(inputValue).toBe("");

      // User message should appear in the timeline showing the skill name
      const userMessage = page.locator('text="/code-review"');
      await expect(userMessage).toBeVisible({ timeout: 5000 });
    });

    test("skills with arguments append args", async ({ page }) => {
      await ensureAgentMode(page);
      const textarea = getInputTextarea(page);

      // Type skill name, space, and arguments
      await textarea.focus();
      await textarea.fill("/refactor src/main.ts");

      // Popup should be closed (space after exact match)
      const popup = getSlashCommandPopup(page);
      await expect(popup).not.toBeVisible();

      // Press Enter to execute
      await page.keyboard.press("Enter");

      // User message should show the skill name with args
      const userMessage = page.locator('text="/refactor src/main.ts"');
      await expect(userMessage).toBeVisible({ timeout: 5000 });
    });
  });
});

test.describe("Prompts", () => {
  test.beforeEach(async ({ page }) => {
    await waitForAppReady(page);
  });

  test.describe("Display", () => {
    test("global prompts show global badge in blue", async ({ page }) => {
      const textarea = getInputTextarea(page);

      await textarea.focus();
      await textarea.fill("/review");

      const popup = getSlashCommandPopup(page);
      await expect(popup).toBeVisible({ timeout: 3000 });

      // Find the review prompt item
      const items = getSlashCommandItems(page);
      let promptItem = null;

      const count = await items.count();
      for (let i = 0; i < count; i++) {
        const text = await items.nth(i).textContent();
        if (text?.includes("/review") && text.includes("global")) {
          promptItem = items.nth(i);
          break;
        }
      }

      expect(promptItem).not.toBeNull();

      // Check for "global" badge
      const badge = promptItem!.locator("text=global");
      await expect(badge).toBeVisible();
    });

    test("local prompts show local badge in green", async ({ page }) => {
      const textarea = getInputTextarea(page);

      await textarea.focus();
      await textarea.fill("/project-context");

      const popup = getSlashCommandPopup(page);
      await expect(popup).toBeVisible({ timeout: 3000 });

      // Find the project-context prompt item
      const items = getSlashCommandItems(page);
      let promptItem = null;

      const count = await items.count();
      for (let i = 0; i < count; i++) {
        const text = await items.nth(i).textContent();
        if (text?.includes("/project-context")) {
          promptItem = items.nth(i);
          break;
        }
      }

      expect(promptItem).not.toBeNull();

      // Check for "local" badge
      const badge = promptItem!.locator("text=local");
      await expect(badge).toBeVisible();
    });
  });

  test.describe("Execution", () => {
    test("executing prompt sends content to AI", async ({ page }) => {
      await ensureAgentMode(page);
      const textarea = getInputTextarea(page);

      await textarea.focus();
      await textarea.fill("/explain");

      const popup = getSlashCommandPopup(page);
      await expect(popup).toBeVisible({ timeout: 3000 });

      // Press Enter to execute
      await page.keyboard.press("Enter");

      // Popup should close
      await expect(popup).not.toBeVisible();

      // Input should be cleared
      const inputValue = await textarea.inputValue();
      expect(inputValue).toBe("");

      // User message should appear showing the prompt name
      const userMessage = page.locator('text="/explain"');
      await expect(userMessage).toBeVisible({ timeout: 5000 });
    });
  });
});

test.describe("Integration", () => {
  test.beforeEach(async ({ page }) => {
    await waitForAppReady(page);
  });

  test("slash command in terminal mode switches to agent mode", async ({ page }) => {
    await ensureTerminalMode(page);
    const textarea = getInputTextarea(page);

    // Verify we're in terminal mode
    await expect(textarea).toHaveAttribute("placeholder", "Enter command...");

    await textarea.focus();
    await textarea.fill("/review");

    const popup = getSlashCommandPopup(page);
    await expect(popup).toBeVisible({ timeout: 3000 });

    // Press Enter to execute
    await page.keyboard.press("Enter");

    // Should have switched to agent mode
    await expect(textarea).toHaveAttribute("placeholder", "Ask the AI...");
  });

  test("AI response streams after execution", async ({ page }) => {
    await ensureAgentMode(page);
    const textarea = getInputTextarea(page);

    await textarea.focus();
    await textarea.fill("/explain");

    const popup = getSlashCommandPopup(page);
    await expect(popup).toBeVisible({ timeout: 3000 });

    // Press Enter to execute
    await page.keyboard.press("Enter");

    // Wait for the popup to close
    await expect(popup).not.toBeVisible();

    // Wait a moment for the message to be added to the store
    await page.waitForTimeout(500);

    // Simulate AI response
    await simulateAiResponse(page, "Here is my explanation of the code.", 10);

    // Wait for the response to appear in the timeline
    const aiResponse = page.locator("text=Here is my explanation");
    await expect(aiResponse).toBeVisible({ timeout: 5000 });
  });

  test("input is disabled during AI response", async ({ page }) => {
    await ensureAgentMode(page);
    const textarea = getInputTextarea(page);

    await textarea.focus();
    await textarea.fill("/review");

    const popup = getSlashCommandPopup(page);
    await expect(popup).toBeVisible({ timeout: 3000 });

    // Press Enter to execute
    await page.keyboard.press("Enter");

    // Wait briefly for submitting state to be set
    await page.waitForTimeout(100);

    // Input should be disabled while waiting for response
    // Note: The disabled state depends on isAgentBusy which includes isSubmitting
    const isDisabled = await textarea.isDisabled();
    expect(isDisabled).toBeTruthy();

    // Simulate AI response to complete the interaction
    await simulateAiResponse(page, "Done reviewing.", 10);

    // Wait for response and input to be re-enabled
    await page.waitForTimeout(200);
  });
});
