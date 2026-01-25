<div align="center">

<img src="app-icon.png" width="128" height="128" alt="Qbit Logo">

# Qbit

**An AI-powered terminal that shows its work.**

[![macOS](https://img.shields.io/badge/macOS-000000?style=flat&logo=apple&logoColor=white)](#quickstart)
[![Rust](https://img.shields.io/badge/Rust-000000?style=flat&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Tauri](https://img.shields.io/badge/Tauri_2-24C8D8?style=flat&logo=tauri&logoColor=white)](https://tauri.app/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

[Quickstart](#quickstart) • [Docs](docs/README.md) • [Development](docs/development.md)

<br>

<img src="docs/img/qbit-screenshot.png" alt="Qbit Screenshot" width="800">

</div>

---

## Why Qbit?

Developers don’t trust magic—we trust logs, stack traces, and reproducible steps. Qbit applies the same principle to AI:

- Tool calls are visible and inspectable (files read/edited, commands run, web queries)
- Planning is explicit, step-by-step
- You can see what context the agent used and why it did what it did

---

## Quickstart

### Install (macOS)

```bash
brew tap qbit-ai/tap
brew install --cask qbit
```

### Run from source

```bash
git clone https://github.com/qbit-ai/qbit.git
cd qbit
just install
just dev
```

### Configure a provider

Settings live at `~/.qbit/settings.toml`.

- See: [Providers](docs/providers.md)
- See: [Configuration](docs/configuration.md)

---

## Documentation

Start here:
- [Docs index](docs/README.md)
- [Getting started](docs/getting-started.md)
- [Configuration](docs/configuration.md)
- [Providers](docs/providers.md)

Using Qbit:
- [Workspaces](docs/workspaces.md)
- [Agent modes](docs/agent-modes.md)
- [Agent skills](docs/agent-skills.md)
- [Tool use](docs/tool-use.md)

Developing:
- [Development](docs/development.md)
- [Architecture](docs/architecture.md)

Evaluation / benchmarks:
- [Rig evals](docs/rig-evals.md)
- [SWE-bench](docs/swebench.md)
