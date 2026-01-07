<div align="center">

<img src="app-icon.png" width="128" height="128" alt="Qbit Logo">

# Qbit

**An open-source AI-powered terminal emulator for developers.**

[![macOS](https://img.shields.io/badge/macOS-000000?style=flat&logo=apple&logoColor=white)](#requirements)
[![Rust](https://img.shields.io/badge/Rust-000000?style=flat&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Tauri](https://img.shields.io/badge/Tauri_2-24C8D8?style=flat&logo=tauri&logoColor=white)](https://tauri.app/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

[Features](#features) • [Installation](#installation) • [Configuration](#configuration) • [Architecture](#architecture) • [Development](#development)

</div>

---

## Features

### AI Agent System

Qbit includes a specialized sub-agent system with five focused agents:

| Agent | Purpose |
|-------|---------|
| **Coder** | Applies code changes using unified diff format |
| **Analyzer** | Deep semantic analysis via tree-sitter: traces data flow, identifies dependencies |
| **Explorer** | Maps codebase structure, finds relevant files for tasks |
| **Researcher** | Web search and documentation lookup |
| **Executor** | Runs shell commands and manages multi-step operations |

### Multi-Provider Support

Connect to your preferred LLM provider:

| Provider | Configuration |
|----------|---------------|
| Anthropic (Vertex AI) | Service account JSON + project ID |
| Anthropic (Direct API) | `ANTHROPIC_API_KEY` |
| OpenAI | `OPENAI_API_KEY` |
| OpenRouter | `OPENROUTER_API_KEY` |
| Google Gemini | `GEMINI_API_KEY` |
| Groq | `GROQ_API_KEY` |
| xAI (Grok) | `XAI_API_KEY` |
| Z.AI (GLM) | `ZAI_API_KEY` |
| Ollama | Local server URL |

### AI Tools

Standard tools available to the agent:

- **File Operations**: `read_file`, `edit_file`, `write_file`, `create_file`, `delete_file`
- **Code Search**: `grep_file`, `list_files`, `ast_grep`, `ast_grep_replace`
- **Shell Execution**: `run_command` with PTY support
- **Web Access**: `web_fetch`, `web_search` (requires Tavily API key)
- **Planning**: `update_plan` for task tracking
- **Indexer Tools**: Semantic code analysis via tree-sitter

### Terminal Features

- **Command Blocks**: Output organized into collapsible blocks with exit codes
- **Split Panes**: Multi-pane layouts for side-by-side terminals
- **Multi-Tab Sessions**: Independent PTY per tab
- **Shell Integration**: Command detection via OSC 133 sequences
- **Fullterm Mode**: Auto-switches to full xterm.js for interactive apps (vim, htop, ssh)

### Context Management

- **Session Persistence**: Conversations saved and resumable
- **Context Compaction**: Automatic pruning when approaching token limits
- **Loop Detection**: Protection against agent infinite loops
- **Human-in-the-Loop**: Approval system with pattern learning

## Installation

### Download (macOS)

1. Download the latest `.dmg` from [Releases](https://github.com/qbit-ai/qbit/releases)
2. Open the `.dmg` and drag **Qbit** to Applications
3. On first launch: **System Settings → Privacy & Security → Open Anyway**

Builds are available for both Apple Silicon (ARM64) and Intel (x86_64).

> **Linux**: The app partially works on Linux but is not officially supported yet.

### Build from Source

#### Requirements

- macOS
- Node.js 20+
- pnpm
- Rust (stable toolchain)
- [just](https://github.com/casey/just) command runner

#### Build

```bash
git clone https://github.com/qbit-ai/qbit.git
cd qbit
pnpm install
just dev
```

Run `just --list` for all available commands.

## Configuration

Most settings can be configured through the Settings UI. Configuration is stored in `~/.qbit/settings.toml`.

## Architecture

```
qbit/
├── frontend/               # React 19 + TypeScript + Vite 7
│   ├── components/         # UI components (shadcn/ui + custom)
│   ├── hooks/              # Tauri event subscriptions
│   ├── lib/                # Typed invoke() wrappers
│   └── store/              # Zustand + Immer state
└── backend/crates/         # Rust workspace
    ├── qbit/               # Main app: Tauri commands, CLI
    ├── qbit-ai/            # Agent orchestration, LLM clients
    ├── qbit-core/          # Foundation types (zero deps)
    ├── qbit-sub-agents/    # Sub-agent definitions
    ├── qbit-tools/         # Tool system and registry
    ├── qbit-pty/           # PTY management
    ├── qbit-context/       # Token budget, pruning
    ├── qbit-workflow/      # Multi-step task pipelines
    ├── qbit-llm-providers/ # Provider abstractions
    └── rig-anthropic-vertex/ # Vertex AI Anthropic
```

### Tech Stack

| Layer | Technology |
|-------|------------|
| Framework | Tauri 2 |
| Frontend | React 19, TypeScript, Vite 7, Tailwind v4 |
| State | Zustand 5 + Immer |
| Terminal | xterm.js 5.5, portable-pty, vte |
| UI | shadcn/ui, Radix primitives |
| AI Integration | vtcode-core, rig-core |
| Workflows | graph-flow |

## Development

### Commands

```bash
just install          # Install dependencies
just build            # Production build
just check            # All checks (biome + clippy + fmt + tests)
just lint             # Lint frontend
just fmt              # Format all code
just clean            # Clean build artifacts
just eval             # Run evaluation scenarios
```

## License

MIT
