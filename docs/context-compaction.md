# Context Compaction

Context compaction automatically manages the conversation history to prevent exceeding the LLM's context window limits. When the conversation grows too long, older messages are pruned while preserving recent context and important information.

## Overview

Large Language Models have finite context windows (e.g., 200K tokens for Claude). Long conversations with many tool calls can approach these limits. Context compaction:

1. **Monitors token usage** as the conversation progresses
2. **Emits warnings** when approaching capacity (default: 70%)
3. **Automatically prunes** old messages when exceeding threshold (default: 80%)
4. **Preserves recent turns** to maintain conversation coherence

## Configuration

Configure context compaction in `~/.qbit/settings.toml`:

```toml
[context]
# Enable automatic context compaction when approaching context window limits.
# When enabled, old messages are automatically pruned to stay within token budget.
enabled = true

# Utilization threshold (0.0 - 1.0) at which compaction is triggered.
# At 0.80 (80%), the system will start pruning old messages.
compaction_threshold = 0.80

# Number of recent turns (user + assistant exchanges) that are always protected
# from pruning. This ensures recent conversation context is never lost.
protected_turns = 2

# Cooldown between compaction operations in seconds.
# Prevents excessive pruning during rapid conversation.
cooldown_seconds = 60
```

### Settings Explained

| Setting | Default | Description |
|---------|---------|-------------|
| `enabled` | `true` | Master switch for context compaction |
| `compaction_threshold` | `0.80` | Utilization level that triggers pruning (80%) |
| `protected_turns` | `2` | Recent turns that are never pruned |
| `cooldown_seconds` | `60` | Minimum time between prune operations |

## How It Works

### Token Budget Tracking

The system estimates tokens for each message using a character-based heuristic (~4 characters per token). It tracks:

- User message tokens
- Assistant message tokens
- Tool result tokens
- System prompt tokens (reserved)
- Response buffer tokens (reserved)

### Threshold Levels

| Level | Utilization | Behavior |
|-------|-------------|----------|
| Normal | < 70% | No action |
| Warning | 70-79% | `context_warning` event emitted |
| Alert | 80-89% | Pruning triggered, `context_pruned` event emitted |
| Critical | ≥ 90% | Aggressive pruning with lower target utilization |

### Pruning Strategy

When pruning is triggered:

1. **Protect recent turns**: The last N turns (default: 2) are never removed
2. **Score messages semantically**: Messages are scored by importance
3. **Remove lowest-scored messages**: Starting from oldest, remove messages until under target utilization
4. **Target utilization**: Aims for 10% below the threshold (e.g., 70% if threshold is 80%)

### What Gets Pruned

Messages are scored and pruned based on:

- **Position**: Older messages score lower
- **Role**: User messages may score higher than assistant responses
- **Content type**: Tool results from read operations may be pruned before substantive responses

### What's Protected

- **Recent turns**: Configurable number of recent user+assistant exchanges
- **System prompt**: Never modified
- **Current turn**: The active request/response cycle

## Events

The system emits events that the frontend can display:

### `context_warning`
Emitted when utilization exceeds warning threshold (70%).

```json
{
  "type": "context_warning",
  "utilization": 0.75,
  "total_tokens": 150000,
  "max_tokens": 200000
}
```

### `context_pruned`
Emitted after messages are removed.

```json
{
  "type": "context_pruned",
  "messages_removed": 3,
  "utilization_before": 0.85,
  "utilization_after": 0.65,
  "tokens_freed": 40000
}
```

### `tool_response_truncated`
Emitted when a large tool response is truncated before adding to context.

```json
{
  "type": "tool_response_truncated",
  "tool_name": "read_file",
  "original_tokens": 50000,
  "truncated_tokens": 25000
}
```

## Frontend Display

The StatusBar shows context utilization with color-coded indicators:

| Color | Utilization | Meaning |
|-------|-------------|---------|
| Green | < 50% | Healthy |
| Yellow | 50-74% | Moderate usage |
| Orange | 75-84% | Approaching limit |
| Red | ≥ 85% | Near capacity |

When pruning occurs, a notification is shown:
> "Context Pruned: Removed 3 old messages to stay within context limits"

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    init_ai_session                          │
│  (loads ContextSettings from ~/.qbit/settings.toml)         │
└─────────────────────┬───────────────────────────────────────┘
                      │ ContextManagerConfig
                      ▼
┌─────────────────────────────────────────────────────────────┐
│                    AgentBridge                              │
│  (passes config to LLM client creation)                     │
└─────────────────────┬───────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────────┐
│                 create_*_components                         │
│  (creates ContextManager with config)                       │
└─────────────────────┬───────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────────┐
│                  ContextManager                             │
│  ┌─────────────────┐  ┌─────────────────┐                  │
│  │ TokenBudgetMgr  │  │  ContextPruner  │                  │
│  │ (tracks usage)  │  │ (removes msgs)  │                  │
│  └─────────────────┘  └─────────────────┘                  │
└─────────────────────┬───────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────────┐
│                   agentic_loop                              │
│  1. update_from_messages() - count tokens                   │
│  2. enforce_context_window() - check thresholds             │
│  3. Emit warning/pruned events if triggered                 │
└─────────────────────────────────────────────────────────────┘
```

## Testing

The feature includes comprehensive integration tests:

```bash
# Run all context tests
cargo test -p qbit-context

# Run compaction-specific tests
cargo test -p qbit-context test_compaction

# Run verbose proof test (shows actual pruning)
cargo test -p qbit-context test_compaction_verbose_proof -- --nocapture
```

### Test Coverage

| Test | What It Proves |
|------|----------------|
| `test_compaction_triggers_warning_at_threshold` | Warning emitted at 70% |
| `test_compaction_prunes_at_alert_threshold` | Messages removed at 80% |
| `test_compaction_preserves_protected_turns` | Recent turns preserved |
| `test_compaction_no_action_under_threshold` | No action below threshold |
| `test_compaction_disabled_does_nothing` | Disabled = no pruning |
| `test_compaction_reduces_utilization_to_target` | Utilization drops after prune |
| `test_compaction_verbose_proof` | Full proof with logs |

### Example Test Output

```
============================================================
CONTEXT COMPACTION PROOF TEST
============================================================

Configuration:
  - Max tokens: 5000
  - Warning threshold: 70%
  - Alert/Prune threshold: 80%
  - Protected turns: 2

Created 10 messages

BEFORE COMPACTION:
  - Total tokens: 4500
  - Utilization: 90.0%
  - Message count: 10
  - Alert level: Alert

>>> Calling enforce_context_window()...

AFTER COMPACTION:
  - Message count: 7 (was 10)

✓ PRUNING OCCURRED:
  - Messages removed: 3
  - Utilization before: 90.0%
  - Utilization after: 63.0%
  - Reduction: 27.0%

============================================================
PROOF COMPLETE: Context compaction is working!
============================================================
```

## Limitations

1. **Token estimation is approximate**: Uses character-based heuristics, not actual tokenizer
2. **System prompt not tracked**: System prompt tokens aren't counted in utilization
3. **No semantic importance scoring yet**: Currently uses position-based scoring
4. **Single session only**: Compaction doesn't persist across session restarts

## Future Improvements

- [ ] Integrate actual tokenizer for accurate counts
- [ ] Track system prompt tokens
- [ ] Semantic importance scoring using embeddings
- [ ] Summarization of pruned context instead of deletion
- [ ] User-configurable pruning strategies
