#!/bin/bash
# Claude Code session start hook for qbit
# Installs required development dependencies
#
# Environment detection:
# - CLAUDE_CODE_REMOTE="true" -> web/remote environment
# - CLAUDE_CODE_REMOTE unset  -> local CLI environment

set -e

# Only run dependency installation in remote/web environment
# Local users are expected to manage their own development setup
if [ "$CLAUDE_CODE_REMOTE" != "true" ]; then
    exit 0
fi

echo "Setting up qbit development environment (remote session)..."

# Install just (command runner) if not present
if ! command -v just &> /dev/null; then
    echo "Installing just..."
    cargo install just
fi

# Install gh (GitHub CLI) if not present
if ! command -v gh &> /dev/null; then
    echo "Installing gh (GitHub CLI)..."
    if command -v apt-get &> /dev/null; then
        curl -fsSL https://cli.github.com/packages/githubcli-archive-keyring.gpg | dd of=/usr/share/keyrings/githubcli-archive-keyring.gpg 2>/dev/null
        chmod go+r /usr/share/keyrings/githubcli-archive-keyring.gpg
        echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main" | tee /etc/apt/sources.list.d/github-cli.list > /dev/null
        apt-get update -qq
        apt-get install -y gh
    fi
fi

# Install GTK and Tauri dependencies if not present
if ! pkg-config --exists gdk-3.0 2>/dev/null; then
    echo "Installing GTK3 and Tauri dependencies..."
    if command -v apt-get &> /dev/null; then
        apt-get update -qq
        apt-get install -y \
            libgtk-3-dev \
            libwebkit2gtk-4.1-dev \
            libayatana-appindicator3-dev \
            librsvg2-dev
    fi
fi

# Install node dependencies if node_modules is missing
if [ ! -d "node_modules" ] && [ -f "package.json" ]; then
    echo "Installing node dependencies..."
    pnpm install
fi

echo "Development environment ready!"
