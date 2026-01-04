# AST-Grep Tools

Qbit includes two AST-based code search and replace tools powered by [ast-grep](https://ast-grep.github.io/). Unlike regex-based search, these tools understand code structure and can match syntactic patterns like function definitions, method calls, and control flow statements.

## Tools

### `ast_grep` - Structural Code Search

Search for code patterns using AST (Abstract Syntax Tree) matching.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `pattern` | string | Yes | AST pattern to search for |
| `path` | string | No | File or directory to search (relative to workspace). Defaults to current directory |
| `language` | string | No | Language for pattern parsing. Auto-detected from file extension if not specified |

**Supported Languages:** `rust`, `typescript`, `javascript`, `python`, `go`, `java`, `c`, `cpp`

**Example:**

```json
{
  "pattern": "console.log($MSG)",
  "path": "src",
  "language": "javascript"
}
```

**Response:**

```json
{
  "matches": [
    {
      "file": "src/app.js",
      "line": 5,
      "column": 3,
      "text": "console.log('Starting...')",
      "end_line": 5,
      "end_column": 28
    }
  ],
  "count": 1,
  "files_searched": 10
}
```

### `ast_grep_replace` - Structural Code Refactoring

Replace code patterns using AST-aware rewriting. Captured meta-variables from the pattern can be used in the replacement.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `pattern` | string | Yes | AST pattern to match |
| `replacement` | string | Yes | Replacement template using captured variables |
| `path` | string | Yes | File or directory to modify (relative to workspace) |
| `language` | string | No | Language for pattern parsing. Auto-detected if not specified |

**Example:**

```json
{
  "pattern": "console.log($MSG)",
  "replacement": "logger.info($MSG)",
  "path": "src",
  "language": "javascript"
}
```

**Response:**

```json
{
  "files_modified": ["src/app.js", "src/utils.js"],
  "replacements_count": 5,
  "changes": [
    {
      "file": "src/app.js",
      "line": 5,
      "original": "console.log('Starting...')",
      "replacement": "logger.info('Starting...')"
    }
  ]
}
```

> **Note:** `ast_grep_replace` is a write tool and requires HITL (Human-in-the-Loop) approval before execution.

## Pattern Syntax

AST-grep uses meta-variables to match code patterns:

| Syntax | Description | Example |
|--------|-------------|---------|
| `$VAR` | Match a single AST node | `console.log($MSG)` matches any single argument |
| `$$$VAR` | Match zero or more AST nodes | `fn $NAME($$$ARGS)` matches functions with any number of arguments |

### Pattern Examples

#### JavaScript/TypeScript

```
// Match console.log calls
console.log($MSG)

// Match arrow functions
($$$ARGS) => $BODY

// Match function declarations
function $NAME($$$ARGS) { $$$BODY }

// Match if statements
if ($COND) { $$$BODY }
```

#### Rust

```
// Match function definitions with return type
fn $NAME($$$ARGS) -> $RET { $$$BODY }

// Match println! macro calls
println!($MSG)

// Match Result unwrap calls
$EXPR.unwrap()

// Match if let patterns
if let $PAT = $EXPR { $$$BODY }
```

#### Python

```
// Match function definitions
def $NAME($$$ARGS):
    $$$BODY

// Match return statements
return $EXPR

// Match for loops
for $VAR in $ITER:
    $$$BODY
```

## Use Cases

### 1. Finding All Function Definitions

```json
{
  "pattern": "fn $NAME($$$ARGS) -> $RET { $$$BODY }",
  "path": "src",
  "language": "rust"
}
```

### 2. Migrating Console Logging to a Logger

```json
{
  "pattern": "console.log($MSG)",
  "replacement": "logger.info($MSG)",
  "path": "src",
  "language": "javascript"
}
```

### 3. Finding Unsafe Unwrap Calls in Rust

```json
{
  "pattern": "$EXPR.unwrap()",
  "path": "src",
  "language": "rust"
}
```

### 4. Replacing Deprecated API Calls

```json
{
  "pattern": "oldApi.fetch($URL)",
  "replacement": "newApi.request($URL)",
  "path": "src",
  "language": "typescript"
}
```

## Comparison with `grep_file`

| Feature | `grep_file` | `ast_grep` |
|---------|-------------|------------|
| Search method | Regex on text | AST pattern matching |
| Code structure awareness | No | Yes |
| Can match across lines | Limited | Yes |
| Language-specific | No | Yes |
| Variable capture | Regex groups | Meta-variables |
| Refactoring support | No | Yes (with `ast_grep_replace`) |

**When to use `grep_file`:**
- Simple text searches
- Searching in non-code files (logs, configs, etc.)
- When you need regex features like lookahead/lookbehind

**When to use `ast_grep`:**
- Searching for code patterns (function calls, definitions, etc.)
- When you need to understand code structure
- When preparing for refactoring with `ast_grep_replace`

## Implementation Details

The tools are implemented in the `qbit-ast-grep` crate, which uses:

- `ast-grep-core` - Core pattern matching engine
- `ast-grep-language` - Built-in language parsers via tree-sitter

Files are processed recursively when a directory is specified, respecting the language parameter or auto-detecting from file extensions.
