<div align="center">

<img src="app-icon.png" width="128" height="128" alt="Qbit Logo">

# Qbit

**An AI-powered terminal that shows its work.**

[![macOS](https://img.shields.io/badge/macOS-000000?style=flat&logo=apple&logoColor=white)](#installation)
[![Rust](https://img.shields.io/badge/Rust-000000?style=flat&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Tauri](https://img.shields.io/badge/Tauri_2-24C8D8?style=flat&logo=tauri&logoColor=white)](https://tauri.app/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

[Features](#features) • [Installation](#installation) • [Configuration](#configuration) • [Development](#development) • [Documentation](#documentation)

<br>

<img src="docs/img/qbit-screenshot.png" alt="Qbit Screenshot" width="800">

</div>

---

## Why Qbit?

Developers don't trust magic. We trust logs, stack traces, and reproducible steps. Qbit applies the same principle to AI: every tool call, every file read, every reasoning step is visible and inspectable.

---

## Features

### Specialized Sub-Agents

Different tasks need different expertise. Qbit routes work to the right specialist:

| Agent | Purpose |
|-------|---------|
| **Coder** | Surgical code edits using unified diffs |
| **Analyzer** | Deep semantic analysis and dependency tracing |
| **Explorer** | Codebase mapping and implementation planning |
| **Researcher** | Web search and documentation lookup |
| **Executor** | Shell commands and system operations |

Each agent sees only the tools relevant to its job.

### Multi-Provider LLM Support

| Provider | Configuration |
|----------|---------------|
| Anthropic (Vertex AI) | `gcloud auth` or service account |
| Anthropic (Direct) | `ANTHROPIC_API_KEY` |
| OpenAI | `OPENAI_API_KEY` |
| OpenRouter | `OPENROUTER_API_KEY` |
| Google Gemini | `GEMINI_API_KEY` |
| Groq | `GROQ_API_KEY` |
| xAI (Grok) | `XAI_API_KEY` |
| Z.AI (GLM) | `ZAI_API_KEY` |
| Ollama | Local (no API key) |

See [Configuration Guide](docs/configuration.md) for detailed setup.

### AI Tools

- **File Operations**: read, edit, write, create, delete
- **Code Search**: grep, list files, AST-based search and replace
- **Shell Execution**: full PTY support
- **Web Access**: search and fetch (via Tavily)
- **Planning**: task tracking

### Terminal & UI

- Clean, minimal interface with theme support
- Command blocks with collapsible output
- Split panes and multi-tab sessions
- Fullterm mode for TUI apps (vim, htop, ssh)

### Agent Skills

Extend Qbit with custom skills via [agentskills.io](https://agentskills.io) specification. See [Agent Skills](docs/agent-skills.md).

### Safety & Control

- Human-in-the-loop approval for risky operations
- Pattern learning for trusted approvals
- Loop detection and context compaction
- Session persistence

---

## Installation

### Download (macOS)

1. Download the latest `.dmg` from [Releases](https://github.com/qbit-ai/qbit/releases)
2. Open the `.dmg` and drag **Qbit** to Applications
3. On first launch: **System Settings → Privacy & Security → Open Anyway**

### Build from Source

```bash
git clone https://github.com/qbit-ai/qbit.git
cd qbit
just install
just dev
```

Requirements: macOS, Node.js 20+, pnpm, Rust, [just](https://github.com/casey/just)

---

## Configuration

Settings: `~/.qbit/settings.toml` (auto-generated on first run)

### Quick Setup

```bash
# Option A: Anthropic Direct
echo "ANTHROPIC_API_KEY=sk-ant-..." >> .env

# Option B: OpenAI
echo "OPENAI_API_KEY=sk-..." >> .env

# Option C: Vertex AI
gcloud auth application-default login
# Then configure project_id in settings.toml
```

See [Configuration Guide](docs/configuration.md) for all providers and options.

---

## Development

```bash
just dev              # Full app with hot reload
just dev-fe           # Frontend only (mock Tauri)
just check            # Lint and format checks
just test             # All tests
```

Run `just --list` for all commands. See [Development Guide](docs/development.md).

---

## Documentation

| Document | Description |
|----------|-------------|
| [Architecture](docs/architecture.md) | Crate layers, project structure, tech stack |
| [Development](docs/development.md) | Commands, testing, adding features |
| [Configuration](docs/configuration.md) | Providers, environment, settings |
| [Event System](docs/event-system.md) | Terminal and AI event reference |
| [Contributing](docs/contributing.md) | Code conventions, commit format |
| [Agent Skills](docs/agent-skills.md) | Custom skill development |
| [Evaluations](docs/rig-evals.md) | Agent testing framework |

---

## Contributing

See [Contributing Guide](docs/contributing.md).

**Commit Format**: [Conventional Commits](https://www.conventionalcommits.org/) required.

```
feat(ai): add context window management
fix(terminal): prevent resize flicker
```

---

## License

MIT
