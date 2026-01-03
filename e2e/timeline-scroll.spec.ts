import { expect, test } from "@playwright/test";

/**
 * Get the active session ID from the store.
 */
async function getActiveSessionId(page: import("@playwright/test").Page): Promise<string | null> {
  return await page.evaluate(() => {
    const store = (
      window as unknown as {
        __QBIT_STORE__?: {
          getState: () => { activeSessionId: string | null };
        };
      }
    ).__QBIT_STORE__;
    return store?.getState().activeSessionId ?? null;
  });
}

test.describe("Timeline Auto-Scroll", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    await page.waitForLoadState("domcontentloaded");

    // Wait for the mock browser mode to be ready
    await page.waitForFunction(
      () =>
        (window as unknown as { __MOCK_BROWSER_MODE__?: boolean }).__MOCK_BROWSER_MODE__ === true,
      { timeout: 10000 }
    );

    // Wait for the store to be available and have an active session
    await page.waitForFunction(
      () => {
        const store = (
          window as unknown as {
            __QBIT_STORE__?: {
              getState: () => { activeSessionId: string | null };
            };
          }
        ).__QBIT_STORE__;
        return store?.getState().activeSessionId != null;
      },
      { timeout: 10000 }
    );
  });

  test("should scroll to bottom when command completes", async ({ page }) => {
    // Get the session ID from the store
    const sessionId = await getActiveSessionId(page);
    expect(sessionId).toBeTruthy();
    if (!sessionId) return;

    // First, add several commands to create enough content to require scrolling
    for (let i = 0; i < 5; i++) {
      await page.evaluate(
        async ({ sid, idx }) => {
          const mocks = await import("../frontend/mocks");
          await mocks.simulateCommand(
            sid,
            `echo "Command ${idx}"`,
            `Output line 1 for command ${idx}\nOutput line 2 for command ${idx}\nOutput line 3 for command ${idx}\nOutput line 4 for command ${idx}\nOutput line 5 for command ${idx}\n`,
            0
          );
        },
        { sid: sessionId, idx: i }
      );

      // Small delay between commands
      await page.waitForTimeout(100);
    }

    // Wait for timeline to update with all commands
    await page.waitForTimeout(500);

    // Scroll to top to simulate user scrolling up
    await page.evaluate(() => {
      const timeline = document.querySelector(".overflow-auto");
      if (timeline) {
        timeline.scrollTop = 0;
      }
    });

    // Verify we're not at the bottom
    const beforeScroll = await page.evaluate(() => {
      const timeline = document.querySelector(".overflow-auto");
      if (timeline) {
        return {
          scrollTop: timeline.scrollTop,
          scrollHeight: timeline.scrollHeight,
          clientHeight: timeline.clientHeight,
        };
      }
      return null;
    });

    expect(beforeScroll).toBeTruthy();
    if (!beforeScroll) return;
    expect(beforeScroll.scrollTop).toBe(0);

    // Now execute another command - this should trigger auto-scroll to bottom
    await page.evaluate(
      async ({ sid }) => {
        const mocks = await import("../frontend/mocks");
        await mocks.simulateCommand(
          sid,
          "echo 'Final command'",
          "This is the final command output\n",
          0
        );
      },
      { sid: sessionId }
    );

    // Wait for the scroll animation frame to complete
    await page.waitForTimeout(200);

    // Check that we've scrolled to the bottom
    const afterScroll = await page.evaluate(() => {
      const timeline = document.querySelector(".overflow-auto");
      if (timeline) {
        return {
          scrollTop: timeline.scrollTop,
          scrollHeight: timeline.scrollHeight,
          clientHeight: timeline.clientHeight,
          isAtBottom:
            Math.abs(timeline.scrollTop + timeline.clientHeight - timeline.scrollHeight) < 5,
        };
      }
      return null;
    });

    expect(afterScroll).toBeTruthy();
    if (!afterScroll) return;
    expect(afterScroll.isAtBottom).toBe(true);
  });

  test("should scroll to bottom when streaming output arrives", async ({ page }) => {
    const sessionId = await getActiveSessionId(page);
    expect(sessionId).toBeTruthy();
    if (!sessionId) return;

    // Start a command
    await page.evaluate(
      async ({ sid }) => {
        const mocks = await import("../frontend/mocks");
        await mocks.emitCommandBlockEvent(sid, "prompt_start");
        await mocks.emitCommandBlockEvent(sid, "command_start", "long-running-command");
      },
      { sid: sessionId }
    );

    // Send multiple lines of output
    for (let i = 0; i < 20; i++) {
      await page.evaluate(
        async ({ sid, idx }) => {
          const mocks = await import("../frontend/mocks");
          await mocks.emitTerminalOutput(sid, `Processing step ${idx}...\r\n`);
        },
        { sid: sessionId, idx: i }
      );
      await page.waitForTimeout(50);
    }

    // Wait for scrolling
    await page.waitForTimeout(200);

    // Check scroll position - we should be near the bottom due to streaming
    const scrollState = await page.evaluate(() => {
      const timeline = document.querySelector(".overflow-auto");
      if (timeline) {
        return {
          scrollTop: timeline.scrollTop,
          scrollHeight: timeline.scrollHeight,
          clientHeight: timeline.clientHeight,
          isNearBottom: timeline.scrollHeight - timeline.scrollTop - timeline.clientHeight < 100,
        };
      }
      return null;
    });

    expect(scrollState).toBeTruthy();
    if (!scrollState) return;
    expect(scrollState.isNearBottom).toBe(true);

    // End the command
    await page.evaluate(
      async ({ sid }) => {
        const mocks = await import("../frontend/mocks");
        await mocks.emitCommandBlockEvent(sid, "command_end", "long-running-command", 0);
      },
      { sid: sessionId }
    );
  });
});
