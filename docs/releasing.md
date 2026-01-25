# Releasing Qbit

This document covers how to release new versions of Qbit.

## Overview

Qbit uses [release-please](https://github.com/googleapis/release-please) for automated releases. When commits are pushed to `main`, release-please creates/updates a PR that bumps versions and updates the changelog. Merging this PR triggers the release workflow.

## Quick Commands

```bash
just release-status   # Show latest release and pending PRs
just publish          # Merge pending release-please PR (interactive)
just release-manual 1.0.0  # Create manual release (bypasses release-please)
```

## Automated Release Process (Recommended)

### 1. Make Changes

Push commits to `main` with [Conventional Commits](https://www.conventionalcommits.org/) format:

```bash
git commit -m "feat: add new feature"      # Minor bump (0.1.0 → 0.2.0)
git commit -m "fix: resolve bug"           # Patch bump (0.1.0 → 0.1.1)
git commit -m "feat!: breaking change"     # Major bump (0.1.0 → 1.0.0)
git commit -m "chore: update deps"         # No release
```

### 2. Review Release PR

Release-please automatically creates a PR with:
- Version bumps in `Cargo.toml`, `package.json`, etc.
- Updated `CHANGELOG.md`

Check the PR:
```bash
just release-status
# Or visit the PR URL directly
```

### 3. Publish

```bash
just publish
```

This merges the release PR, which triggers:
1. **Build job**: Compiles for macOS (ARM64 + x64) and Linux
2. **Code signing**: Signs macOS binaries with Developer ID certificate
3. **Notarization**: Submits to Apple for notarization (macOS only)
4. **GitHub Release**: Creates release with DMG, AppImage, deb, rpm artifacts
5. **Homebrew update**: Updates the Homebrew cask formula

## Manual Release Process

Use this when you need to release outside the normal flow.

### 1. Ensure Clean State

```bash
git checkout main
git pull origin main
git status  # Should be clean
```

### 2. Update Version (if needed)

Edit version in:
- `package.json`
- `backend/Cargo.toml` (workspace version)
- `backend/crates/qbit/Cargo.toml`
- `backend/crates/qbit/tauri.conf.json`

Or use cargo-edit:
```bash
cd backend && cargo set-version 1.0.0
```

### 3. Update Changelog

Add entry to `CHANGELOG.md`:
```markdown
## [1.0.0] - 2026-01-24

### Added
- New feature X

### Fixed
- Bug Y

### Changed
- Behavior Z
```

### 4. Commit and Tag

```bash
git add -A
git commit -m "chore: release 1.0.0"
git tag -a v1.0.0 -m "Release v1.0.0"
git push origin main --tags
```

Or use the just command:
```bash
just release-manual 1.0.0
```

### 5. Monitor Release

```bash
gh run watch  # Watch the release workflow
gh release view v1.0.0  # View the created release
```

## What Gets Released

| Platform | Artifacts |
|----------|-----------|
| macOS ARM64 | `qbit_X.Y.Z_aarch64.dmg`, `qbit_aarch64.app.tar.gz` |
| macOS x64 | `qbit_X.Y.Z_x64.dmg`, `qbit_x64.app.tar.gz` |
| Linux x64 | `qbit_X.Y.Z_amd64.deb`, `qbit_X.Y.Z_amd64.AppImage`, `qbit-X.Y.Z-1.x86_64.rpm` |

## Distribution Channels

### GitHub Releases
Direct downloads from: https://github.com/qbit-ai/qbit/releases

### Homebrew (macOS)
```bash
brew tap qbit-ai/tap
brew install --cask qbit
```

The Homebrew cask is automatically updated when a new release is published.

## Code Signing & Notarization (macOS)

macOS builds are:
1. **Signed** with a Developer ID Application certificate
2. **Notarized** with Apple's notary service

This ensures users don't see "unidentified developer" warnings.

### Required Secrets (GitHub Environment: `macos-signing`)

| Secret | Description |
|--------|-------------|
| `APPLE_CERTIFICATE` | Base64-encoded .p12 certificate |
| `APPLE_CERTIFICATE_PASSWORD` | Password for the .p12 |
| `APPLE_SIGNING_IDENTITY` | e.g., `Developer ID Application: Name (TEAM_ID)` |
| `APPLE_ID` | Apple ID email for notarization |
| `APPLE_PASSWORD` | App-specific password for notarization |
| `APPLE_TEAM_ID` | Apple Developer Team ID |

### Local Signing (for testing)

```bash
APPLE_SIGNING_IDENTITY="Developer ID Application: Your Name (TEAM_ID)" \
APPLE_ID="your@email.com" \
APPLE_PASSWORD="xxxx-xxxx-xxxx-xxxx" \
APPLE_TEAM_ID="TEAM_ID" \
pnpm tauri build
```

## Troubleshooting

### Release PR not created
- Ensure commits use conventional commit format
- Check that release-please workflow ran: `gh run list --workflow=release-please.yml`

### Notarization stuck/failed
- First-time certificates may require manual Apple review
- Check status: `xcrun notarytool history --apple-id EMAIL --team-id TEAM_ID --password PASSWORD`
- Get logs: `xcrun notarytool log SUBMISSION_ID --apple-id EMAIL --team-id TEAM_ID --password PASSWORD`

### Homebrew not updated
- Check `HOMEBREW_TAP_TOKEN` secret is set
- Check update-homebrew workflow: `gh run list --workflow=update-homebrew.yml`

### Build failed
- Check workflow logs: `gh run view --log-failed`
- Common issues: missing dependencies, signing certificate issues
