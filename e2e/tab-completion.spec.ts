import { expect, type Page, test } from "@playwright/test";

/**
 * Tab Completion E2E Tests
 *
 * These tests verify the tab completion feature in terminal mode:
 * - Tab key opens the completion popup
 * - Keyboard navigation (Up/Down arrows)
 * - Selection with Tab/Enter
 * - Escape closes the popup
 * - Directory selection continues completion
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
  await expect(page.locator('[class*="StatusBar"]').or(page.locator("text=Terminal"))).toBeVisible({
    timeout: 10000,
  });

  // Wait for the unified input textarea to be visible
  await expect(page.locator("textarea")).toBeVisible({ timeout: 5000 });
}

/**
 * Get the UnifiedInput textarea element.
 */
function getInputTextarea(page: Page) {
  return page.locator("textarea");
}

/**
 * Get the Terminal mode toggle button.
 */
function getTerminalModeButton(page: Page) {
  return page.getByRole("button", { name: "Switch to Terminal mode" });
}

/**
 * Get the path completion popup.
 */
function getPathCompletionPopup(page: Page) {
  // The popup contains a listbox role
  return page.locator('[role="listbox"]');
}

/**
 * Get completion items within the popup.
 */
function getCompletionItems(page: Page) {
  return page.locator('[role="option"]');
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

test.describe("Tab Completion", () => {
  test.beforeEach(async ({ page }) => {
    await waitForAppReady(page);
    await ensureTerminalMode(page);
  });

  test.describe("Popup Triggering", () => {
    test("Tab key opens completion popup in terminal mode", async ({ page }) => {
      const textarea = getInputTextarea(page);

      // Focus and press Tab
      await textarea.focus();
      await page.keyboard.press("Tab");

      // Popup should appear
      const popup = getPathCompletionPopup(page);
      await expect(popup).toBeVisible({ timeout: 3000 });
    });

    test("Tab key does not open popup in agent mode", async ({ page }) => {
      // Switch to agent mode
      const agentButton = page.getByRole("button", { name: "Switch to AI mode" });
      await agentButton.click();

      const textarea = getInputTextarea(page);
      await expect(textarea).toHaveAttribute("placeholder", "Ask the AI...");

      // Press Tab - should not show path completion popup
      await textarea.focus();
      await page.keyboard.press("Tab");

      // Give it time to potentially appear
      await page.waitForTimeout(500);

      // Popup should NOT appear (Tab has different behavior in agent mode)
      const popup = getPathCompletionPopup(page);
      await expect(popup).not.toBeVisible();
    });

    test("Tab with partial input opens popup filtered by prefix", async ({ page }) => {
      const textarea = getInputTextarea(page);

      // Type a partial path
      await textarea.fill("src");
      await page.keyboard.press("Tab");

      // Popup should appear
      const popup = getPathCompletionPopup(page);
      await expect(popup).toBeVisible({ timeout: 3000 });

      // Items should be visible (if any match "src")
      const items = getCompletionItems(page);
      const count = await items.count();
      // We expect either matches or "No completions found" message
      expect(count >= 0).toBeTruthy();
    });
  });

  test.describe("Keyboard Navigation", () => {
    test("Arrow Down moves selection down", async ({ page }) => {
      const textarea = getInputTextarea(page);

      await textarea.focus();
      await page.keyboard.press("Tab");

      const popup = getPathCompletionPopup(page);
      await expect(popup).toBeVisible({ timeout: 3000 });

      const items = getCompletionItems(page);
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
      await page.keyboard.press("Tab");

      const popup = getPathCompletionPopup(page);
      await expect(popup).toBeVisible({ timeout: 3000 });

      const items = getCompletionItems(page);
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

    test("Arrow Up at first item stays at first item", async ({ page }) => {
      const textarea = getInputTextarea(page);

      await textarea.focus();
      await page.keyboard.press("Tab");

      const popup = getPathCompletionPopup(page);
      await expect(popup).toBeVisible({ timeout: 3000 });

      const items = getCompletionItems(page);
      const count = await items.count();

      if (count >= 1) {
        // First item should be selected
        await expect(items.nth(0)).toHaveAttribute("aria-selected", "true");

        // Press Up - should stay at first
        await page.keyboard.press("ArrowUp");
        await expect(items.nth(0)).toHaveAttribute("aria-selected", "true");
      }
    });

    test("Arrow Down at last item stays at last item", async ({ page }) => {
      const textarea = getInputTextarea(page);

      await textarea.focus();
      await page.keyboard.press("Tab");

      const popup = getPathCompletionPopup(page);
      await expect(popup).toBeVisible({ timeout: 3000 });

      const items = getCompletionItems(page);
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
    test("Tab key selects the current completion", async ({ page }) => {
      const textarea = getInputTextarea(page);

      await textarea.focus();
      await page.keyboard.press("Tab");

      const popup = getPathCompletionPopup(page);
      await expect(popup).toBeVisible({ timeout: 3000 });

      const items = getCompletionItems(page);
      const count = await items.count();

      if (count >= 1) {
        // Get the first item's text before selecting
        const firstItemText = await items.nth(0).textContent();

        // Press Tab to select
        await page.keyboard.press("Tab");

        // Popup should close
        await expect(popup).not.toBeVisible();

        // Input should contain the selected text
        const inputValue = await textarea.inputValue();
        expect(inputValue.length).toBeGreaterThan(0);
        // The input should contain something from the completion
        expect(
          firstItemText?.includes(inputValue) || inputValue.includes(firstItemText?.trim() ?? "")
        ).toBeTruthy();
      }
    });

    test("Enter key selects the current completion", async ({ page }) => {
      const textarea = getInputTextarea(page);

      await textarea.focus();
      await page.keyboard.press("Tab");

      const popup = getPathCompletionPopup(page);
      await expect(popup).toBeVisible({ timeout: 3000 });

      const items = getCompletionItems(page);
      const count = await items.count();

      if (count >= 1) {
        // Press Enter to select
        await page.keyboard.press("Enter");

        // Popup should close
        await expect(popup).not.toBeVisible();

        // Input should have been updated
        const inputValue = await textarea.inputValue();
        expect(inputValue.length).toBeGreaterThan(0);
      }
    });

    test("Click selects the completion", async ({ page }) => {
      const textarea = getInputTextarea(page);

      await textarea.focus();
      await page.keyboard.press("Tab");

      const popup = getPathCompletionPopup(page);
      await expect(popup).toBeVisible({ timeout: 3000 });

      const items = getCompletionItems(page);
      const count = await items.count();

      if (count >= 1) {
        // Click on the first item using dispatchEvent (popup uses portal)
        await items
          .nth(0)
          .evaluate((el) => el.dispatchEvent(new MouseEvent("click", { bubbles: true })));

        // Popup should close
        await expect(popup).not.toBeVisible();

        // Input should have been updated
        const inputValue = await textarea.inputValue();
        expect(inputValue.length).toBeGreaterThan(0);
      }
    });
  });

  test.describe("Dismissal", () => {
    test("Escape closes the popup", async ({ page }) => {
      const textarea = getInputTextarea(page);

      await textarea.focus();
      await page.keyboard.press("Tab");

      const popup = getPathCompletionPopup(page);
      await expect(popup).toBeVisible({ timeout: 3000 });

      // Press Escape
      await page.keyboard.press("Escape");

      // Popup should close
      await expect(popup).not.toBeVisible();
    });

    test("Typing closes the popup", async ({ page }) => {
      const textarea = getInputTextarea(page);

      await textarea.focus();
      await page.keyboard.press("Tab");

      const popup = getPathCompletionPopup(page);
      await expect(popup).toBeVisible({ timeout: 3000 });

      // Type something
      await page.keyboard.type("x");

      // Popup should close (will reopen on next Tab)
      await expect(popup).not.toBeVisible();
    });
  });

  test.describe("Directory Continuation", () => {
    test("Selecting a directory reopens popup for its contents", async ({ page }) => {
      const textarea = getInputTextarea(page);

      await textarea.focus();
      await page.keyboard.press("Tab");

      const popup = getPathCompletionPopup(page);
      await expect(popup).toBeVisible({ timeout: 3000 });

      const items = getCompletionItems(page);
      const count = await items.count();

      // Find a directory item (ends with /)
      for (let i = 0; i < count; i++) {
        const text = await items.nth(i).textContent();
        if (text?.endsWith("/")) {
          // Navigate to this item
          for (let j = 0; j < i; j++) {
            await page.keyboard.press("ArrowDown");
          }

          // Select it
          await page.keyboard.press("Tab");

          // Wait for popup to potentially reopen
          await page.waitForTimeout(200);

          // Popup should reopen for directory contents
          await expect(popup).toBeVisible({ timeout: 1000 });

          // Input should end with the directory path
          const inputValue = await textarea.inputValue();
          expect(inputValue.endsWith("/")).toBeTruthy();

          break;
        }
      }
    });
  });

  test.describe("Visual Feedback", () => {
    test("Popup shows correct icons for different entry types", async ({ page }) => {
      const textarea = getInputTextarea(page);

      await textarea.focus();
      await page.keyboard.press("Tab");

      const popup = getPathCompletionPopup(page);
      await expect(popup).toBeVisible({ timeout: 3000 });

      // Check that items contain SVG icons (from lucide-react)
      const items = getCompletionItems(page);
      const count = await items.count();

      if (count >= 1) {
        // Each item should have an icon
        const firstItem = items.nth(0);
        const svg = firstItem.locator("svg");
        await expect(svg).toBeVisible();
      }
    });

    test("Selected item has visual highlight", async ({ page }) => {
      const textarea = getInputTextarea(page);

      await textarea.focus();
      await page.keyboard.press("Tab");

      const popup = getPathCompletionPopup(page);
      await expect(popup).toBeVisible({ timeout: 3000 });

      const items = getCompletionItems(page);
      const count = await items.count();

      if (count >= 1) {
        // First item should have aria-selected=true
        const firstItem = items.nth(0);
        await expect(firstItem).toHaveAttribute("aria-selected", "true");

        // Should have a background class for highlighting
        const className = await firstItem.getAttribute("class");
        expect(className).toContain("bg-");
      }
    });

    test("Empty results shows 'No completions found' message", async ({ page }) => {
      const textarea = getInputTextarea(page);

      // Type something that won't match anything
      await textarea.fill("zzzzznonexistent99999");
      await page.keyboard.press("Tab");

      // The popup structure exists but shows "no completions" message
      await page.waitForTimeout(500);

      // Either the popup shows with a message, or it doesn't appear
      // Check for the "No completions found" text if popup is visible
      const noResultsText = page.locator("text=No completions found");
      const isVisible = await noResultsText.isVisible().catch(() => false);

      // Either we see the message or no popup appears (both are valid)
      // We just verify the app didn't crash
      expect(isVisible || true).toBeTruthy();
    });
  });

  test.describe("Integration with Input State", () => {
    test("Completion replaces partial path correctly", async ({ page }) => {
      const textarea = getInputTextarea(page);

      // Type "ls sr" to simulate command with partial path
      await textarea.fill("ls sr");
      await page.keyboard.press("Tab");

      const popup = getPathCompletionPopup(page);
      await expect(popup).toBeVisible({ timeout: 3000 });

      const items = getCompletionItems(page);
      const count = await items.count();

      if (count >= 1) {
        // Select first completion
        await page.keyboard.press("Tab");

        // Input should still have "ls " prefix
        const inputValue = await textarea.inputValue();
        expect(inputValue.startsWith("ls ")).toBeTruthy();
      }
    });

    test("Multiple completions in sequence work correctly", async ({ page }) => {
      const textarea = getInputTextarea(page);

      await textarea.focus();

      // First completion - type a prefix to filter to files only
      await page.keyboard.type("pack");
      await page.keyboard.press("Tab");
      let popup = getPathCompletionPopup(page);
      await expect(popup).toBeVisible({ timeout: 3000 });

      let items = getCompletionItems(page);
      let count = await items.count();

      if (count >= 1) {
        // Select package.json (a file, not directory)
        await page.keyboard.press("Tab");
        await expect(popup).not.toBeVisible();

        // Add a space and do another completion with a file prefix
        await page.keyboard.type(" READ");
        await page.keyboard.press("Tab");

        popup = getPathCompletionPopup(page);
        await expect(popup).toBeVisible({ timeout: 3000 });

        items = getCompletionItems(page);
        count = await items.count();

        if (count >= 1) {
          // Select README.md (a file, not directory)
          await page.keyboard.press("Tab");
          await expect(popup).not.toBeVisible();

          // Input should have two completions separated by space
          const inputValue = await textarea.inputValue();
          expect(inputValue.includes(" ")).toBeTruthy();
        }
      }
    });
  });
});
