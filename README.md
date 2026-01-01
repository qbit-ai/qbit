<div align="center">

<img src="app-icon.png" width="128" height="128" alt="Qbit Logo">

# Qbit

**The open-source agentic terminal for developers who want to see how the magic works.**

[![macOS](https://img.shields.io/badge/macOS-000000?style=flat&logo=apple&logoColor=white)](#requirements)
[![Rust](https://img.shields.io/badge/Rust-000000?style=flat&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Tauri](https://img.shields.io/badge/Tauri_2-24C8D8?style=flat&logo=tauri&logoColor=white)](https://tauri.app/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

[Features](#features) • [Getting Started](#getting-started) • [Architecture](#architecture) • [Roadmap](#roadmap)

</div>

---

## Why Qbit?

AI coding assistants are powerful, but they're black boxes. You paste code, get answers, and hope for the best.

**Qbit flips that model.** It's a terminal with a transparent, modular agent system where you can see exactly what's happening: which agent is running, what tools it's using, and why it made each decision.

Built for developers who want AI assistance *and* understanding.

## Features

### 🤖 Specialized Sub-Agents

Not one monolithic AI — a team of focused agents, each optimized for specific tasks:

| Agent | Purpose |
|-------|---------|
| **Code Analyzer** | Deep semantic analysis via Tree-sitter: structure, patterns, metrics |
| **Code Explorer** | Maps codebases, traces dependencies, finds integration points |
| **Code Writer** | Implements features with patch-based editing for large changes |
| **Unified Diff Editor** | Applies surgical code edits using unified diff format |
| **Research Agent** | Web search and documentation lookup for external information |
| **Shell Executor** | Runs commands, builds, tests with security controls |

### ⚡ Composable Workflows

Chain agents together for complex tasks. The built-in `git_commit` workflow analyzes your changes and generates logical, well-organized commits automatically.

### 📚 Codebase Indexing

Index and manage multiple codebases with per-project memory files:

- **Multi-Codebase Support** — Add and index multiple repositories
- **Memory Files** — Associate CLAUDE.md or AGENTS.md files per project for persistent context
- **Settings UI** — Manage indexed codebases from the Settings panel

### 📦 Sidecar Context System

Automatic context capture and commit synthesis:

- **Session Tracking** — Captures agent interactions, file changes, and decisions
- **Context Panel** — Inspect session artifacts, patches, and synthesis metadata in-app
- **Staged Commits** — Auto-generates git format-patch files with conventional commit messages
- **Project Artifacts** — Proposes README.md and CLAUDE.md updates based on changes
- **LLM Synthesis** — Multiple backends (Vertex AI, OpenAI, Grok) or rule-based generation

### 🔧 Bring Your Own Model

Multi-provider support with easy configuration:

| Provider | Status |
|----------|--------|
| Anthropic (Vertex AI) | ✅ Supported |
| Anthropic (Direct API) | ✅ Supported |
| OpenRouter | ✅ Supported |
| OpenAI | ✅ Supported |
| Google Gemini | ✅ Supported |
| Groq | ✅ Supported |
| xAI (Grok) | ✅ Supported |
| Z.AI (GLM) | ✅ Supported |
| Ollama (Local) | ✅ Supported |

### 📦 Modern Terminal Features

- **Command Blocks** — Output organized into collapsible blocks with exit codes and timing
- **Split Panes** — Multi-pane layouts for side-by-side terminals
- **Multi-Tab Sessions** — Independent PTY per tab (`Cmd+T`)
- **Shell Integration** — Automatic command detection via OSC 133
- **Fullterm Mode** — Auto-switch to full xterm.js for interactive apps (vim, htop, ssh)
- **GPU Accelerated** — Smooth rendering powered by xterm.js

### 🎨 Customization

- **Theme Engine** — Theme presets with background image support
- **Flexible Layouts** — Toggleable panels and status indicators for active modes

## Getting Started

### Install (macOS)

1. Open the Releases section on GitHub and download the latest `Qbit` `.dmg`.
2. Open the `.dmg` and drag **Qbit** into `Applications`.
3. On first launch, if macOS blocks the app, go to **System Settings → Privacy & Security** and choose **Open Anyway**.

### Build from Source

#### Requirements

- macOS (Linux support planned)
- Node.js 18+
- pnpm
- Rust 1.70+
- [just](https://github.com/casey/just) (command runner)
- A POSIX shell (zsh, bash, fish, etc.)

#### Build & Run

```bash
# Clone the repo
git clone https://github.com/qbit-ai/qbit.git
cd qbit

# Install dependencies
pnpm install

# Run in development mode
just dev
```

> **Note:** This project uses [just](https://github.com/casey/just) as a command runner. Run `just --list` to see all available commands.

### Configure AI

Qbit supports multiple AI providers. Configure your preferred provider in `~/.qbit/settings.toml` or via environment variables.

**Quick start with Vertex AI:**

1. Set up [Vertex AI credentials](https://cloud.google.com/vertex-ai/docs/authentication) for your GCP project

2. Create `.env` in project root:
   ```bash
   GOOGLE_APPLICATION_CREDENTIALS=/path/to/service-account.json
   VERTEX_AI_PROJECT_ID=your-project-id
   VERTEX_AI_LOCATION=us-east5
   ```

**Alternative providers:** Set API keys in `settings.toml` or environment:
- `ANTHROPIC_API_KEY` — Direct Anthropic API
- `OPENAI_API_KEY` — OpenAI
- `OPENROUTER_API_KEY` — OpenRouter
- `GEMINI_API_KEY` — Google Gemini
- `GROQ_API_KEY` — Groq
- `XAI_API_KEY` — xAI (Grok)
- `ZAI_API_KEY` — Z.AI (GLM)

3. Select your model from the dropdown in the bottom bar

Settings are stored in `~/.qbit/settings.toml` (auto-generated on first run).

## Architecture

```
qbit/
├── frontend/               # React frontend
│   ├── components/         # UI components (shadcn + custom)
│   ├── hooks/              # Tauri event subscriptions
│   ├── lib/                # Typed invoke() wrappers
│   └── store/              # Zustand state (single file)
├── backend/crates/         # Rust workspace (29 modular crates)
│   ├── qbit/               # Main app crate (Tauri commands, CLI)
│   ├── qbit-ai/            # Agent orchestration, LLM clients
│   ├── qbit-core/          # Foundation types (zero internal deps)
│   ├── qbit-context/       # Token budget, context pruning
│   ├── qbit-pty/           # PTY management, OSC parsing
│   ├── qbit-sidecar/       # Context capture + commit synthesis
│   ├── qbit-tools/         # Tool system and registry
│   ├── qbit-workflow/      # Composable workflow engine
│   ├── qbit-sub-agents/    # Sub-agent definitions and execution
│   ├── qbit-llm-providers/ # Provider configuration types
│   ├── rig-anthropic-vertex/ # Vertex AI Anthropic provider
│   ├── rig-zai/            # Z.AI GLM provider
│   └── ...                 # 17 more infrastructure crates
└── docs/                   # Documentation
```

### Tech Stack

| Layer | Technology |
|-------|------------|
| Framework | [Tauri 2](https://tauri.app) |
| Frontend | React 19, TypeScript, Vite, Tailwind v4 |
| State | Zustand + Immer |
| Terminal | xterm.js, portable-pty, vte |
| Orchestration | [graph-flow](https://github.com/jkhoel/graph-flow) |
| UI Components | [shadcn/ui](https://ui.shadcn.com) |

### AI Tooling

- **File Operations** — Read, write, refactor with unified diff output
- **Code Analysis** — Semantic understanding via Tree-sitter (Rust, Python, TypeScript, Go, Java, Swift)
- **Shell Execution** — Controlled command execution with security allowlists
- **Context Management** — Smart token budgeting for efficient LLM usage
- **MCP Support** — Extend capabilities with Model Context Protocol tools

All tools run with workspace isolation and audit logging.

### CLI Binary

Qbit includes a headless CLI binary for scripting and automation:

```bash
# Build the CLI
cargo build -p qbit --features cli,local-tools --no-default-features --bin qbit-cli

# Run with a prompt
./target/debug/qbit-cli -e "your prompt here" --auto-approve
```

| Feature Flag | Description |
|--------------|-------------|
| `tauri` | GUI application (default) |
| `cli` | Headless CLI binary |
| `local-tools` | Local file/shell tools for CLI |
| `local-llm` | Local LLM via mistral.rs (Metal GPU) |

> **Note:** `tauri` and `cli` flags are mutually exclusive.

## Roadmap

| Feature | Status |
|---------|--------|
| PTY + multi-session | ✅ Done |
| Command blocks UI | ✅ Done |
| Shell integration (OSC 133) | ✅ Done |
| AI agentic loop | ✅ Done |
| Sub-agent system | ✅ Done |
| Composable workflows | ✅ Done |
| CLI binary (headless mode) | ✅ Done |
| Sidecar context capture (L1) | ✅ Done |
| Staged commits with LLM synthesis (L2) | ✅ Done |
| Project artifact generation (L3) | ✅ Done |
| Sidecar UI panel | ✅ Done |
| LLM evaluation framework | ✅ Done |
| Multi-provider support | ✅ Done |
| Codebase indexing + memory files | ✅ Done |
| Interactive commands (vim, htop) | ✅ Done |
| Downloadable releases | ✅ Done |
| Linux support | 📋 Planned |
| Plugin system | 📋 Planned |
| Custom keybindings | 📋 Planned |
| Theme engine | ✅ Done |

## Contributing

Qbit is early-stage and moving fast. Contributions welcome.

```bash
# Lint and format
just check      # Run all checks
just fix        # Auto-fix issues

# Run tests
just test       # All tests (frontend + Rust)
just test-fe    # Frontend only
just test-rust  # Rust only
```

## License

MIT — use it, fork it, make it yours.

---

<div align="center">

**[⬆ Back to top](#qbit)**

</div>
