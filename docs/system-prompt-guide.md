# System Prompt Development Guide

This document explains how to update, build, and maintain the Qbit system prompts. Use this as a reference when modifying agent behavior or adding new capabilities.

## Quick Reference

| What to Change | Where |
|----------------|-------|
| Main agent behavior | `qbit-ai/src/system_prompt.rs` → `build_system_prompt_with_contributions()` |
| Agent modes (Planning, AutoApprove) | `qbit-ai/src/system_prompt.rs` → `get_agent_mode_instructions()` |
| Sub-agent prompts | `qbit-sub-agents/src/defaults.rs` → `create_default_sub_agents()` |
| Sub-agent tool allocation | Same file, `.with_tools(vec![...])` on each definition |
| Dynamic contributions | `qbit-ai/src/contributors/` directory |
| Tool definitions (schemas) | `qbit-tools/src/definitions.rs` |

---

## Architecture Overview

The system prompt is composed at runtime from multiple sources:

```
┌────────────────────────────────────────────────────────────────┐
│            build_system_prompt_with_contributions()            │
├────────────────────────────────────────────────────────────────┤
│  1. BASE PROMPT (static, inline in system_prompt.rs)           │
│     ├── <identity> - Who is Qbit                               │
│     ├── <environment> - Workspace, date                        │
│     ├── <style> - Communication rules                          │
│     ├── # Workflow - 5-phase execution model                   │
│     ├── # Tool Selection - Tables and rules                    │
│     ├── # Delegation - When to use sub-agents                  │
│     ├── <security> - Credential handling                       │
│     └── <completion_checklist> - Verification requirements     │
│                                                                │
│  2. PROJECT INSTRUCTIONS (from memory file, e.g. CLAUDE.md)    │
│                                                                │
│  3. AGENT MODE INSTRUCTIONS (conditional)                      │
│     ├── Planning Mode: <planning_mode> read-only rules         │
│     └── AutoApprove Mode: <autoapprove_mode> caution notes     │
│                                                                │
│  4. DYNAMIC CONTRIBUTIONS (from PromptContributorRegistry)     │
│     ├── SubAgentPromptContributor → sub-agent docs             │
│     └── ProviderBuiltinToolsContributor → provider-specific    │
└────────────────────────────────────────────────────────────────┘
```

---

## Modifying the Base Prompt

### Location

`backend/crates/qbit-ai/src/system_prompt.rs`

### Structure

The base prompt is a Rust raw string literal inside `build_system_prompt_with_contributions()`. It uses `format!()` with these placeholders:

| Placeholder | Source |
|-------------|--------|
| `{workspace}` | `workspace_path.display()` |
| `{date}` | `Local::now().format("%Y-%m-%d")` |
| `{project_instructions}` | From `read_project_instructions()` |
| `{agent_mode_instructions}` | From `get_agent_mode_instructions()` |

### Section Tags

The prompt uses these semantic tags:

| Tag | Purpose |
|-----|---------|
| `<identity>` | Agent identity definition |
| `<environment>` | Runtime context |
| `<style>` | Communication rules |
| `<critical>` | Non-negotiable requirements |
| `<rule name="...">` | Named behavioral rules |
| `<security>` | Security boundaries |
| `<completion_checklist>` | Verification requirements |
| `<planning_mode>` | Planning mode restrictions |
| `<autoapprove_mode>` | AutoApprove mode guidance |

### Adding a New Section

1. Identify where it fits in the logical flow (workflow, tools, delegation, etc.)
2. Add the content as markdown or use semantic tags for critical rules
3. If it should be conditional, consider making it a contributor instead

**Example: Adding a new tool category**

```rust
// In the base prompt string, under # Tool Selection:

## Code Analysis

| Need | Tool | When to Use |
|------|------|-------------|
| **Structural search** | `ast_grep` | Finding code patterns |
| **Structural refactor** | `ast_grep_replace` | Pattern-based refactoring |

<rule name="ast-over-regex">
Use `ast_grep` instead of `grep_file` for code patterns.
</rule>
```

### Adding a Named Rule

Named rules (`<rule name="...">`) are used for important behavioral constraints:

```rust
<rule name="read-before-edit">
Before using `edit_file` or `write_file` on an existing file, you MUST read it first.
Edits without reading will fail or corrupt content.
</rule>
```

Existing rules:
- `read-before-edit` - File operation safety
- `ast-over-regex` - Prefer AST search for code patterns
- `explorer-first` - Always explore unfamiliar code first

---

## Modifying Agent Modes

### Location

`backend/crates/qbit-ai/src/system_prompt.rs` → `get_agent_mode_instructions()`

### Available Modes

```rust
pub enum AgentMode {
    Default,      // No additional instructions
    Planning,     // Read-only restrictions
    AutoApprove,  // Auto-approval cautions
}
```

### Updating Mode Instructions

Each mode returns a raw string that gets appended to the base prompt:

```rust
AgentMode::Planning => r#"
<planning_mode>
# Planning Mode Active

You are in READ-ONLY mode...

**Allowed**:
- `read_file`, `list_files`, `ast_grep` (structural search)
- `indexer_*` tools

**Forbidden**:
- `edit_file`, `write_file`, `create_file`
- Delegating to `coder`, `executor`
</planning_mode>
"#.to_string(),
```

### Adding a New Mode

1. Add the variant to `AgentMode` enum in `agent_mode.rs`
2. Add a match arm in `get_agent_mode_instructions()`
3. Update any mode-switching logic in `agent_bridge.rs`

---

## Modifying Sub-Agent Prompts

### Location

`backend/crates/qbit-sub-agents/src/defaults.rs`

### Structure

Each sub-agent is a `SubAgentDefinition` with:

```rust
SubAgentDefinition::new(
    "coder",                    // ID (used as sub_agent_coder tool)
    "Coder",                    // Display name
    "Description...",           // Shown in tool schema
    CODER_SYSTEM_PROMPT,        // Full system prompt (const or inline)
)
.with_tools(vec![               // Allowed tools
    "read_file".to_string(),
    "grep_file".to_string(),
    "ast_grep".to_string(),
    "ast_grep_replace".to_string(),
])
.with_max_iterations(20)        // Iteration limit
```

### Sub-Agent Prompt Template

Each sub-agent prompt typically includes:

```rust
const AGENT_SYSTEM_PROMPT: &str = r#"<identity>
You are a [role]. Your role is to [purpose].
</identity>

<capabilities>
- What you can do
</capabilities>

<workflow>
1. Step one
2. Step two
</workflow>

<output_format>
How to structure responses
</output_format>

<constraints>
- What you cannot do
- Tool restrictions
</constraints>"#;
```

### Current Sub-Agents

| ID | Purpose | Tools | Max Iter |
|----|---------|-------|----------|
| `coder` | Surgical code edits via diffs | read, list, grep, ast_grep, ast_grep_replace | 20 |
| `analyzer` | Deep code analysis | read, grep, ast_grep, indexer_* | 30 |
| `explorer` | Codebase navigation | read, list, grep, ast_grep, find, run_pty | 40 |
| `researcher` | Web research | web_search, web_fetch, read | 25 |
| `executor` | Shell command orchestration | run_pty, read, list_dir | 30 |

### Adding a Tool to a Sub-Agent

```rust
SubAgentDefinition::new(...)
.with_tools(vec![
    "read_file".to_string(),
    "ast_grep".to_string(),
    "ast_grep_replace".to_string(),  // ADD NEW TOOL
])
```

Also update the sub-agent's system prompt `<constraints>` section to mention the tool.

### Adding a New Sub-Agent

1. Define the system prompt (as const or inline)
2. Add to the vector in `create_default_sub_agents()`
3. Configure tools and max iterations
4. The tool `sub_agent_{id}` is automatically created

---

## Dynamic Contributions

### Location

`backend/crates/qbit-ai/src/contributors/`

### When to Use Contributors

Use contributors for prompt content that:
- Depends on runtime context (provider, available tools)
- Should be conditionally included
- Comes from external sources (sub-agent registry)

### Creating a Contributor

```rust
// In contributors/my_contributor.rs
pub struct MyContributor;

impl PromptContributor for MyContributor {
    fn contribute(&self, ctx: &PromptContext) -> Option<Vec<PromptSection>> {
        // Return None if nothing to contribute
        if !ctx.has_web_search {
            return None;
        }

        Some(vec![PromptSection::new(
            "my-section",          // Unique ID
            PromptPriority::Tools, // When to appear
            "Content here...",     // Prompt text
        )])
    }

    fn name(&self) -> &str {
        "MyContributor"
    }
}
```

### Priority Order

| Priority | Value | Use For |
|----------|-------|---------|
| Core | 0 | Identity, environment |
| Workflow | 100 | Behavior rules |
| Tools | 200 | Tool documentation |
| Features | 300 | Feature-specific |
| Provider | 400 | Provider-specific |
| Context | 500 | Runtime context |

### Registering Contributors

In `contributors/mod.rs`:

```rust
pub fn create_default_contributors(...) -> Vec<Arc<dyn PromptContributor>> {
    vec![
        Arc::new(SubAgentPromptContributor::new(sub_agent_registry)),
        Arc::new(ProviderBuiltinToolsContributor),
        Arc::new(MyContributor),  // ADD HERE
    ]
}
```

---

## Tool Definitions

### Location

`backend/crates/qbit-tools/src/definitions.rs`

### Adding Tool Documentation

Tool definitions include descriptions that become part of the tool schema sent to the LLM:

```rust
ToolDefinition::new("ast_grep")
    .with_description(
        "Search for code patterns using AST-based structural matching. 
         Use $VAR for single node, $$$VAR for multiple nodes."
    )
    .with_parameter(...)
```

The tool description is critical - it's the primary guidance the LLM has for using the tool.

---

## Testing Changes

### Unit Tests

```bash
# Test system prompt generation
cargo test -p qbit-ai system_prompt

# Test sub-agent definitions
cargo test -p qbit-sub-agents

# Test prompt parity (evals match main agent)
cargo test -p qbit-evals prompt
```

### Verification Checklist

After modifying prompts:

1. [ ] Run `cargo test` to ensure no breakage
2. [ ] Check that the prompt compiles (no syntax errors in raw strings)
3. [ ] Verify section tags are balanced (`<tag>...</tag>`)
4. [ ] Test with actual LLM to verify behavior changes
5. [ ] If adding tools, verify tool name matches definition in `definitions.rs`

### Common Issues

| Issue | Cause | Fix |
|-------|-------|-----|
| `edit_file` fails | Forgot to add `read_file` to sub-agent | Add to `.with_tools()` |
| Tool not found | Tool name mismatch | Check `definitions.rs` spelling |
| Mode not applying | Mode not propagated | Check `agent_bridge.rs` |
| Contribution missing | Contributor not registered | Add to `create_default_contributors()` |

---

## Best Practices

### Prompt Writing

1. **Be explicit** - LLMs follow literal instructions
2. **Use examples** - Show the desired pattern
3. **Negative examples** - State what NOT to do
4. **Gates over suggestions** - "MUST" beats "should"
5. **Keep rules atomic** - One rule, one concern

### Tool Allocation

1. **Minimal by default** - Only give sub-agents tools they need
2. **Read-only for analysis** - analyzer/explorer shouldn't have write tools
3. **Verify before adding** - Test that the tool actually helps the agent

### Section Organization

1. **Identity first** - Establish who the agent is
2. **Workflow before tools** - Process before mechanics
3. **Rules near usage** - Keep rules close to what they govern
4. **Security last** - Final guardrails before output

---

## Related Documentation

- [prompt-contributions.md](./prompt-contributions.md) - Dynamic contribution system details
- [system-prompt-design-input.md](./system-prompt-design-input.md) - Original design brief
- [agent-modes.md](./agent-modes.md) - Agent mode specifications
- [ast-grep-tools.md](./ast-grep-tools.md) - AST-based search/replace tools
