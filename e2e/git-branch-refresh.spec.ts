import { expect, type Page, test } from "@playwright/test";

async function waitForAppReady(page: Page) {
  await page.goto("/");
  await page.waitForLoadState("domcontentloaded");

  await page.waitForFunction(
    () => (window as unknown as { __MOCK_BROWSER_MODE__?: boolean }).__MOCK_BROWSER_MODE__ === true,
    { timeout: 15000 }
  );
}

test.describe("Git branch refresh", () => {
  test("updates git badge after branch-changing command even if command_end has null command", async ({
    page,
  }) => {
    await waitForAppReady(page);

    const gitBadge = page.locator('button[title="Toggle Git Panel"]');

    // Initial state in mock mode should show main.
    await expect(gitBadge).toBeVisible({ timeout: 15000 });
    await expect(gitBadge).toContainText("main", { timeout: 15000 });

    await page.evaluate(() => {
      const w = window as unknown as {
        __MOCK_SET_GIT_STATE__?: (next: {
          branch?: string | null;
          insertions?: number;
          deletions?: number;
          ahead?: number;
          behind?: number;
        }) => void;
        __MOCK_EMIT_COMMAND_BLOCK_EVENT__?: (
          sessionId: string,
          eventType: "prompt_start" | "prompt_end" | "command_start" | "command_end",
          command?: string | null,
          exitCode?: number | null
        ) => void;
      };

      // Update mock git state so get_git_branch/git_status reflect the new branch.
      w.__MOCK_SET_GIT_STATE__?.({ branch: "hiii", insertions: 112, deletions: 111 });

      // Simulate a branch-changing command where command_end omits the command.
      // This matches the real-world case where the PTY integration sometimes sends null.
      w.__MOCK_EMIT_COMMAND_BLOCK_EVENT__?.(
        "mock-session-001",
        "command_start",
        "git checkout -b hiii",
        null
      );
      w.__MOCK_EMIT_COMMAND_BLOCK_EVENT__?.("mock-session-001", "command_end", null, 0);
    });

    // Badge should refresh from main -> hiii.
    await expect(gitBadge).toContainText("hiii", { timeout: 15000 });
  });
});
