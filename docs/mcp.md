# MCP (Model Context Protocol) Support

Qbit supports the [Model Context Protocol (MCP)](https://modelcontextprotocol.io/), an open standard created by Anthropic for connecting AI applications to external tools and data sources. This allows you to extend Qbit's capabilities by connecting to any MCP-compatible server.

## What is MCP?

MCP provides a standardized way for AI assistants to:
- Access tools and capabilities from external servers
- Connect to databases, APIs, and other services
- Use community-built integrations

The protocol is supported by Claude Desktop, VS Code Copilot, and other major AI tools.

## Configuration

MCP servers are configured in JSON files, not in `settings.toml`.

### User-Global Configuration

Create `~/.qbit/mcp.json` for servers available across all projects:

```json
{
  "mcpServers": {
    "filesystem": {
      "transport": "stdio",
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "/home/user/documents"]
    },
    "github": {
      "transport": "http",
      "url": "https://api.github.com/mcp",
      "headers": {
        "Authorization": "Bearer ${GITHUB_TOKEN}"
      }
    }
  }
}
```

### Project-Specific Configuration

Create `<project>/.qbit/mcp.json` for project-specific servers. These override user-global servers with the same name:

```json
{
  "mcpServers": {
    "project-db": {
      "transport": "stdio",
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-postgres"],
      "env": {
        "DATABASE_URL": "${DATABASE_URL}"
      }
    }
  }
}
```

## Server Configuration Options

| Field | Type | Description |
|-------|------|-------------|
| `transport` | `"stdio"` \| `"http"` | Transport type (default: `stdio`) |
| `command` | string | Command to run for stdio transport |
| `args` | string[] | Arguments for the command |
| `env` | object | Environment variables for the server process |
| `url` | string | URL for HTTP transport |
| `headers` | object | HTTP headers (for authentication, etc.) |
| `enabled` | boolean | Whether the server is enabled (default: `true`) |
| `timeout` | number | Server startup timeout in seconds (default: `30`) |

## Environment Variables

Configuration values support environment variable interpolation using both `$VAR` and `${VAR}` syntax:

```json
{
  "mcpServers": {
    "example": {
      "transport": "http",
      "url": "https://api.example.com/mcp",
      "headers": {
        "Authorization": "Bearer ${API_TOKEN}",
        "X-Custom-Header": "$CUSTOM_VALUE"
      }
    }
  }
}
```

Variables are resolved from the environment at connection time. If a variable is not set, it's replaced with an empty string.

## Transport Types

### stdio (Recommended)

Runs the MCP server as a child process. Best for local tools:

```json
{
  "mcpServers": {
    "sqlite": {
      "transport": "stdio",
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-sqlite", "database.db"]
    }
  }
}
```

### HTTP

Connects to a remote MCP server over HTTP:

```json
{
  "mcpServers": {
    "remote-api": {
      "transport": "http",
      "url": "https://mcp.example.com/v1",
      "headers": {
        "Authorization": "Bearer ${API_KEY}"
      }
    }
  }
}
```

## Tool Naming

MCP tools are exposed to the AI agent with namespaced names to avoid conflicts:

```
mcp__{server}__{tool}
```

For example, a tool named `query` from a server named `sqlite` becomes `mcp__sqlite__query`.

## Using MCP in the UI

1. Open **Settings** (⌘,)
2. Navigate to **MCP Servers**
3. View configured servers and their connection status
4. Connect/disconnect servers manually
5. Browse available tools from connected servers

## Using MCP in CLI Mode

MCP servers auto-connect when running in headless CLI mode:

```bash
# MCP servers will connect automatically based on workspace config
qbit -e "Query the database for recent users"

# Verbose mode shows MCP connection info
qbit -v -e "List files in documents"
```

## Security

### Project Config Trust

When opening a workspace with a `.qbit/mcp.json` file for the first time, Qbit tracks whether you've approved it. Trusted project configs are stored in `~/.qbit/trusted-mcp-configs.json`.

This prevents malicious repositories from running arbitrary commands through MCP servers without your knowledge.

### Best Practices

1. **Review project configs** before trusting them
2. **Use environment variables** for secrets, never hardcode them
3. **Prefer stdio transport** for local tools (isolated child processes)
4. **Limit file system access** when configuring filesystem servers

## Finding MCP Servers

Browse the official MCP server registry for community-built integrations:

- [MCP Server Registry](https://github.com/modelcontextprotocol/servers)

Popular servers include:
- `@modelcontextprotocol/server-filesystem` - File system access
- `@modelcontextprotocol/server-sqlite` - SQLite database queries
- `@modelcontextprotocol/server-postgres` - PostgreSQL database
- `@modelcontextprotocol/server-github` - GitHub API integration

## Troubleshooting

### Server won't connect

1. Check the command exists and is executable
2. Verify environment variables are set
3. Check logs in `~/.qbit/backend.log` for error details
4. Try running the command manually to see error output

### Tools not appearing

1. Ensure the server is connected (check Settings > MCP Servers)
2. Verify the server is enabled in config (`"enabled": true`)
3. Check that tools are listed when you click on the server in Settings

### Environment variables not resolved

1. Variables must be set in your shell environment before starting Qbit
2. Use `${VAR}` syntax for variables with special characters
3. Check for typos in variable names

## Example Configurations

### Local Development Stack

```json
{
  "mcpServers": {
    "filesystem": {
      "transport": "stdio",
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "."]
    },
    "postgres": {
      "transport": "stdio",
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-postgres"],
      "env": {
        "DATABASE_URL": "${DATABASE_URL}"
      }
    }
  }
}
```

### Remote API Integration

```json
{
  "mcpServers": {
    "company-api": {
      "transport": "http",
      "url": "https://mcp.company.com/v1",
      "headers": {
        "Authorization": "Bearer ${COMPANY_API_TOKEN}",
        "X-Team-ID": "${TEAM_ID}"
      }
    }
  }
}
```

## Related Documentation

- [MCP Specification](https://modelcontextprotocol.io/specification/2025-11-25)
- [Official Rust SDK (rmcp)](https://github.com/modelcontextprotocol/rust-sdk)
- [Tool Use in Qbit](tool-use.md)
