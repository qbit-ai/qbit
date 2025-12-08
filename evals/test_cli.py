"""Integration tests for qbit-cli using DeepEval.

Run basic tests (no API needed):
    pytest test_cli.py -v -k "TestCliBasics"

Run all tests including API/eval tests:
    RUN_API_TESTS=1 pytest test_cli.py -v

Configure models in settings.toml:
    [eval]
    model = "gpt-4o-mini"       # evaluator model (OpenAI)
    agent_model = "claude-..."  # qbit-cli agent model (or use QBIT_EVAL_MODEL env var)
    api_key = "sk-..."          # or use OPENAI_API_KEY env var

Test Organization:
- TestCliBasics: Fast CLI tests, no API needed
- TestCliBehavior: CLI behavior tests with API, no DeepEval
- TestMemoryAndState: Memory recall tests with DeepEval
- TestResponseQuality: Arithmetic/instruction tests with DeepEval
- TestCharacterHandling: Unicode/special char tests with DeepEval
- TestToolUsage: Tool execution tests with DeepEval
"""

from typing import Any

import pytest
from deepeval import evaluate
from deepeval.metrics import GEval
from deepeval.test_case import LLMTestCase, LLMTestCaseParams

from conftest import CliRunner, JsonRunResult, get_last_response


# =============================================================================
# Helper Functions
# =============================================================================


def run_scenario(cli: CliRunner, scenario: dict) -> dict:
    """Run a single CLI scenario.

    Args:
        cli: The CLI runner instance
        scenario: Scenario dict with 'prompts' or 'prompt' key

    Returns:
        Completed scenario with 'output' and 'success' fields added
    """
    if "prompts" in scenario:
        # Batch mode
        result = cli.run_batch(scenario["prompts"], quiet=True)
        output = get_last_response(result.stdout) if result.returncode == 0 else ""
        return {
            **scenario,
            "result": result,
            "output": output,
            "success": result.returncode == 0,
        }
    elif "prompt" in scenario:
        # Single prompt mode (JSON)
        json_result = cli.run_prompt_json(scenario["prompt"])
        return {
            **scenario,
            "json_result": json_result,
            "output": json_result.response,
            "success": json_result.returncode == 0,
        }
    else:
        raise ValueError("Scenario must have 'prompts' or 'prompt' key")


def evaluate_scenario(scenario: dict, eval_model: Any) -> None:
    """Evaluate a single scenario with DeepEval.

    Args:
        scenario: Completed scenario with 'output' field
        eval_model: DeepEval model for evaluation

    Raises:
        AssertionError: If evaluation fails
    """
    # Determine eval params based on scenario config
    eval_params = [LLMTestCaseParams.ACTUAL_OUTPUT]
    if scenario.get("use_context"):
        eval_params.append(LLMTestCaseParams.CONTEXT)
    else:
        eval_params.append(LLMTestCaseParams.EXPECTED_OUTPUT)

    test_case = LLMTestCase(
        input=scenario["input"],
        actual_output=scenario["output"],
        expected_output=scenario.get("expected", ""),
        context=scenario.get("context", []),
    )

    metric = GEval(
        name=scenario["metric_name"],
        criteria=scenario["criteria"],
        evaluation_steps=scenario["steps"],
        evaluation_params=eval_params,
        threshold=scenario.get("threshold", 0.8),
        model=eval_model,
    )

    results = evaluate([test_case], [metric])

    if not results.test_results[0].success:
        raise AssertionError(
            f"DeepEval failed for {scenario['metric_name']}: "
            f"input={scenario['input']}, output={scenario['output'][:200] if scenario['output'] else ''}"
        )


# =============================================================================
# Basic CLI Tests (no API needed)
# =============================================================================


class TestCliBasics:
    """Tests that don't require API credentials - instant execution."""

    def test_help(self, cli: CliRunner):
        """CLI shows help."""
        result = cli.run("--help")
        assert result.returncode == 0
        assert "--execute" in result.stdout
        assert "--file" in result.stdout
        assert "--auto-approve" in result.stdout
        assert "--json" in result.stdout
        assert "--quiet" in result.stdout

    def test_version(self, cli: CliRunner):
        """CLI shows version."""
        result = cli.run("--version")
        assert result.returncode == 0
        assert "qbit-cli" in result.stdout

    def test_conflicting_args(self, cli: CliRunner, temp_prompt_file):
        """Cannot use -e and -f together."""
        temp_prompt_file.write_text("test")
        result = cli.run("-e", "test", "-f", str(temp_prompt_file))
        assert result.returncode != 0
        assert "cannot be used with" in result.stderr

    def test_missing_file(self, cli: CliRunner):
        """Error on missing prompt file."""
        result = cli.run("-f", "/nonexistent/path.txt", "--auto-approve")
        assert result.returncode != 0

    def test_empty_file(self, cli: CliRunner, temp_prompt_file):
        """Error on empty prompt file."""
        temp_prompt_file.write_text("")
        result = cli.run("-f", str(temp_prompt_file), "--auto-approve")
        assert result.returncode != 0
        assert "No prompts found" in result.stderr

    def test_comments_only_file(self, cli: CliRunner, temp_prompt_file):
        """Error when file has only comments."""
        temp_prompt_file.write_text("# comment 1\n# comment 2\n")
        result = cli.run("-f", str(temp_prompt_file), "--auto-approve")
        assert result.returncode != 0
        assert "No prompts found" in result.stderr


# =============================================================================
# CLI Behavior Tests (API required, no DeepEval)
# =============================================================================


@pytest.mark.requires_api
class TestCliBehavior:
    """Tests that verify CLI behavior without DeepEval evaluation.

    Optimized by consolidating tests that can share CLI calls.
    """

    def test_batch_progress_output(self, cli: CliRunner):
        """Batch mode shows progress."""
        result = cli.run_batch(["Say 'one'", "Say 'two'", "Say 'three'"], quiet=False)
        assert result.returncode == 0
        assert "[1/3]" in result.stderr
        assert "[2/3]" in result.stderr
        assert "[3/3]" in result.stderr
        assert "All 3 prompt(s) completed" in result.stderr

    def test_batch_skips_comments(self, cli: CliRunner, temp_prompt_file):
        """Batch mode skips comment lines."""
        temp_prompt_file.write_text(
            "# This is a comment\nSay 'first'\n# Another comment\n\nSay 'second'\n"
        )
        result = cli.run("-f", str(temp_prompt_file), "--auto-approve")
        assert result.returncode == 0
        assert "[1/2]" in result.stderr
        assert "[2/2]" in result.stderr

    def test_simple_json_response(self, cli: CliRunner):
        """JSON structure, event sequence, turn_id, duration, and streaming.

        Consolidates multiple JSON output tests into one CLI call.
        """
        result: JsonRunResult = cli.run_prompt_json("Say 'hello world'")
        assert result.returncode == 0

        # JSON output structure
        assert len(result.events) > 0, "Expected at least one event"
        event_types = {e.event for e in result.events}
        assert "started" in event_types
        assert "completed" in event_types
        for event in result.events:
            assert event.timestamp > 0
        assert result.response

        # Event sequence (started before completed, timestamps ascending)
        event_type_list = [e.event for e in result.events]
        started_idx = event_type_list.index("started")
        completed_idx = event_type_list.index("completed")
        assert started_idx < completed_idx
        timestamps = [e.timestamp for e in result.events]
        assert timestamps == sorted(timestamps)

        # Started event has turn_id
        started = [e for e in result.events if e.event == "started"]
        assert len(started) == 1
        assert started[0].get("turn_id") is not None

        # Completed event has duration
        assert result.duration_ms is not None and result.duration_ms > 0

        # Text delta events contain streaming chunks
        deltas = [e for e in result.events if e.event == "text_delta"]
        assert len(deltas) > 0
        for d in deltas:
            assert "delta" in d.data or "accumulated" in d.data

    def test_file_reading_json_events(self, cli: CliRunner):
        """Tool calls, results, sequence, event types, and convenience methods.

        Consolidates multiple tool-related tests into one CLI call.
        """
        result: JsonRunResult = cli.run_prompt_json(
            "Read the file ./conftest.py in the current directory and tell me briefly what it contains"
        )
        assert result.returncode == 0

        # Tool calls include input parameters
        assert len(result.tool_calls) > 0
        assert result.tool_calls[0].get("input") is not None

        # Tool results include output
        successful = [tr for tr in result.tool_results if tr.get("success")]
        assert len(successful) > 0
        assert successful[0].get("output") is not None

        # Tool calls precede results
        events = result.events
        call_idx = [
            i for i, e in enumerate(events)
            if e.event in ("tool_call", "tool_auto_approved")
        ]
        result_idx = [i for i, e in enumerate(events) if e.event == "tool_result"]
        assert len(call_idx) > 0 and len(result_idx) > 0
        assert call_idx[0] < result_idx[0]

        # All event types recognized
        known = {
            "started", "text_delta", "tool_call", "tool_result", "tool_approval",
            "tool_auto_approved", "tool_denied", "reasoning", "completed", "error",
            "sub_agent_started", "sub_agent_tool_request", "sub_agent_tool_result",
            "sub_agent_completed", "sub_agent_error", "context_pruned", "context_warning",
            "tool_response_truncated", "loop_warning", "loop_blocked", "max_iterations_reached",
            "workflow_started", "workflow_step_started", "workflow_step_completed",
            "workflow_completed", "workflow_error",
        }
        for e in result.events:
            assert e.event in known, f"Unknown event: {e.event}"

        # Convenience methods work
        assert not result.has_tool("nonexistent_tool_xyz")
        if result.tool_calls:
            first = result.tool_calls[0].get("tool_name")
            assert result.has_tool(first)
        if result.tool_results:
            name = result.tool_results[0].get("tool_name")
            assert result.get_tool_output(name) is not None

    def test_unicode_in_json(self, cli: CliRunner):
        """Unicode characters are preserved in JSON output."""
        result: JsonRunResult = cli.run_prompt_json("Say the Japanese word '日本語'")
        assert result.returncode == 0
        assert len(result.events) > 0
        assert result.completed_event is not None
        if any(ord(c) > 127 for c in result.response):
            assert "\\u" not in result.response

    def test_newlines_in_json(self, cli: CliRunner):
        """Newlines don't break JSON parsing."""
        result: JsonRunResult = cli.run_prompt_json(
            "Print 'line1' then 'line2' on separate lines"
        )
        assert result.returncode == 0
        assert len(result.events) > 0
        event_types = {e.event for e in result.events}
        assert "started" in event_types and "completed" in event_types


# =============================================================================
# DeepEval Tests - Memory & State
# =============================================================================


@pytest.mark.requires_api
class TestMemoryAndState:
    """Tests for session memory and state tracking."""

    def test_number_recall(self, cli: CliRunner, eval_model):
        """Agent remembers a number across prompts."""
        scenario = {
            "prompts": [
                "Remember: the magic number is 42. Just say 'OK'.",
                "What is the magic number? Reply with just the number.",
            ],
            "input": "What is the magic number?",
            "expected": "42",
            "context": ["The magic number is 42."],
            "metric_name": "Number Recall",
            "criteria": "The response must contain the number 42.",
            "steps": ["Check if response contains 42", "Should be exactly or close to '42'"],
            "threshold": 0.8,
        }
        completed = run_scenario(cli, scenario)
        assert completed["success"], f"CLI failed: {completed['result'].stderr}"
        evaluate_scenario(completed, eval_model)

    def test_word_recall(self, cli: CliRunner, eval_model):
        """Agent remembers a word across prompts."""
        scenario = {
            "prompts": [
                "The word of the day is 'elephant'. Just say 'understood'.",
                "What was the word of the day? Reply with just that word.",
            ],
            "input": "What was the word of the day?",
            "expected": "elephant",
            "context": ["The word of the day is 'elephant'."],
            "metric_name": "Word Recall",
            "criteria": "The response must contain 'elephant' (case-insensitive).",
            "steps": ["Check if response contains 'elephant'", "Case should not matter"],
            "threshold": 0.8,
        }
        completed = run_scenario(cli, scenario)
        assert completed["success"], f"CLI failed: {completed['result'].stderr}"
        evaluate_scenario(completed, eval_model)

    def test_multi_fact_recall(self, cli: CliRunner, eval_model):
        """Agent remembers multiple facts across prompts."""
        scenario = {
            "prompts": [
                "My name is Alice. Say 'noted'.",
                "My favorite color is blue. Say 'noted'.",
                "I live in Paris. Say 'noted'.",
                "Summarize what you know about me in one sentence.",
            ],
            "input": "Summarize what you know about me.",
            "expected": "Alice lives in Paris and her favorite color is blue.",
            "context": ["User's name is Alice", "Favorite color is blue", "Lives in Paris"],
            "metric_name": "Multi-Fact Recall",
            "criteria": "Summary must include: name (Alice), color (blue), location (Paris).",
            "steps": ["Check for Alice", "Check for blue", "Check for Paris", "No hallucinations"],
            "threshold": 0.9,
            "use_context": True,
        }
        completed = run_scenario(cli, scenario)
        assert completed["success"], f"CLI failed: {completed['result'].stderr}"
        evaluate_scenario(completed, eval_model)

    def test_cumulative_calculation(self, cli: CliRunner, eval_model):
        """Agent tracks cumulative state."""
        scenario = {
            "prompts": [
                "I have 3 apples. Say 'noted'.",
                "I buy 2 more apples. Say 'noted'.",
                "How many apples do I have now? Just the number.",
            ],
            "input": "How many apples do I have now?",
            "expected": "5",
            "context": ["Had 3 apples", "Bought 2 more", "Total should be 5"],
            "metric_name": "Arithmetic Recall",
            "criteria": "Response must contain 5 (3 + 2 = 5).",
            "steps": ["Check if response contains 5", "Calculation 3 + 2 = 5 is correct"],
            "threshold": 0.9,
        }
        completed = run_scenario(cli, scenario)
        assert completed["success"], f"CLI failed: {completed['result'].stderr}"
        evaluate_scenario(completed, eval_model)

    def test_long_chain_recall(self, cli: CliRunner, eval_model):
        """Agent remembers facts over many turns."""
        scenario = {
            "prompts": [
                "Step 1: Remember A=1. Say 'ok'.",
                "Step 2: Remember B=2. Say 'ok'.",
                "Step 3: Remember C=3. Say 'ok'.",
                "Step 4: Remember D=4. Say 'ok'.",
                "Step 5: What are the values of A, B, C, and D? List them.",
            ],
            "input": "What are the values of A, B, C, and D?",
            "expected": "A=1, B=2, C=3, D=4",
            "context": ["A=1", "B=2", "C=3", "D=4"],
            "metric_name": "Long Chain Recall",
            "criteria": "Response must contain all four values: A=1, B=2, C=3, D=4.",
            "steps": ["Check for A=1", "Check for B=2", "Check for C=3", "Check for D=4"],
            "threshold": 0.9,
            "use_context": True,
        }
        completed = run_scenario(cli, scenario)
        assert completed["success"], f"CLI failed: {completed['result'].stderr}"
        evaluate_scenario(completed, eval_model)


# =============================================================================
# DeepEval Tests - Response Quality
# =============================================================================


@pytest.mark.requires_api
class TestResponseQuality:
    """Tests for arithmetic and instruction following."""

    def test_basic_arithmetic(self, cli: CliRunner, eval_model):
        """Agent performs basic arithmetic."""
        scenario = {
            "prompt": "What is 1+1? Just the number.",
            "input": "What is 1+1?",
            "expected": "2",
            "metric_name": "Basic Arithmetic",
            "criteria": "Response must contain the number 2.",
            "steps": ["Check if response contains '2'"],
            "threshold": 0.9,
        }
        completed = run_scenario(cli, scenario)
        assert completed["success"], "CLI failed"
        evaluate_scenario(completed, eval_model)

    def test_batch_arithmetic(self, cli: CliRunner, eval_model):
        """Agent performs arithmetic in batch mode."""
        scenario = {
            "prompts": ["What is 2+2? Just the number."],
            "input": "What is 2+2?",
            "expected": "4",
            "metric_name": "Batch Arithmetic",
            "criteria": "Response must contain the number 4.",
            "steps": ["Check if response contains '4'"],
            "threshold": 0.9,
        }
        completed = run_scenario(cli, scenario)
        assert completed["success"], f"CLI failed: {completed['result'].stderr}"
        evaluate_scenario(completed, eval_model)

    def test_instruction_following(self, cli: CliRunner, eval_model):
        """Agent follows exact instructions."""
        scenario = {
            "prompts": ["Say exactly: 'test response'"],
            "input": "Say exactly: 'test response'",
            "expected": "test response",
            "metric_name": "Instruction Following",
            "criteria": "Response should contain or closely match 'test response'.",
            "steps": ["Check if response contains 'test response' (case-insensitive)"],
            "threshold": 0.8,
        }
        completed = run_scenario(cli, scenario)
        assert completed["success"], f"CLI failed: {completed['result'].stderr}"
        evaluate_scenario(completed, eval_model)


# =============================================================================
# DeepEval Tests - Character Handling
# =============================================================================


@pytest.mark.requires_api
class TestCharacterHandling:
    """Tests for unicode, special characters, and multiline responses."""

    def test_unicode_recall(self, cli: CliRunner, eval_model):
        """Agent preserves unicode characters."""
        scenario = {
            "prompts": [
                "The word is '日本語'. Say 'received'.",
                "What was the word? Reply with just that word.",
            ],
            "input": "What was the word?",
            "expected": "日本語",
            "context": ["The word is '日本語'"],
            "metric_name": "Unicode Recall",
            "criteria": "Response must contain the Japanese characters '日本語'.",
            "steps": ["Check for '日本語'", "Unicode should be preserved exactly"],
            "threshold": 0.9,
        }
        completed = run_scenario(cli, scenario)
        assert completed["success"], f"CLI failed: {completed['result'].stderr}"
        evaluate_scenario(completed, eval_model)

    def test_special_characters(self, cli: CliRunner, eval_model):
        """Agent handles special characters."""
        scenario = {
            "prompt": "Echo back exactly: @#$%^&*()",
            "input": "Echo back exactly: @#$%^&*()",
            "expected": "@#$%^&*()",
            "metric_name": "Special Character Handling",
            "criteria": "Response should contain some or all of: @#$%^&*()",
            "steps": ["Check for at least some special characters"],
            "threshold": 0.6,
        }
        completed = run_scenario(cli, scenario)
        assert completed["success"], "CLI failed"
        evaluate_scenario(completed, eval_model)

    def test_multiline_response(self, cli: CliRunner, eval_model):
        """Agent produces multiline output."""
        scenario = {
            "prompt": "List the numbers 1, 2, 3 on separate lines.",
            "input": "List the numbers 1, 2, 3 on separate lines.",
            "expected": "1\n2\n3",
            "metric_name": "Multiline Output",
            "criteria": "Response should contain 1, 2, 3 each on separate lines or clearly listed.",
            "steps": ["Check for '1'", "Check for '2'", "Check for '3'", "Should be separated"],
            "threshold": 0.8,
        }
        completed = run_scenario(cli, scenario)
        assert completed["success"], "CLI failed"
        evaluate_scenario(completed, eval_model)


# =============================================================================
# DeepEval Tests - Tool Usage
# =============================================================================


@pytest.mark.requires_api
class TestToolUsage:
    """Tests for tool execution and file operations."""

    def test_read_file(self, cli: CliRunner, eval_model):
        """Agent reads and summarizes file contents."""
        scenario = {
            "prompt": "Read the file ./conftest.py and tell me what the CliRunner class does in one sentence.",
            "input": "What does the CliRunner class do?",
            "expected": "CliRunner is a helper class that runs CLI commands for testing.",
            "context": [
                "conftest.py contains the CliRunner class",
                "CliRunner wraps subprocess calls to qbit-cli",
                "Methods include run(), run_prompt(), run_batch()",
            ],
            "metric_name": "File Reading Comprehension",
            "criteria": "Response should accurately describe what CliRunner does.",
            "steps": [
                "Check if mentions CLI or command execution",
                "Check if mentions running or testing",
                "Should demonstrate understanding of file contents",
            ],
            "threshold": 0.7,
            "use_context": True,
            "verify_tool": {
                "tools": {"read_file", "read", "file_read"},
                "content_check": "CliRunner",
            },
        }
        completed = run_scenario(cli, scenario)
        assert completed["success"], "CLI failed"

        # Verify tool usage
        json_result = completed["json_result"]
        expected_tools = scenario["verify_tool"]["tools"]
        tool_names = {tc.get("tool_name") for tc in json_result.tool_calls}
        assert tool_names & expected_tools, f"Expected tool from {expected_tools}. Got: {tool_names}"

        successful = [tr for tr in json_result.tool_results if tr.get("success")]
        assert len(successful) > 0, "Expected at least one successful tool result"

        evaluate_scenario(completed, eval_model)

    def test_list_directory(self, cli: CliRunner, eval_model):
        """Agent lists directory contents."""
        scenario = {
            "prompt": "What files are in the current directory? Just list a few.",
            "input": "What files are in the current directory?",
            "expected": "conftest.py, test_cli.py, pyproject.toml",
            "context": [
                "Directory contains conftest.py",
                "Directory contains test_cli.py",
                "Directory contains pyproject.toml",
            ],
            "metric_name": "Directory Listing",
            "criteria": "Response should list at least one relevant file from the test directory.",
            "steps": [
                "Check for conftest.py, test_cli.py, or pyproject.toml",
                "Should indicate files were successfully listed",
            ],
            "threshold": 0.7,
            "use_context": True,
            "verify_tool": {
                "tools": {"list_directory", "ls", "list_files", "glob", "list_dir"},
            },
        }
        completed = run_scenario(cli, scenario)
        assert completed["success"], "CLI failed"

        # Verify tool usage
        json_result = completed["json_result"]
        expected_tools = scenario["verify_tool"]["tools"]
        tool_names = {tc.get("tool_name") for tc in json_result.tool_calls}
        assert tool_names & expected_tools, f"Expected tool from {expected_tools}. Got: {tool_names}"

        successful = [tr for tr in json_result.tool_results if tr.get("success")]
        assert len(successful) > 0, "Expected at least one successful tool result"

        evaluate_scenario(completed, eval_model)
