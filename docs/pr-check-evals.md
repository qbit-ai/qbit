# PR Check Evaluations

This document describes the lightweight evaluation system designed for pull request checks.

## Overview

The `pr-check` scenario is a multi-turn evaluation designed to quickly verify core agent capabilities during CI/CD without the cost and time of running the full evaluation suite.

## Why PR Check Evals?

### The Problem

Full evaluation suites are comprehensive but have significant drawbacks for PR workflows:

1. **Cost**: Running 15+ scenarios across 3 providers uses substantial API tokens
2. **Time**: Full evals can take 10-15 minutes, slowing down PR iteration
3. **Feedback latency**: Developers wait too long to know if their changes work

### The Solution

The `pr-check` scenario addresses these issues by:

1. **Single multi-turn scenario**: Tests multiple capabilities in one conversation
2. **~60 second runtime**: Fast enough for responsive CI feedback
3. **Core capability coverage**: Validates the most critical agent functions
4. **Provider-agnostic**: Works with vertex-claude, openai, and zai

## What It Tests

The pr-check scenario validates 9 core capabilities in a single multi-turn conversation:

| Turn | Capability | What It Tests |
|------|------------|---------------|
| 1 | Tool Awareness | Agent correctly lists available tools (read_file, edit_file, grep, etc.) |
| 2 | Sub-Agent Awareness | Agent knows about coder, analyzer, explorer, researcher, executor |
| 3 | File Operations | List directory contents, create new files |
| 4 | Edit & Search | Edit existing files, use grep to search |
| 5 | AST-grep | Structural code search using ast_grep patterns |
| 6 | Coder Create File | Delegate to coder sub-agent to create a new file using udiff |
| 7 | Coder Edit File | Delegate to coder sub-agent to edit the created file |
| 8 | Executor Delete File | Delegate to executor sub-agent to delete a file |
| 9 | Creative Response | Generate a poem about AI evals |

### Metrics Evaluated

- `tool_awareness`: LLM judge verifies agent lists core tools
- `sub_agent_awareness`: LLM judge verifies agent describes sub-agents
- `file_created`: Verifies src/lib.rs was created
- `file_edited`: Verifies src/lib.rs contains "modified"
- `coder_file_created`: Verifies src/greeting.rs was created by coder sub-agent
- `coder_file_edited`: Verifies src/greeting.rs was edited by coder (contains "edited")
- `file_deleted`: Verifies src/temp.rs was deleted by executor sub-agent
- `poem_quality`: LLM judge evaluates the creative response
- `turns_completed`: Score of completed turns (9/9)
- `sufficient_tool_usage`: At least 5 tool calls made

## CLI Usage

### Running PR Check

```bash
# Basic usage
qbit-cli --eval --scenario pr-check

# With specific provider
qbit-cli --eval --scenario pr-check --eval-provider vertex-claude

# With transcript output (shows full agent conversation with actual prompts)
qbit-cli --eval --scenario pr-check --transcript

# With transcript and pretty results
qbit-cli --eval --scenario pr-check --transcript --pretty

# Save results to file
qbit-cli --eval --scenario pr-check --output results.json --pretty
```

### The --transcript Flag

The `--transcript` flag provides visibility into what the agent actually did during evaluation:

```bash
qbit-cli --eval --scenario pr-check --transcript
```

This outputs:
1. **First**: The full agent transcript with each turn's response
2. **Then**: The evaluation results summary

The transcript shows:
- Each user turn and agent response
- All tool calls made
- Clear visual separators between turns

Example output:
```
═══════════════════════════════════════════════════════════════
                    AGENT TRANSCRIPT
═══════════════════════════════════════════════════════════════

┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
┃ Scenario: pr-check
┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

┌─ User Turn 1 ──────────────────────────────────────────────────
│ [prompt sent to agent]
├─ Agent Response ─────────────────────────────────────────────
│ # Main Tools Available
│ - read_file, edit_file, create_file...
└───────────────────────────────────────────────────────────────

┌─ User Turn 2 ──────────────────────────────────────────────────
│ [prompt sent to agent]
├─ Agent Response ─────────────────────────────────────────────
│ ## Sub-Agents
│ - coder: Handles multiple related edits...
└───────────────────────────────────────────────────────────────
```

## GitHub Actions Integration

The evaluation workflow automatically runs `pr-check` for pull requests:

```yaml
# For PRs: lightweight pr-check scenario with transcript
if [ "${{ github.event_name }}" = "pull_request" ]; then
  ./target/debug/qbit-cli --eval --scenario pr-check --transcript --pretty
fi
```

This ensures:
- Fast feedback on PRs (~60 seconds per provider)
- Full transcript available in CI logs for debugging
- Reduced API costs compared to full eval suite
- Full eval suite still runs on main branch and scheduled runs

### Exit Codes and PASS/FAIL Output

The CLI outputs a clear PASS or FAIL summary at the end for easy CI integration:

```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  PASS: All 1 scenarios passed
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

Exit codes:
- **0**: All scenarios passed
- **1**: One or more scenarios failed

This allows GitHub Actions to correctly detect and report failures

## When to Use What

| Scenario | When to Use |
|----------|-------------|
| `--scenario pr-check` | PR checks, quick validation |
| Full suite (no --scenario) | Main branch, release validation, weekly runs |
| `--scenario <specific>` | Debugging a specific capability |

## Adding New Capabilities to PR Check

If you need to test additional capabilities in PR checks, edit:
`backend/crates/qbit-evals/src/scenarios/pr_check.rs`

Guidelines:
1. Keep the total scenario under 90 seconds
2. Each turn should test one distinct capability
3. Add corresponding metrics to verify success
4. Test with all three providers before merging
