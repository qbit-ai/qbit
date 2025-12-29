# Versioning Strategy Implementation Plan

This plan implements unified version management for Qbit using `backend/Cargo.toml` as the single source of truth.

## Overview

**Goal:** Consolidate version management so that `backend/Cargo.toml` workspace version is the canonical source, with release-please automatically syncing versions to `package.json` on release.

**Source of Truth:** `backend/Cargo.toml` → `[workspace.package] version`

## Prerequisites

- [ ] Verify current version is consistent across all files (currently `0.1.0`)
- [ ] Ensure all changes are committed before starting

## Implementation Steps

### Step 1: Remove version from tauri.conf.json

**File:** `backend/crates/qbit/tauri.conf.json`

**Action:** Remove the `"version"` field from the JSON object. Tauri will automatically fall back to using the version from `Cargo.toml`.

**Before:**
```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "qbit",
  "version": "0.1.0",
  "identifier": "com.qbit.terminal",
  ...
}
```

**After:**
```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "qbit",
  "identifier": "com.qbit.terminal",
  ...
}
```

**Rationale:** Tauri v2 automatically uses the version from `Cargo.toml` when the `version` field is absent from `tauri.conf.json`.

---

### Step 2: Update release-please-config.json

**File:** `release-please-config.json` (project root)

**Action:** Replace the entire file contents with the following configuration:

```json
{
  "$schema": "https://raw.githubusercontent.com/googleapis/release-please/main/schemas/config.json",
  "packages": {
    "backend": {
      "release-type": "rust",
      "component": "qbit",
      "extra-files": [
        {
          "type": "json",
          "path": "../package.json",
          "jsonpath": "$.version"
        }
      ]
    }
  },
  "plugins": ["cargo-workspace"],
  "changelog-sections": [
    { "type": "feat", "section": "Features" },
    { "type": "fix", "section": "Bug Fixes" },
    { "type": "perf", "section": "Performance" },
    { "type": "refactor", "section": "Refactoring" },
    { "type": "docs", "section": "Documentation", "hidden": true },
    { "type": "chore", "section": "Miscellaneous", "hidden": true }
  ]
}
```

**Changes from current config:**
1. Added `"plugins": ["cargo-workspace"]` - ensures all workspace crate versions stay in sync
2. Updated `extra-files` to target `../package.json` instead of `tauri.conf.json` - syncs frontend package version
3. Removed the `tauri.conf.json` extra-file entry since version field is removed from that file

---

### Step 3: Verify workspace version inheritance

**File:** `backend/Cargo.toml`

**Action:** Verify the workspace defines the version:

```toml
[workspace.package]
version = "0.1.0"
```

**File:** `backend/crates/qbit/Cargo.toml`

**Action:** Verify the main crate inherits workspace version:

```toml
[package]
name = "qbit"
version.workspace = true
```

No changes needed if already configured this way.

---

### Step 4: Verify package.json version matches

**File:** `package.json` (project root)

**Action:** Ensure version matches the workspace version (`0.1.0`). No changes needed if already matching.

---

### Step 5: Test the build

**Action:** Run the following commands to verify the changes don't break the build:

```bash
# Test Rust build
cargo build -p qbit --features tauri

# Test Tauri build (includes frontend)
pnpm tauri build --debug
```

**Expected:** Build completes successfully with version inherited from Cargo.toml.

---

### Step 6: Commit changes

**Action:** Create a commit with the versioning changes:

```bash
git add backend/crates/qbit/tauri.conf.json release-please-config.json
git commit -m "chore: unify version management with Cargo.toml as source of truth

- Remove version from tauri.conf.json (Tauri uses Cargo.toml version)
- Add cargo-workspace plugin to release-please
- Configure extra-files to sync package.json version on release"
```

---

## Validation Checklist

After implementation, verify:

- [ ] `pnpm tauri build --debug` succeeds
- [ ] Built app shows correct version (check About dialog or bundle info)
- [ ] `cargo build -p qbit` succeeds
- [ ] No version field exists in `tauri.conf.json`
- [ ] `release-please-config.json` has `cargo-workspace` plugin
- [ ] `release-please-config.json` extra-files targets `../package.json`

## How Version Bumps Work After Implementation

1. Merge PRs with conventional commits (`feat:`, `fix:`, etc.) to `main`
2. release-please GitHub Action creates a Release PR with:
   - Updated `backend/CHANGELOG.md`
   - Bumped version in `backend/Cargo.toml` (workspace version)
   - Bumped version in `package.json` (via extra-files)
   - All workspace crates get the new version (via cargo-workspace plugin)
3. Merge the Release PR when ready
4. release-please creates a GitHub Release with version tag
5. `build-release.yml` triggers on release, builds artifacts for all platforms

## File Summary

| File | Role | Action |
|------|------|--------|
| `backend/Cargo.toml` | Source of truth | No change (already has workspace version) |
| `backend/crates/qbit/Cargo.toml` | Inherits version | No change (already uses `version.workspace = true`) |
| `backend/crates/qbit/tauri.conf.json` | App bundle config | Remove `version` field |
| `package.json` | Frontend package | Synced by release-please on release |
| `release-please-config.json` | Release automation | Add plugin + update extra-files |
| `.release-please-manifest.json` | Version tracking | Managed by release-please (no manual changes) |

## References

- [release-please documentation](https://github.com/googleapis/release-please)
- [Tauri v2 configuration](https://v2.tauri.app/reference/config/)
- [Cargo workspace inheritance](https://doc.rust-lang.org/cargo/reference/workspaces.html#the-package-table)
