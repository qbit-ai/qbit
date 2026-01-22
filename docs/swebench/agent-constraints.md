# Agent Constraints

This document details exactly what the agent can and cannot do during SWE-bench evaluation. These constraints ensure fair evaluation that mirrors how a human developer would approach the problem.

## Information Available to the Agent

### Provided Information

| Information | Source | Example |
|-------------|--------|---------|
| Repository name | Instance metadata | `django/django` |
| Version | Instance metadata | `3.0` |
| Problem statement | GitHub issue text | "HttpResponse doesn't handle memoryview..." |
| Failing test names | FAIL_TO_PASS field | `["test_memoryview_content"]` |
| Hints (optional) | hints_text field | "Try looking at make_bytes" |
| Full source code | Cloned repository | All files at base_commit |

### Hidden Information

| Information | Why Hidden |
|-------------|------------|
| Gold patch | Would reveal the solution |
| Git history after base_commit | Contains fix commits |
| PASS_TO_PASS test names | Used only for regression checking |
| Other instances' solutions | Cross-contamination |

## Access Restrictions

### 1. No Git History Access

The agent works on a repository checked out at `base_commit`. Git history is not directly accessible because:

1. **Test tool restriction** - Agent uses `run_swebench_test` instead of direct docker exec
2. **No git commands in prompt** - Agent isn't told how to access history
3. **Working directory isolation** - Agent sees `/workspace/repo`, not `.git` internals

```rust
// From tools.rs - Why we use a custom tool
/// Get the tool definition for the SWE-bench test runner.
///
/// This tool allows the agent to run tests in the Docker container
/// without giving it direct access to docker exec (which would allow
/// accessing git history containing the fix commits).
pub fn get_swebench_test_tool_definition() -> ToolDefinition {
    // ...
}
```

### 2. Test Files are Read-Only

Before the agent starts, test files are protected:

```rust
// From repo.rs
pub fn protect_test_files(&self, repo_path: &Path) -> Result<usize> {
    for entry in walkdir::WalkDir::new(repo_path) {
        if is_test_file(&path) {
            // Remove write permissions (mode &= !0o222)
            let mode = metadata.permissions().mode() & !0o222;
            std::fs::set_permissions(&path, Permissions::from_mode(mode))?;
        }
    }
}
```

**Test file patterns that are protected:**
- `tests/*`
- `test/*`
- `*/tests/*`
- `*/test/*`
- `test_*.py`
- `*_test.py`

### 3. Test Files Never Synced to Testbed

When the agent runs tests, only source files are synced:

```bash
# From tools.rs - build_test_command
is_test_file() {
    local file="$1"
    case "$file" in
        tests/*|test/*|*/tests/*|*/test/*|test_*.py|*_test.py)
            return 0  # Is a test file - SKIP
            ;;
        *)
            return 1  # Not a test file - sync it
            ;;
    esac
}

for file in $(git diff --name-only HEAD); do
    if is_test_file "$file"; then
        continue  # NEVER sync test files
    fi
    cp "$file" "/testbed/$file"
done
```

## Available Tool: `run_swebench_test`

The agent has exactly one tool for interacting with the test environment:

```json
{
    "name": "run_swebench_test",
    "description": "Run tests in the SWE-bench Docker test environment",
    "parameters": {
        "test_path": {
            "type": "string",
            "description": "Test file, function, or pattern"
        },
        "verbose": {
            "type": "boolean",
            "description": "Enable verbose output"
        }
    }
}
```

### Valid Test Path Formats

| Format | Example |
|--------|---------|
| Full file | `tests/test_example.py` |
| Specific test | `tests/test_example.py::test_function` |
| Test class | `tests/test_example.py::TestClass` |
| Pattern | `-k test_memoryview` |
| Django format | `admin_views.tests.AdminViewBasicTest.test_login` |

### Input Validation (Security)

```rust
// From tools.rs
fn validate_test_path(path: &str) -> Result<()> {
    // Forbidden characters that could be used for injection
    let forbidden_chars = [
        '`', '$', ';', '&', '|', '>', '<', '!', '\\', '\n', '\r', '\'', '"',
    ];

    for c in forbidden_chars {
        if path.contains(c) {
            anyhow::bail!("Forbidden character '{}' in test path", c);
        }
    }

    // No command substitution
    if path.contains("$(") || path.contains("${") {
        anyhow::bail!("Command substitution not allowed");
    }

    // No path traversal
    if path.contains("..") {
        anyhow::bail!("Path traversal not allowed");
    }

    // Length limit
    if path.len() > 1000 {
        anyhow::bail!("Test path too long (max 1000 characters)");
    }

    Ok(())
}
```

## What the Agent CAN Do

### 1. Read Source Code

The agent can read any file in the repository:
- Source files (`*.py`, etc.)
- Configuration files
- Documentation
- Non-test code in `tests/` directories (conftest.py, etc.)

### 2. Modify Source Code

The agent can edit source files to implement a fix:
- Add/modify functions
- Fix bugs
- Add error handling
- Refactor code

### 3. Run Tests

The agent can run tests to:
- See the initial failure
- Verify the fix works
- Check for regressions

### 4. Create New Files

The agent can create new source files if needed for the fix.

## What the Agent CANNOT Do

### 1. Modify Test Files

```
✗ Cannot edit tests/test_example.py
✗ Cannot delete test files
✗ Cannot rename test files
```

Even if the agent tries, the files are read-only and won't be synced.

### 2. Access Git History

```
✗ Cannot run `git log`
✗ Cannot run `git show`
✗ Cannot run `git diff` with commit references
✗ Cannot access .git directory contents
```

The custom test tool doesn't expose git commands.

### 3. Run Arbitrary Shell Commands

```
✗ Cannot run docker exec directly
✗ Cannot run arbitrary bash commands in container
✗ Cannot install packages
✗ Cannot access network from container
```

### 4. Access the Gold Patch

```
✗ Cannot see the actual fix
✗ Cannot access patch field from the instance
✗ Cannot see commits after base_commit
```

## Prompt Template

The agent receives this structured prompt:

```markdown
You are fixing a software engineering issue from the SWE-bench benchmark.

## Repository
- Repository: {repo}
- Version: {version}

## Problem Statement

{problem_statement}

## Hints (if available)

{hints_text}

## Tests to Fix

The following tests currently fail and must pass after your fix:

- `{test_1}`
- `{test_2}`

## Success Criteria

You are done when:
1. All tests listed above pass
2. No other tests regress (existing functionality preserved)
3. Only necessary files were modified

## Approach

1. **Run the failing test** - Use `run_swebench_test` to see the actual error
2. **Locate the bug** - The traceback shows the exact file, function, and line
3. **Make a minimal fix** - Change only what's necessary
4. **Verify** - Run the test again to confirm it passes
5. **Check for regressions** - Ensure you haven't broken other tests

## Constraints

- Do not modify test files (they are read-only)
- Do not refactor or "improve" unrelated code
- Preserve all existing functionality

## Running Tests

Use relative paths from the repository root for all file operations.

Test with `run_swebench_test`:
```json
{"test_path": "tests/test_example.py::test_function"}
```
```

## Comparison with Human Developers

| Aspect | Human Developer | AI Agent |
|--------|----------------|----------|
| See problem description | ✓ Yes | ✓ Yes |
| Read source code | ✓ Yes | ✓ Yes |
| Run tests | ✓ Yes | ✓ Yes |
| Modify source | ✓ Yes | ✓ Yes |
| See the fix | ✗ No (solving blind) | ✗ No |
| Modify tests | ✗ Should not | ✗ Cannot |
| Access git history | ✓ Yes (but wouldn't use for cheating) | ✗ No |

The agent operates under slightly stricter constraints than a human (no git history), but the core task is identical: understand the problem, find the bug, fix it, verify with tests.

## Enforcement Mechanisms

### Layer 1: Information Hiding

Gold patch and git history simply aren't provided.

### Layer 2: Tool Restriction

Only `run_swebench_test` is available, not general shell access.

### Layer 3: Input Validation

Test paths are validated to prevent injection attacks.

### Layer 4: File Protection

Test files are made read-only at the filesystem level.

### Layer 5: Sync Exclusion

Test files are explicitly excluded from workspace→testbed sync.

### Layer 6: Final Verification

Official SWE-bench harness runs in a fresh environment, ignoring any workspace tampering.

## Why These Constraints Matter

1. **Prevents gaming** - Can't just copy the solution or modify tests
2. **Fair comparison** - Same constraints for all agents on the leaderboard
3. **Meaningful metric** - Actually measures problem-solving ability
4. **Reproducible** - Independent verification is possible
5. **Real-world relevant** - Mirrors actual bug-fixing workflows
