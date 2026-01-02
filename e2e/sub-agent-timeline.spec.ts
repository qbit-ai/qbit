import { expect, type Page, test } from "@playwright/test";

/**
 * Sub-Agent Timeline Ordering E2E Tests
 *
 * These tests verify that sub-agents appear in the correct position in the timeline,
 * not always at the bottom. Sub-agents should be interleaved with other content
 * based on when they were spawned during the conversation.
 */

// Type definitions for the global mock functions
declare global {
  interface Window {
    __MOCK_BROWSER_MODE__?: boolean;
    __MOCK_EMIT_AI_EVENT__?: (event: AiEventType) => Promise<void>;
    __MOCK_SIMULATE_AI_RESPONSE_WITH_SUB_AGENT__?: (
      subAgentName: string,
      subAgentTask: string,
      subAgentResponse: string,
      finalResponse: string,
      delayMs?: number
    ) => Promise<void>;
    __MOCK_SIMULATE_AI_RESPONSE__?: (response: string, delayMs?: number) => Promise<void>;
  }
}

type AiEventType =
  | { type: "started"; turn_id: string }
  | { type: "text_delta"; delta: string; accumulated: string }
  | { type: "tool_request"; tool_name: string; args: unknown; request_id: string }
  | {
      type: "tool_result";
      tool_name: string;
      result: unknown;
      success: boolean;
      request_id: string;
    }
  | {
      type: "completed";
      response: string;
      tokens_used?: number;
      duration_ms?: number;
      input_tokens?: number;
      output_tokens?: number;
    }
  | { type: "error"; message: string; error_type: string }
  | { type: "sub_agent_started"; agent_id: string; agent_name: string; task: string; depth: number }
  | {
      type: "sub_agent_tool_request";
      agent_id: string;
      tool_name: string;
      args: unknown;
      request_id: string;
    }
  | {
      type: "sub_agent_tool_result";
      agent_id: string;
      tool_name: string;
      result: unknown;
      success: boolean;
      request_id: string;
    }
  | { type: "sub_agent_completed"; agent_id: string; response: string; duration_ms: number }
  | { type: "sub_agent_error"; agent_id: string; error: string };

/**
 * Wait for the app to be fully ready in browser mode.
 */
async function waitForAppReady(page: Page) {
  await page.goto("/");
  await page.waitForLoadState("domcontentloaded");

  // Wait for the mock browser mode flag to be set
  await page.waitForFunction(() => window.__MOCK_BROWSER_MODE__ === true, { timeout: 15000 });

  // Wait for the status bar to appear (indicates React has rendered)
  await expect(page.locator('[data-testid="status-bar"]')).toBeVisible({
    timeout: 10000,
  });

  // Wait for the unified input textarea to be visible
  await expect(page.locator("textarea")).toBeVisible({ timeout: 5000 });

  // Ensure mock functions are available
  await page.waitForFunction(() => typeof window.__MOCK_EMIT_AI_EVENT__ === "function", {
    timeout: 5000,
  });
}

/**
 * Get the Agent/AI mode toggle button (Bot icon).
 */
function getAgentModeButton(page: Page) {
  return page.getByRole("button", { name: "Switch to AI mode" });
}

/**
 * Get the UnifiedInput textarea element.
 */
function getInputTextarea(page: Page) {
  return page.locator("textarea");
}

test.describe("Sub-Agent Timeline Ordering", () => {
  test.beforeEach(async ({ page }) => {
    await waitForAppReady(page);
  });

  test("sub-agent should appear in timeline when turn completes, not in streaming section", async ({
    page,
  }) => {
    // Switch to AI mode
    const agentButton = getAgentModeButton(page);
    await agentButton.click();

    const textarea = getInputTextarea(page);
    await expect(textarea).toHaveAttribute("placeholder", "Ask the AI...", { timeout: 3000 });

    // Simulate an AI response with a sub-agent using the global mock helper
    await page.evaluate(async () => {
      const simulate = window.__MOCK_SIMULATE_AI_RESPONSE_WITH_SUB_AGENT__;
      if (simulate) {
        await simulate(
          "Explorer",
          "Explore the codebase structure",
          "Found 10 TypeScript files in src/",
          "The explorer has analyzed the codebase.",
          10
        );
      }
    });

    // Wait for the response to complete
    await page.waitForTimeout(500);

    // Look for the sub-agent by name in the timeline
    const explorerText = page.getByText("Explorer", { exact: false });
    await expect(explorerText.first()).toBeVisible({ timeout: 5000 });

    // The response text should also be visible
    await expect(page.getByText("The explorer has analyzed the codebase")).toBeVisible({
      timeout: 3000,
    });
  });

  test("sub-agent should be cleared from active state after turn completes", async ({ page }) => {
    // Switch to AI mode
    const agentButton = getAgentModeButton(page);
    await agentButton.click();

    const textarea = getInputTextarea(page);
    await expect(textarea).toHaveAttribute("placeholder", "Ask the AI...", { timeout: 3000 });

    // Simulate first AI response with sub-agent
    await page.evaluate(async () => {
      const simulate = window.__MOCK_SIMULATE_AI_RESPONSE_WITH_SUB_AGENT__;
      if (simulate) {
        await simulate(
          "Analyzer",
          "Analyze code quality",
          "Code quality is good",
          "Analysis complete.",
          10
        );
      }
    });

    // Wait for completion
    await page.waitForTimeout(500);

    // Verify the analyzer is visible
    await expect(page.getByText("Analyzer", { exact: false }).first()).toBeVisible({
      timeout: 3000,
    });

    // Now simulate a second response WITHOUT sub-agents
    await page.evaluate(async () => {
      const simulate = window.__MOCK_SIMULATE_AI_RESPONSE__;
      if (simulate) {
        await simulate("This is a second response without sub-agents.", 10);
      }
    });

    // Wait for completion
    await page.waitForTimeout(500);

    // The second response should be visible
    await expect(page.getByText("This is a second response without sub-agents")).toBeVisible({
      timeout: 3000,
    });

    // Count how many times "Analyzer" appears - should only be once (from the first message)
    // and NOT duplicated in the second streaming section
    const analyzerOccurrences = await page.getByText("Analyzer", { exact: false }).count();

    // There should be exactly one occurrence (in the timeline, not duplicated in streaming)
    expect(analyzerOccurrences).toBe(1);
  });

  test("sub-agent should be saved to message history on turn completion", async ({ page }) => {
    // Switch to AI mode
    const agentButton = getAgentModeButton(page);
    await agentButton.click();

    const textarea = getInputTextarea(page);
    await expect(textarea).toHaveAttribute("placeholder", "Ask the AI...", { timeout: 3000 });

    // Simulate AI response with sub-agent
    await page.evaluate(async () => {
      const simulate = window.__MOCK_SIMULATE_AI_RESPONSE_WITH_SUB_AGENT__;
      if (simulate) {
        await simulate(
          "Debugger",
          "Find the bug in the code",
          "Found issue on line 42",
          "Debug analysis complete.",
          10
        );
      }
    });

    // Wait for completion
    await page.waitForTimeout(500);

    // Verify the debugger is visible
    await expect(page.getByText("Debugger", { exact: false }).first()).toBeVisible({
      timeout: 3000,
    });

    // Verify the response text is visible
    await expect(page.getByText("Debug analysis complete")).toBeVisible({
      timeout: 3000,
    });
  });

  test("multiple sub-agents should maintain their order in timeline", async ({ page }) => {
    // Switch to AI mode
    const agentButton = getAgentModeButton(page);
    await agentButton.click();

    const textarea = getInputTextarea(page);
    await expect(textarea).toHaveAttribute("placeholder", "Ask the AI...", { timeout: 3000 });

    // Simulate a turn with multiple sub-agents in sequence
    await page.evaluate(async () => {
      const emit = window.__MOCK_EMIT_AI_EVENT__;
      if (!emit) return;

      const delay = (ms: number) => new Promise((r) => setTimeout(r, ms));
      const turnId = `mock-turn-${Date.now()}`;

      // Start turn
      await emit({ type: "started", turn_id: turnId });
      await delay(20);

      // First sub-agent tool call
      const req1 = `mock-req-1-${Date.now()}`;
      await emit({
        type: "tool_request",
        tool_name: "sub_agent_first",
        args: { task: "First task" },
        request_id: req1,
      });
      await delay(20);

      // First sub-agent started
      await emit({
        type: "sub_agent_started",
        agent_id: "agent-1",
        agent_name: "FirstAgent",
        task: "First task",
        depth: 1,
      });
      await delay(20);

      // First sub-agent completed
      await emit({
        type: "sub_agent_completed",
        agent_id: "agent-1",
        response: "First done",
        duration_ms: 1000,
      });
      await delay(20);

      // First sub-agent tool result
      await emit({
        type: "tool_result",
        tool_name: "sub_agent_first",
        result: "First done",
        success: true,
        request_id: req1,
      });
      await delay(20);

      // Second sub-agent tool call
      const req2 = `mock-req-2-${Date.now()}`;
      await emit({
        type: "tool_request",
        tool_name: "sub_agent_second",
        args: { task: "Second task" },
        request_id: req2,
      });
      await delay(20);

      // Second sub-agent started
      await emit({
        type: "sub_agent_started",
        agent_id: "agent-2",
        agent_name: "SecondAgent",
        task: "Second task",
        depth: 1,
      });
      await delay(20);

      // Second sub-agent completed
      await emit({
        type: "sub_agent_completed",
        agent_id: "agent-2",
        response: "Second done",
        duration_ms: 1000,
      });
      await delay(20);

      // Second sub-agent tool result
      await emit({
        type: "tool_result",
        tool_name: "sub_agent_second",
        result: "Second done",
        success: true,
        request_id: req2,
      });
      await delay(20);

      // Final response text
      await emit({
        type: "text_delta",
        delta: "Both agents completed their tasks.",
        accumulated: "Both agents completed their tasks.",
      });
      await delay(20);

      // Complete the turn
      await emit({
        type: "completed",
        response: "Both agents completed their tasks.",
        tokens_used: 100,
        duration_ms: 3000,
        input_tokens: 50,
        output_tokens: 50,
      });
    });

    // Wait for completion
    await page.waitForTimeout(500);

    // Both agents should be visible
    await expect(page.getByText("FirstAgent", { exact: false }).first()).toBeVisible({
      timeout: 3000,
    });
    await expect(page.getByText("SecondAgent", { exact: false }).first()).toBeVisible({
      timeout: 3000,
    });

    // Get the positions of the agents in the DOM to verify order
    const positions = await page.evaluate(() => {
      const firstAgent = document.body.innerText.indexOf("FirstAgent");
      const secondAgent = document.body.innerText.indexOf("SecondAgent");
      const finalText = document.body.innerText.indexOf("Both agents completed");

      return { firstAgent, secondAgent, finalText };
    });

    // FirstAgent should appear before SecondAgent
    expect(positions.firstAgent).toBeLessThan(positions.secondAgent);
    // Both agents should appear before the final text response
    expect(positions.secondAgent).toBeLessThan(positions.finalText);
  });
});
