# Getting started

## Install

### Download (macOS)

1. Download the latest `.dmg` from GitHub Releases.
2. Open the `.dmg` and drag **Qbit** to Applications.
3. On first launch: **System Settings → Privacy & Security → Open Anyway**.

### Build from source (macOS)

**Requirements**: macOS, Node.js 20+, pnpm, Rust (stable), `just`.

```bash
git clone https://github.com/qbit-ai/qbit.git
cd qbit
just install
just dev
```

### Linux

#### Install from release build

Download and extract the release build:

```bash
curl -L -o qbit_x64.app.tar.gz \
  https://github.com/qbit-ai/qbit/releases/download/v0.2.13/qbit_x64.app.tar.gz

mkdir -p qbit-release

tar -xzf qbit_x64.app.tar.gz -C qbit-release
```

Add the binary to your `PATH` (adjust as needed for your system):

```bash
sudo install -m 755 qbit-release/qbit /usr/local/bin/qbit
```

#### Build from source

##### Prerequisites

Make sure the following tools are installed on your system before proceeding.

###### System packages

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

###### Rust toolchain

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

###### Just (command runner)

Qbit uses **just** to manage build and install commands.

```bash
cargo install just
```

Verify:

```bash
just --version
```

##### Installation

###### 1. Clone the repository

```bash
git clone https://github.com/qbit-ai/qbit.git
cd qbit
```

---

###### 2. Development build (optional)

Build and run Qbit in development mode:

```bash
just dev
```

---

###### 3. Install system-wide

Build and install Qbit to your system (typically `/usr/local/bin`):

```bash
just install
```

You may be prompted for `sudo` depending on your system configuration.

##### Verification

After installation, verify Qbit is available:

```bash
qbit --version
```

For available commands:

```bash
qbit help
```

##### Notes

* Release builds and source installs are both supported on Linux
* `just install` performs a release build before installing
* Ensure `$HOME/.cargo/bin` is in your `PATH`

## First run

- Settings live at `~/.qbit/settings.toml` (auto-generated on first run).
- Pick an LLM provider in the Settings UI or via `settings.toml`.

Next:
- [Configuration](configuration.md)
- [Providers](providers.md)
- [Workspaces](workspaces.md)
