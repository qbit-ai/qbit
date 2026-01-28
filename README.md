<div align="center">

<img src="app-icon.png" width="128" height="128" alt="Qbit Logo">

# Qbit - Open-source agentic IDE


[![macOS](https://img.shields.io/badge/macOS-000000?style=flat&logo=apple&logoColor=white)](#quickstart)
[![Rust](https://img.shields.io/badge/Rust-000000?style=flat&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Tauri](https://img.shields.io/badge/Tauri_2-24C8D8?style=flat&logo=tauri&logoColor=white)](https://tauri.app/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

[Quickstart](#quickstart) • [Docs](docs/README.md) • [Development](docs/development.md)

<br>

<img src="docs/img/qbit-screenshot.png" alt="Qbit Screenshot" width="800">

</div>

---

## About Qbit

- Free and open-source.
- No account or subscription required. Bring your own keys.
- Fully transparent. No mysteries, no bullshit.
- Empowers users with information and full control.

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

---

## Linux Installation (Build from Source)

Qbit currently supports Linux installation **from source only**.

---

## Prerequisites

Make sure the following tools are installed on your system before proceeding.

### System packages

```bash
sudo apt update
sudo apt install -y \
  build-essential \
  curl \
  git \
  pkg-config \
  libssl-dev
```

> For non-Debian-based distros, install the equivalent packages using your system package manager.

---

### Rust toolchain

Qbit is written in Rust.

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

Verify:

```bash
rustc --version
cargo --version
```

---

### Just (command runner)

Qbit uses **just** to manage build and install commands.

```bash
cargo install just
```

Verify:

```bash
just --version
```

---

## Installation

### 1. Clone the repository

```bash
git clone https://github.com/qbit-ai/qbit.git
cd qbit
```

---

### 2. Development build (optional)

Build and run Qbit in development mode:

```bash
just dev
```

---

### 3. Install system-wide

Build and install Qbit to your system (typically `/usr/local/bin`):

```bash
just install
```

You may be prompted for `sudo` depending on your system configuration.

---

## Verification

After installation, verify Qbit is available:

```bash
qbit --version
```

For available commands:

```bash
qbit help
```

---

## Notes

* Linux installation is **source-based only**
* `just install` performs a release build before installing
* Ensure `$HOME/.cargo/bin` is in your `PATH`

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
