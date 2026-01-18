# Agent Skills

This document explains the Agent Skills system that provides directory-based extensions to the AI agent with specialized instructions and capabilities.

## Overview

Agent Skills follow the [agentskills.io](https://agentskills.io) specification. Skills are self-contained directories that provide specialized instructions to the AI agent, enabling domain-specific behaviors without modifying the core system.

Skills support:
- **Manual invocation** via `/skill-name` slash commands
- **Automatic matching** based on user prompt content (keyword-based)
- **Optional subdirectories** for scripts, references, and assets
- **Tool restrictions** via `allowed-tools` frontmatter

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                         Skill System                                │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  ┌────────────────────────┐    ┌────────────────────────────────┐   │
│  │   ~/.qbit/skills/      │    │   <project>/.qbit/skills/      │   │
│  │   (Global skills)      │    │   (Local skills - override)    │   │
│  └───────────┬────────────┘    └───────────────┬────────────────┘   │
│              │                                  │                   │
│              └──────────┬───────────────────────┘                   │
│                         │                                           │
│                         ▼                                           │
│              ┌─────────────────────┐                                │
│              │   qbit-skills crate │                                │
│              │   - Discovery       │                                │
│              │   - Parsing         │                                │
│              │   - Matching        │                                │
│              └──────────┬──────────┘                                │
│                         │                                           │
│           ┌─────────────┴─────────────┐                             │
│           ▼                           ▼                             │
│  ┌─────────────────┐        ┌──────────────────────┐                │
│  │ Slash Commands  │        │ SkillsPromptContrib. │                │
│  │ (manual invoke) │        │ (auto-matching)      │                │
│  └─────────────────┘        └──────────────────────┘                │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

## Directory Structure

Skills are discovered from two locations:

```
~/.qbit/skills/                   # Global skills (user-level)
  skill-name/
    SKILL.md                      # Required: YAML frontmatter + instructions
    scripts/                      # Optional: executable scripts
    references/                   # Optional: reference documents
    assets/                       # Optional: assets (images, etc.)

<project>/.qbit/skills/           # Local skills (project-level, override global)
  skill-name/
    SKILL.md
    ...
```

Local skills override global skills with the same name.

## SKILL.md Format

Each skill directory must contain a `SKILL.md` file with YAML frontmatter:

```markdown
---
name: skill-name
description: Short description of what the skill does (1-1024 chars)
license: MIT                         # Optional
compatibility: Claude 3.5+           # Optional (1-500 chars)
allowed-tools: read_file write_file  # Optional: space-delimited tool names
metadata:                            # Optional: arbitrary key-value pairs
  author: your-name
  version: 1.0.0
---

Your skill instructions here.

This markdown content is sent to the AI agent when the skill is invoked.
You can include any instructions, examples, or context the agent needs.
```

### Frontmatter Fields

| Field | Required | Description |
|-------|----------|-------------|
| `name` | Yes | Skill name (must match directory name) |
| `description` | Yes | Short description (1-1024 chars) |
| `license` | No | License identifier (e.g., "MIT", "Apache-2.0") |
| `compatibility` | No | Compatibility info (1-500 chars) |
| `allowed-tools` | No | Space-delimited list of allowed tool names |
| `metadata` | No | Arbitrary key-value pairs |

### Skill Name Rules

Skill names must follow these rules:
- 1-64 characters
- Lowercase alphanumeric characters and hyphens only
- No consecutive hyphens (`--`)
- No leading or trailing hyphens

Valid examples: `git-commit`, `code-review`, `my-skill-123`

Invalid examples: `Test-Skill` (uppercase), `test--skill` (consecutive hyphens), `-test` (leading hyphen)

## Usage

### Manual Invocation

Type `/` in the input field to see available slash commands (prompts and skills):

1. Skills display with a puzzle icon and their description
2. Tab completes the command name
3. Enter executes the skill
4. Arguments can be appended: `/skill-name some arguments`

### Automatic Matching

Skills can be automatically matched to user prompts based on keywords extracted from the skill name and description. The matching algorithm:

1. Extracts keywords from skill name (split by hyphens) and description
2. Filters out common stopwords
3. Matches keywords against the user's prompt
4. Returns skills above a confidence threshold (0.4 by default)

When skills are matched, their instructions are injected into the system prompt under an "Active Skills" section.

## Key Components

### qbit-skills Crate (`backend/crates/qbit-skills/`)

The core skill library providing:

- **Discovery** (`discovery.rs`) - Scans global and local skill directories
- **Parsing** (`parser.rs`) - Parses SKILL.md files with YAML frontmatter
- **Matching** (`matcher.rs`) - Keyword-based skill matching algorithm
- **Types** (`types.rs`) - SkillInfo, SkillMetadata, MatchedSkill, etc.

### Tauri Commands (`backend/crates/qbit/src/commands/skills.rs`)

Thin wrappers around the qbit-skills crate:

```rust
// List available skills
pub async fn list_skills(working_directory: Option<String>) -> Result<Vec<SkillInfo>>

// Read full SKILL.md content
pub async fn read_skill(path: String) -> Result<String>

// Read only the body (instructions)
pub async fn read_skill_body(path: String) -> Result<String>

// List files in a skill subdirectory
pub async fn list_skill_files(skill_path: String, subdir: String) -> Result<Vec<SkillFileInfo>>

// Read a specific file from a skill
pub async fn read_skill_file(skill_path: String, relative_path: String) -> Result<String>
```

### Prompt Contributor (`backend/crates/qbit-ai/src/contributors/skills.rs`)

Injects skill content into the system prompt:

```rust
pub struct SkillsPromptContributor;

impl PromptContributor for SkillsPromptContributor {
    fn contribute(&self, ctx: &PromptContext) -> Option<Vec<PromptSection>> {
        // 1. Generate summary of available skills
        // 2. Inject full bodies for matched skills
    }
}
```

### Frontend Integration

- **useSlashCommands hook** (`frontend/hooks/useSlashCommands.ts`) - Loads and merges prompts and skills
- **SlashCommandPopup** (`frontend/components/SlashCommandPopup/`) - Unified popup for prompts and skills
- **Tauri wrappers** (`frontend/lib/tauri.ts`) - `listSkills`, `readSkillBody`, etc.

## Example Skill

Here's a complete example of a git commit skill:

```
~/.qbit/skills/git-commit/
├── SKILL.md
├── scripts/
│   └── validate-message.sh
└── references/
    └── conventional-commits.md
```

**SKILL.md:**

```markdown
---
name: git-commit
description: Create git commits following conventional commit format
license: MIT
compatibility: Git 2.0+
allowed-tools: bash read_file write_file
metadata:
  author: qbit-team
  version: 1.0.0
---

You are a git commit assistant. Help the user create well-structured commits.

## Commit Message Format

Use the conventional commits format:
- `feat`: A new feature
- `fix`: A bug fix
- `docs`: Documentation changes
- `style`: Code style changes (formatting, etc.)
- `refactor`: Code refactoring
- `test`: Adding or updating tests
- `chore`: Maintenance tasks

## Guidelines

1. Keep the subject line under 72 characters
2. Use the imperative mood ("Add feature" not "Added feature")
3. Include a body for complex changes
4. Reference issues when applicable

## Example

```
feat(auth): add OAuth2 login support

Implement OAuth2 authentication flow with Google and GitHub providers.
This replaces the legacy session-based authentication.

Closes #123
```
```

## Precedence

When resolving slash commands, the following precedence applies:

1. **Prompts** (`.qbit/prompts/*.md`) - Highest priority
2. **Local skills** (`<project>/.qbit/skills/`) - Override global
3. **Global skills** (`~/.qbit/skills/`) - Lowest priority

If a prompt and skill have the same name, the prompt takes precedence.

## Skill Matching Algorithm

The `SkillMatcher` uses a conservative keyword-based approach:

```rust
pub struct SkillMatcher {
    pub min_score: f32,    // Default: 0.4
    pub max_skills: usize, // Default: 3
}
```

**Scoring:**
- Skill name in prompt: +0.5 (e.g., "use git-commit" matches "git-commit")
- Keyword match: +0.15 per match (up to 3 matches = 0.45)
- Maximum score: 1.0

**Keywords are extracted from:**
- Skill name (split by hyphens)
- Description words (filtered for stopwords, min 3 chars)

## Security

The skill system includes security measures:

- **Path traversal protection** - `read_skill_file` validates paths stay within the skill directory
- **Subdirectory restrictions** - Only `scripts/`, `references/`, and `assets/` subdirectories are accessible
- **Name validation** - Skill names must match directory names

## Testing

Run skill-related tests:

```bash
# Unit tests
cargo test -p qbit-skills
cargo test -p qbit -- skills

# E2E tests
pnpm test:e2e -- slash-commands
```

## Related Files

- `backend/crates/qbit-skills/` - Core skills library
  - `src/lib.rs` - Public API and error types
  - `src/types.rs` - SkillInfo, SkillMetadata, SkillFrontmatter
  - `src/discovery.rs` - Skill discovery from directories
  - `src/parser.rs` - SKILL.md parsing and validation
  - `src/matcher.rs` - Keyword-based skill matching
- `backend/crates/qbit/src/commands/skills.rs` - Tauri command wrappers
- `backend/crates/qbit-ai/src/contributors/skills.rs` - Prompt contribution
- `backend/crates/qbit-core/src/prompt.rs` - PromptSkillInfo, PromptMatchedSkill
- `frontend/hooks/useSlashCommands.ts` - Unified slash command loading
- `frontend/components/SlashCommandPopup/` - UI for slash commands
- `frontend/lib/tauri.ts` - Frontend skill invoke wrappers
