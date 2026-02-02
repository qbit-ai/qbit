<div align="center">

<img src="app-icon.png" width="128" height="128" alt="Qbit Logo">

# Qbit - Open-source agentic IDE


[![macOS](https://img.shields.io/badge/macOS-000000?style=flat&logo=apple&logoColor=white)](#quickstart)
[![Rust](https://img.shields.io/badge/Rust-000000?style=flat&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Tauri](https://img.shields.io/badge/Tauri_2-24C8D8?style=flat&logo=tauri&logoColor=white)](https://tauri.app/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

[Quickstart](#quickstart) • [Docs](docs/README.md) • [Development](docs/development.md)

</div>

---

## About Qbit

- Free and open-source.
- No account or subscription required. Bring your own keys.
- Fully transparent. No mysteries, no bullshit.
- Empowers users with information and full control.

---

## Features

<table>
<tr>
<td width="50%" align="center">
<img src="docs/img/home.png" alt="Home" width="400"><br>
<b>Project Management</b><br>
<sub>Organize workspaces with project shortcuts and quick access</sub>
</td>
<td width="50%" align="center">
<img src="docs/img/timeline.png" alt="Timeline" width="400"><br>
<b>Unified Timeline</b><br>
<sub>Seamless conversation with AI, tool results, and terminal output</sub>
</td>
</tr>
<tr>
<td width="50%" align="center">
<img src="docs/img/model-selection.png" alt="Model Selection" width="400"><br>
<b>Model Selection</b><br>
<sub>Choose from multiple AI providers and models</sub>
</td>
<td width="50%" align="center">
<img src="docs/img/text-editor.png" alt="Text Editor" width="400"><br>
<b>Inline Text Editing</b><br>
<sub>Review and edit AI-generated content before applying</sub>
</td>
</tr>
<tr>
<td width="50%" align="center">
<img src="docs/img/tool-details.png" alt="Tool Details" width="400"><br>
<b>Tool Transparency</b><br>
<sub>Full visibility into every tool call and its execution</sub>
</td>
<td width="50%" align="center">
<img src="docs/img/sub-agent-details.png" alt="Sub-agent Details" width="400"><br>
<b>Sub-agent Execution</b><br>
<sub>Detailed view of sub-agent tasks and their results</sub>
</td>
</tr>
<tr>
<td colspan="2" align="center">
<img src="docs/img/git-integration.png" alt="Git Integration" width="400"><br>
<b>Git Integration</b><br>
<sub>Built-in version control with diff visualization</sub>
</td>
</tr>
</table>

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

## Linux Installation

### Install from release build

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

For source builds and Linux prerequisites, see [Getting started](docs/getting-started.md).

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
