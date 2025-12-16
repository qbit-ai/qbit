"""Extended thinking / reasoning evaluation tests.

Tests the Qbit agent's extended thinking feature:
- Main agent produces reasoning events
- Reasoning content is non-empty for complex tasks
- Sub-agents handle thinking blocks correctly in multi-turn conversations

Run all tests:
    RUN_API_TESTS=1 pytest test_thinking.py -v

Configure models in ~/.qbit/settings.toml:
    [eval]
    model = "gpt-4o-mini"       # DeepEval evaluator model
    agent_model = "claude-..."  # Qbit agent model (should support extended thinking)
"""

import pytest

from client import RunResult, StreamingRunner


# =============================================================================
# Shared Fixtures - Run LLM once, test multiple things
# =============================================================================


@pytest.fixture(scope="class")
async def simple_thinking_result(class_runner: StreamingRunner) -> RunResult:
    """Shared fixture: Simple response to verify reasoning events are emitted.

    One LLM call shared by: test_has_reasoning_events, test_reasoning_content_exists
    """
    return await class_runner.run("What is 15 + 27? Think through it step by step.")


@pytest.fixture(scope="class")
async def complex_thinking_result(class_runner: StreamingRunner) -> RunResult:
    """Shared fixture: Complex task requiring more reasoning.

    One LLM call shared by: test_complex_reasoning_length, test_reasoning_quality
    """
    return await class_runner.run(
        "Explain the pros and cons of using a linked list vs an array for a queue implementation. "
        "Consider memory usage, time complexity, and implementation complexity."
    )


# =============================================================================
# Main Agent Thinking Tests - Basic Events
# =============================================================================


@pytest.mark.requires_api
class TestMainAgentThinkingEvents:
    """Tests that verify the main agent produces reasoning events."""

    @pytest.mark.asyncio
    async def test_response_succeeds(self, simple_thinking_result: RunResult):
        """Basic response with thinking request succeeds."""
        result = simple_thinking_result
        assert result.success
        assert result.response

    @pytest.mark.asyncio
    async def test_has_reasoning_events(self, simple_thinking_result: RunResult):
        """Response contains reasoning events when extended thinking is enabled."""
        result = simple_thinking_result
        # With extended thinking enabled, we expect reasoning events
        # If this fails, extended thinking may not be enabled on the model
        assert result.has_reasoning, (
            "No reasoning events found. "
            "Ensure extended thinking is enabled on the Vertex AI model "
            "(check llm_client.rs: .with_default_thinking())"
        )

    @pytest.mark.asyncio
    async def test_reasoning_content_exists(self, simple_thinking_result: RunResult):
        """Reasoning events contain actual content."""
        result = simple_thinking_result
        if result.has_reasoning:
            assert len(result.reasoning_content) > 0, (
                "Reasoning events exist but have no content"
            )

    @pytest.mark.asyncio
    async def test_reasoning_events_structure(self, simple_thinking_result: RunResult):
        """Reasoning events have correct structure."""
        result = simple_thinking_result
        for event in result.reasoning_events:
            assert event.event == "reasoning"
            assert event.timestamp > 0
            # Each reasoning event should have a 'content' field
            assert "content" in event.data


# =============================================================================
# Main Agent Thinking Tests - Complex Reasoning
# =============================================================================


@pytest.mark.requires_api
class TestMainAgentComplexThinking:
    """Tests for reasoning quality on complex tasks."""

    @pytest.mark.asyncio
    async def test_complex_response_succeeds(self, complex_thinking_result: RunResult):
        """Complex reasoning task succeeds."""
        result = complex_thinking_result
        assert result.success
        assert result.response

    @pytest.mark.asyncio
    async def test_complex_has_reasoning(self, complex_thinking_result: RunResult):
        """Complex tasks produce reasoning events."""
        result = complex_thinking_result
        assert result.has_reasoning, (
            "Complex reasoning task should produce thinking content"
        )

    @pytest.mark.asyncio
    async def test_complex_reasoning_length(self, complex_thinking_result: RunResult):
        """Complex tasks produce substantial reasoning content."""
        result = complex_thinking_result
        if result.has_reasoning:
            # Complex tasks should produce more reasoning than simple ones
            reasoning_len = len(result.reasoning_content)
            assert reasoning_len >= 100, (
                f"Expected substantial reasoning for complex task, got {reasoning_len} chars"
            )

    @pytest.mark.asyncio
    async def test_reasoning_before_response(self, complex_thinking_result: RunResult):
        """Reasoning events occur before the final response."""
        result = complex_thinking_result
        if result.has_reasoning and result.completed_event:
            # Find first reasoning event timestamp
            first_reasoning_ts = min(e.timestamp for e in result.reasoning_events)
            # Find completed event timestamp
            completed_ts = result.completed_event.timestamp
            assert first_reasoning_ts < completed_ts, (
                "Reasoning should occur before completion"
            )


# =============================================================================
# Thinking with Tool Use
# =============================================================================


@pytest.mark.requires_api
class TestThinkingWithTools:
    """Tests for reasoning combined with tool usage."""

    @pytest.mark.asyncio
    async def test_thinking_with_file_read(self, runner: StreamingRunner):
        """Agent uses reasoning when reading and analyzing files."""
        result = await runner.run(
            "Read the file ./main.go and explain what design patterns it uses. "
            "Think carefully about the code structure."
        )
        assert result.success
        # Should have both tool calls and reasoning
        assert len(result.tool_calls) > 0, "Expected file read tool call"
        # Extended thinking should be present even with tool use
        # Note: This may or may not produce reasoning depending on task complexity
        # We don't assert has_reasoning here as tool-focused tasks may not always think

    @pytest.mark.asyncio
    async def test_thinking_preserved_across_tool_calls(self, runner: StreamingRunner):
        """Reasoning is maintained correctly across multiple tool calls."""
        result = await runner.run(
            "First, read ./main.go, then read ./go.mod. "
            "Compare what you find and explain the relationship between them. "
            "Think through your analysis step by step."
        )
        assert result.success
        # Should have multiple tool calls
        assert len(result.tool_calls) >= 2, "Expected multiple file reads"


# =============================================================================
# Sub-Agent Thinking Tests
# =============================================================================


@pytest.mark.requires_api
class TestSubAgentThinking:
    """Tests for sub-agent thinking support.

    Note: Sub-agent thinking content is logged but not directly exposed to the
    parent agent or frontend. These tests verify that sub-agents complete
    successfully when thinking is enabled on the shared model.
    """

    @pytest.mark.asyncio
    async def test_sub_agent_task_succeeds(self, runner: StreamingRunner):
        """Sub-agent tasks complete successfully with thinking enabled.

        This tests that sub-agents properly handle thinking blocks in their
        message history without errors.
        """
        # This prompt should trigger the code analysis sub-agent
        result = await runner.run(
            "Analyze the code structure of main.go and explain what it does. "
            "Use the code analysis agent if available."
        )
        assert result.success
        assert result.response

    @pytest.mark.asyncio
    async def test_sub_agent_multi_turn(self, runner: StreamingRunner):
        """Sub-agents handle thinking correctly in multi-turn conversations.

        Tests that thinking blocks are properly ordered in message history
        (thinking must come before text/tool calls per Anthropic API).
        """
        # First turn - establish context
        result1 = await runner.run(
            "Read the file ./main.go and remember what package it belongs to."
        )
        assert result1.success

        # Second turn - use remembered context
        result2 = await runner.run(
            "What package was that file in? Also explain what fmt.Println does."
        )
        assert result2.success
        # Response should reference the package name from the file
        assert "main" in result2.response.lower() or "fmt" in result2.response.lower()


# =============================================================================
# Batch Mode with Thinking
# =============================================================================


@pytest.mark.requires_api
class TestBatchModeThinking:
    """Tests for thinking in batch execution mode."""

    @pytest.mark.asyncio
    async def test_batch_with_thinking_tasks(self, runner: StreamingRunner):
        """Batch mode handles thinking tasks correctly."""
        result = await runner.run_batch(
            [
                "What is 7 * 8? Think step by step.",
                "What is 12 + 15? Think step by step.",
            ],
            quiet=True,
        )
        assert result.success
        assert len(result.responses) == 2
        # Both responses should have the correct answers
        assert "56" in result.responses[0]
        assert "27" in result.responses[1]
