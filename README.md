<div align="center">

<img src="app-icon.png" width="128" height="128" alt="Qbit Logo">

# Qbit

**A terminal where AI shows its work.**

[![macOS](https://img.shields.io/badge/macOS-000000?style=flat&logo=apple&logoColor=white)](#installation)
[![Rust](https://img.shields.io/badge/Rust-000000?style=flat&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Tauri](https://img.shields.io/badge/Tauri_2-24C8D8?style=flat&logo=tauri&logoColor=white)](https://tauri.app/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

<br>

<img src="docs/img/qbit-screenshot.png" alt="Qbit Screenshot" width="800">

</div>

---

## The Problem

AI coding assistants are black boxes. They read your files, run commands, and produce output—but you're left wondering: *What did it actually do? Why did it make that decision? What context did it use?*

When something breaks, you're debugging two things: your code and the AI's mysterious reasoning.

## What Qbit Does Differently

Qbit is a terminal emulator with an integrated AI that **exposes every step** of its operation:

- **See what context was used** — Not just "I read the file," but which lines, why, and how it fit the token budget
- **Watch agent handoffs** — Work routes to specialized sub-agents (coder, analyzer, researcher); you see who's doing what
- **Approve before execution** — Shell commands and file writes require your approval (or auto-approve patterns you trust)
- **Trace reasoning** — Extended thinking is visible, not hidden behind a loading spinner

```
┌─────────────────────────────────────────────────────────────────┐
│ You: Fix the type error in auth.ts                              │
├─────────────────────────────────────────────────────────────────┤
│ [Thinking] (264 chars)                                    [▼]   │
│ ─────────────────────────────────────────────────────────────── │
│ Routing to: Coder                                               │
│ ─────────────────────────────────────────────────────────────── │
│ [Tool] read_file: src/auth.ts (lines 1-150)                     │
│ [Tool] grep_file: "UserSession" → 3 matches                     │
│ ─────────────────────────────────────────────────────────────── │
│ [Approval Required]                                             │
│ edit_file: src/auth.ts                                          │
│ @@ -47,3 +47,3 @@                                                │
│ -  const session: UserSession = null;                           │
│ +  const session: UserSession | null = null;                    │
│                                         [Approve] [Reject]      │
└─────────────────────────────────────────────────────────────────┘
```

---

## Why Not Just Use [Other Tool]?

Honest comparisons:

| If you want... | Use this | Why |
|----------------|----------|-----|
| Claude in a terminal | [Claude Code](https://github.com/anthropics/claude-code) | Official, well-supported, simpler |
| IDE integration | [Cursor](https://cursor.sh), [Copilot](https://github.com/features/copilot) | Mature, battle-tested |
| Local models only | [Ollama](https://ollama.ai) + your shell | Simpler, no overhead |
| **To see exactly what the AI is doing** | **Qbit** | That's the point |

Qbit isn't trying to be the fastest or most magical. It's for developers who:
- Want to **learn** from AI suggestions, not just accept them
- Need to **audit** what an AI agent did in their codebase
- Are tired of "trust me, I fixed it" black boxes
- Want to use **multiple providers** without changing tools

---

## Features

### Specialized Sub-Agents

Instead of one monolithic AI, tasks route to specialists:

| Agent | Job | Why It Matters |
|-------|-----|----------------|
| **Coder** | Applies code changes via unified diffs | Focused toolset, no permission creep |
| **Analyzer** | Traces data flow, explains dependencies | Deep semantic understanding |
| **Explorer** | Maps codebase structure | Finds relevant files before editing |
| **Researcher** | Web search, documentation lookup | External knowledge when needed |
| **Executor** | Runs shell commands | Isolated command execution |

You see which agent is active and what tools it has access to.

### Multi-Provider LLM Support

No vendor lock-in. Switch providers mid-conversation.

| Provider | Setup |
|----------|-------|
| **Anthropic** (Direct) | `ANTHROPIC_API_KEY` |
| **Anthropic** (Vertex AI) | `gcloud auth` or service account |
| **OpenAI** | `OPENAI_API_KEY` |
| **Google Gemini** | `GEMINI_API_KEY` |
| **OpenRouter** | `OPENROUTER_API_KEY` |
| **Groq** | `GROQ_API_KEY` |
| **xAI (Grok)** | `XAI_API_KEY` |
| **Ollama** | Local, no API key |

Use local models for sensitive codebases. Use cloud models for complex tasks. Your choice.

### Safety Controls

- **Human-in-the-loop** — Approve shell commands and file modifications before execution
- **Pattern learning** — Auto-approve commands you've trusted before (e.g., `git status`, `npm test`)
- **Loop detection** — Automatic protection against agent infinite loops
- **Context visibility** — See token budget usage and what got pruned

### Terminal Features

- **Split panes** — Multiple terminals side-by-side
- **Command blocks** — Output organized into collapsible sections with exit codes
- **Fullterm mode** — Auto-switches to raw terminal for TUI apps (vim, htop, ssh)
- **Session persistence** — Resume conversations where you left off
- **Themes** — Multiple themes included, easy to add custom ones

---

## Installation

### Download (macOS)

1. Get the latest `.dmg` from [Releases](https://github.com/anthropics/qbit/releases)
2. Drag to Applications
3. First launch: **System Settings → Privacy & Security → Open Anyway** (unsigned builds)

Apple Silicon (ARM64) and Intel (x86_64) builds available.

### Build from Source

**Requirements**: macOS, Node.js 20+, pnpm, Rust (stable), [just](https://github.com/casey/just)

```bash
git clone https://github.com/anthropics/qbit.git
cd qbit
pnpm install
just dev
```

### Headless CLI

For scripting or CI environments:

```bash
cargo build -p qbit --features cli --no-default-features --bin qbit-cli
./target/debug/qbit-cli -e "explain this codebase" --auto-approve
```

---

## Quick Configuration

Settings live in `~/.qbit/settings.toml` (auto-generated on first run).

**Anthropic (simplest)**
```bash
export ANTHROPIC_API_KEY=sk-ant-...
```

**Vertex AI (if you have GCP)**
```bash
gcloud auth application-default login
```
Then in `~/.qbit/settings.toml`:
```toml
[ai]
default_provider = "vertex_ai"

[ai.vertex_ai]
project_id = "your-project-id"
location = "us-east5"
```

**Ollama (fully local)**
```bash
ollama serve  # Just have it running
```

**Web search (optional)**
```bash
export TAVILY_API_KEY=tvly-...
```

Most settings are also configurable through the in-app Settings UI.

---

## Architecture

Qbit is a Tauri 2 app: React frontend, Rust backend.

```
qbit/
├── frontend/               # React 19 + TypeScript + Vite
│   ├── components/         # UI (shadcn/ui + custom)
│   ├── hooks/              # Event subscriptions
│   ├── lib/                # Typed IPC wrappers
│   └── store/              # Zustand state
│
└── backend/crates/         # Rust workspace (29 crates)
    ├── qbit/               # App entry point, Tauri commands
    ├── qbit-ai/            # Agent loop, LLM abstraction
    ├── qbit-core/          # Foundation types (zero deps)
    ├── qbit-sub-agents/    # Sub-agent definitions
    ├── qbit-tools/         # Tool implementations
    ├── qbit-pty/           # Terminal/PTY management
    ├── qbit-context/       # Token budget, pruning
    ├── qbit-hitl/          # Approval system
    └── ...                 # 20+ more infrastructure crates
```

### Why This Stack?

- **Tauri over Electron** — ~10MB binary vs 150MB+, native performance, smaller attack surface
- **Rust backend** — Memory safety, fearless concurrency, sane dependency management
- **29 crates** — Strict layering prevents spaghetti; each crate has one job
- **React 19** — Concurrent rendering for smooth streaming UI

### Crate Layers

```
Layer 4 (App):        qbit ─────────────────────────────────────┐
                          │                                     │
Layer 3 (Domain):     qbit-ai (orchestration) ──────────────────┤
                          │                                     │
Layer 2 (Infra):      25 crates (tools, pty, context, etc.) ────┤
                          │                                     │
Layer 1 (Foundation): qbit-core (zero internal deps) ───────────┘
```

Dependencies only flow downward. No cycles. Easy to test in isolation.

---

## Limitations & Roadmap

### Current Limitations

- **macOS only** — Windows/Linux support requires PTY abstraction work
- **No IDE integration** — This is a standalone terminal, not a plugin
- **Unsigned builds** — You'll need to bypass Gatekeeper on first launch
- **No mobile** — Desktop-focused

### Planned

- [ ] Windows and Linux support
- [ ] Code signing for macOS builds
- [ ] Plugin system for custom tools
- [ ] Team features (shared sessions, audit logs)

We're not trying to build everything. The focus is on **visibility into AI operations**.

---

## Development

```bash
just dev              # Full app with hot reload
just dev-fe           # Frontend only (no backend, uses mocks)
just check            # Lint + typecheck + clippy
just test             # All tests
just test-e2e         # Playwright E2E tests
```

The frontend runs standalone with full mock support—useful for UI work without LLM costs.

### Adding a New Tool

1. Define schema in `backend/crates/qbit-ai/src/tool_definitions.rs`
2. Implement executor in `backend/crates/qbit-ai/src/tool_executors.rs`
3. Register in the tool registry
4. Handle events in `frontend/hooks/useAiEvents.ts`

See `docs/tool-use.md` for details.

---

## Contributing

We use conventional commits:

```
feat(ai): add context window visualization
fix(terminal): prevent resize flicker on split
docs: clarify sub-agent routing behavior
```

CI runs lint, tests, and AI evaluation scenarios on every PR.

Read `AGENTS.md` for agent system internals.

---

## FAQ

**Q: How is this different from Claude Code?**

Claude Code is an official CLI that runs Claude in your terminal. Qbit is a terminal emulator with AI built in. Key differences: Qbit shows you the full agent execution trace, routes work to specialized sub-agents, and supports multiple LLM providers.

**Q: Can I use this with private/local models only?**

Yes. Configure Ollama and use whatever models you've pulled locally. No data leaves your machine.

**Q: Why approve every command? Isn't that slow?**

Pattern learning reduces friction over time. Once you've approved `git status` a few times, Qbit auto-approves it. You control the trust boundary.

**Q: Is this production-ready?**

It's alpha software. We use it daily for development, but expect rough edges. File issues when you hit them.

**Q: Why Rust?**

Terminal emulators are surprisingly complex (PTY handling, ANSI parsing, concurrent I/O). Rust's safety guarantees and performance characteristics are well-suited. Also, Tauri's Rust backend means we can do heavy lifting without IPC overhead.

---

## License

MIT

---

<div align="center">

**[Documentation](docs/)** · **[Issues](https://github.com/anthropics/qbit/issues)** · **[Discussions](https://github.com/anthropics/qbit/discussions)**

</div>
