# Getting started

## Install

### Download (macOS)

1. Download the latest `.dmg` from GitHub Releases.
2. Open the `.dmg` and drag **Qbit** to Applications.
3. On first launch: **System Settings → Privacy & Security → Open Anyway**.

### Build from source

**Requirements**: macOS, Node.js 20+, pnpm, Rust (stable), `just`.

```bash
git clone https://github.com/qbit-ai/qbit.git
cd qbit
just install
just dev
```

## First run

- Settings live at `~/.qbit/settings.toml` (auto-generated on first run).
- Pick an LLM provider in the Settings UI or via `settings.toml`.

Next:
- [Configuration](configuration.md)
- [Providers](providers.md)
- [Workspaces](workspaces.md)
