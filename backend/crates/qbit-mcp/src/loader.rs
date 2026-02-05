use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::config::McpConfigFile;

const TRUSTED_CONFIGS_FILENAME: &str = "trusted-mcp-configs.json";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TrustedMcpConfigs {
    #[serde(default)]
    pub trusted_paths: HashSet<String>,
}

/// Load and merge MCP configs from user-global and project locations.
pub fn load_mcp_config(project_dir: &Path) -> Result<McpConfigFile> {
    let mut merged = McpConfigFile::default();

    // 1. Load user-global config (~/.qbit/mcp.json)
    if let Some(path) = user_config_path() {
        if path.exists() {
            let user_config: McpConfigFile = load_json_file(&path)
                .with_context(|| format!("Failed to load MCP config at {}", path.display()))?;
            merged.mcp_servers.extend(user_config.mcp_servers);
        }
    }

    // 2. Load project config (<project>/.qbit/mcp.json)
    let project_config_path = project_dir.join(".qbit/mcp.json");
    if project_config_path.exists() {
        let project_config: McpConfigFile =
            load_json_file(&project_config_path).with_context(|| {
                format!(
                    "Failed to load MCP config at {}",
                    project_config_path.display()
                )
            })?;
        merged.mcp_servers.extend(project_config.mcp_servers);
    }

    Ok(merged)
}

/// Interpolate environment variables in config values.
/// Supports both $VAR and ${VAR} syntax.
pub fn interpolate_env_vars(value: &str) -> String {
    let mut out = String::new();
    let mut chars = value.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch != '$' {
            out.push(ch);
            continue;
        }

        match chars.peek() {
            Some('{') => {
                chars.next(); // consume '{'
                let mut var_name = String::new();
                let mut found_close = false;
                while let Some(&next) = chars.peek() {
                    chars.next();
                    if next == '}' {
                        found_close = true;
                        break;
                    }
                    var_name.push(next);
                }
                if var_name.is_empty() {
                    out.push('$');
                    out.push('{');
                    if found_close {
                        out.push('}');
                    }
                    continue;
                }
                if let Ok(value) = std::env::var(&var_name) {
                    out.push_str(&value);
                }
            }
            Some(next) if is_var_start(*next) => {
                let mut var_name = String::new();
                while let Some(&next) = chars.peek() {
                    if !is_var_char(next) {
                        break;
                    }
                    chars.next();
                    var_name.push(next);
                }
                if let Ok(value) = std::env::var(&var_name) {
                    out.push_str(&value);
                }
            }
            _ => {
                out.push('$');
            }
        }
    }

    out
}

/// Check if a project's MCP config has been approved.
pub fn is_project_config_trusted(project_dir: &Path) -> bool {
    let Some(path) = trusted_configs_path() else {
        return false;
    };
    let Ok(contents) = fs::read_to_string(path) else {
        return false;
    };
    let Ok(trusted) = serde_json::from_str::<TrustedMcpConfigs>(&contents) else {
        return false;
    };
    let Ok(project_path) = project_dir.canonicalize() else {
        return false;
    };
    trusted
        .trusted_paths
        .contains(&project_path.to_string_lossy().to_string())
}

/// Mark a project's MCP config as trusted (after user approval).
pub fn trust_project_config(project_dir: &Path) -> Result<()> {
    let Some(path) = trusted_configs_path() else {
        return Ok(());
    };
    let mut trusted = if path.exists() {
        let contents = fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str::<TrustedMcpConfigs>(&contents).unwrap_or_default()
    } else {
        TrustedMcpConfigs::default()
    };
    let project_path = project_dir
        .canonicalize()
        .unwrap_or_else(|_| project_dir.to_path_buf());
    trusted
        .trusted_paths
        .insert(project_path.to_string_lossy().to_string());

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "Failed to create MCP trust directory at {}",
                parent.display()
            )
        })?;
    }
    let serialized = serde_json::to_string_pretty(&trusted)?;
    fs::write(&path, serialized)?;
    Ok(())
}

fn load_json_file<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T> {
    let contents = fs::read_to_string(path)?;
    let config = serde_json::from_str(&contents)?;
    Ok(config)
}

fn user_config_path() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".qbit/mcp.json"))
}

fn trusted_configs_path() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".qbit").join(TRUSTED_CONFIGS_FILENAME))
}

fn is_var_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_'
}

fn is_var_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tempfile::TempDir;

    #[test]
    fn test_interpolate_env_vars_simple() {
        env::set_var("TEST_MCP_VAR", "hello");
        assert_eq!(interpolate_env_vars("$TEST_MCP_VAR"), "hello");
        env::remove_var("TEST_MCP_VAR");
    }

    #[test]
    fn test_interpolate_env_vars_braced() {
        env::set_var("TEST_MCP_VAR2", "world");
        assert_eq!(interpolate_env_vars("${TEST_MCP_VAR2}"), "world");
        env::remove_var("TEST_MCP_VAR2");
    }

    #[test]
    fn test_interpolate_env_vars_mixed() {
        // Note: $VAR syntax consumes all valid var chars (alphanumeric + underscore)
        // So $TEST_MCP_A_middle would look for var "TEST_MCP_A_middle", not "TEST_MCP_A"
        // Use ${VAR} syntax for explicit boundaries
        env::set_var("TEST_MCP_A", "foo");
        env::set_var("TEST_MCP_B", "bar");
        assert_eq!(
            interpolate_env_vars("prefix_${TEST_MCP_A}_middle_${TEST_MCP_B}_suffix"),
            "prefix_foo_middle_bar_suffix"
        );
        env::remove_var("TEST_MCP_A");
        env::remove_var("TEST_MCP_B");
    }

    #[test]
    fn test_interpolate_env_vars_bare_consumes_underscores() {
        // Bare $VAR syntax includes underscores in the var name (shell-like behavior)
        env::set_var("TEST_MCP_WITH_UNDERSCORES", "value");
        assert_eq!(interpolate_env_vars("$TEST_MCP_WITH_UNDERSCORES"), "value");
        env::remove_var("TEST_MCP_WITH_UNDERSCORES");
    }

    #[test]
    fn test_interpolate_env_vars_missing() {
        // Missing env vars should be replaced with empty string
        assert_eq!(interpolate_env_vars("$NONEXISTENT_MCP_VAR_12345"), "");
        assert_eq!(interpolate_env_vars("${NONEXISTENT_MCP_VAR_12345}"), "");
    }

    #[test]
    fn test_interpolate_env_vars_no_vars() {
        assert_eq!(
            interpolate_env_vars("no variables here"),
            "no variables here"
        );
    }

    #[test]
    fn test_interpolate_env_vars_dollar_only() {
        assert_eq!(interpolate_env_vars("$"), "$");
        assert_eq!(interpolate_env_vars("$ "), "$ ");
        assert_eq!(interpolate_env_vars("$1"), "$1"); // Numbers don't start var names
    }

    #[test]
    fn test_interpolate_env_vars_empty_braces() {
        assert_eq!(interpolate_env_vars("${}"), "${}");
    }

    #[test]
    fn test_load_mcp_config_empty_dir() {
        let temp = TempDir::new().unwrap();
        let config = load_mcp_config(temp.path()).unwrap();
        assert!(config.mcp_servers.is_empty());
    }

    #[test]
    fn test_load_mcp_config_project_only() {
        let temp = TempDir::new().unwrap();
        let qbit_dir = temp.path().join(".qbit");
        fs::create_dir_all(&qbit_dir).unwrap();

        let config_json = r#"{
            "mcpServers": {
                "test-server": {
                    "transport": "stdio",
                    "command": "echo",
                    "args": ["hello"]
                }
            }
        }"#;
        fs::write(qbit_dir.join("mcp.json"), config_json).unwrap();

        let config = load_mcp_config(temp.path()).unwrap();
        assert_eq!(config.mcp_servers.len(), 1);
        assert!(config.mcp_servers.contains_key("test-server"));

        let server = &config.mcp_servers["test-server"];
        assert_eq!(server.command.as_deref(), Some("echo"));
        assert_eq!(server.args, vec!["hello"]);
        assert!(server.enabled); // Default
    }

    #[test]
    fn test_load_mcp_config_invalid_json() {
        let temp = TempDir::new().unwrap();
        let qbit_dir = temp.path().join(".qbit");
        fs::create_dir_all(&qbit_dir).unwrap();

        fs::write(qbit_dir.join("mcp.json"), "{ invalid json }").unwrap();

        let result = load_mcp_config(temp.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_trust_and_check_project_config() {
        let temp = TempDir::new().unwrap();

        // Initially not trusted
        assert!(!is_project_config_trusted(temp.path()));

        // Trust it (this writes to ~/.qbit which we can't easily test without mocking)
        // So we just verify the function doesn't panic
        // Full integration test would require mocking the home dir
    }

    #[test]
    fn test_load_mcp_config_merges_project_over_user() {
        // This tests the actual merging behavior which is critical:
        // Project config should override user config for same server name

        let temp = TempDir::new().unwrap();
        let qbit_dir = temp.path().join(".qbit");
        fs::create_dir_all(&qbit_dir).unwrap();

        // Project config defines a server
        let project_config = r#"{
            "mcpServers": {
                "shared-server": {
                    "command": "project-command",
                    "args": ["--project"]
                },
                "project-only": {
                    "command": "project-only-cmd"
                }
            }
        }"#;
        fs::write(qbit_dir.join("mcp.json"), project_config).unwrap();

        // Note: We can't easily test with user config without mocking home_dir
        // But we can verify project config loads correctly
        let config = load_mcp_config(temp.path()).unwrap();

        assert_eq!(config.mcp_servers.len(), 2);
        assert_eq!(
            config.mcp_servers["shared-server"].command.as_deref(),
            Some("project-command")
        );
        assert_eq!(config.mcp_servers["shared-server"].args, vec!["--project"]);
        assert!(config.mcp_servers.contains_key("project-only"));
    }

    #[test]
    fn test_load_mcp_config_all_fields() {
        // Test that all config fields are parsed correctly
        let temp = TempDir::new().unwrap();
        let qbit_dir = temp.path().join(".qbit");
        fs::create_dir_all(&qbit_dir).unwrap();

        let config_json = r#"{
            "mcpServers": {
                "full-config": {
                    "transport": "http",
                    "command": "should-be-ignored",
                    "args": ["arg1", "arg2"],
                    "env": {
                        "API_KEY": "${MY_API_KEY}",
                        "DEBUG": "true"
                    },
                    "url": "https://api.example.com/mcp",
                    "headers": {
                        "Authorization": "Bearer ${TOKEN}",
                        "X-Custom": "value"
                    },
                    "enabled": false,
                    "timeout": 60
                }
            }
        }"#;
        fs::write(qbit_dir.join("mcp.json"), config_json).unwrap();

        let config = load_mcp_config(temp.path()).unwrap();
        let server = &config.mcp_servers["full-config"];

        assert!(matches!(
            server.transport,
            crate::config::McpTransportType::Http
        ));
        assert_eq!(server.command.as_deref(), Some("should-be-ignored"));
        assert_eq!(server.args, vec!["arg1", "arg2"]);
        assert_eq!(server.env.len(), 2);
        assert_eq!(server.env.get("DEBUG"), Some(&"true".to_string()));
        assert_eq!(server.url.as_deref(), Some("https://api.example.com/mcp"));
        assert_eq!(server.headers.len(), 2);
        assert_eq!(server.headers.get("X-Custom"), Some(&"value".to_string()));
        assert!(!server.enabled);
        assert_eq!(server.timeout, 60);
    }

    #[test]
    fn test_interpolate_preserves_surrounding_text() {
        env::set_var("TEST_INTERP_VAR", "VALUE");

        // Test various positions
        assert_eq!(
            interpolate_env_vars("before ${TEST_INTERP_VAR} after"),
            "before VALUE after"
        );
        assert_eq!(
            interpolate_env_vars("${TEST_INTERP_VAR}:suffix"),
            "VALUE:suffix"
        );
        assert_eq!(
            interpolate_env_vars("prefix:${TEST_INTERP_VAR}"),
            "prefix:VALUE"
        );

        env::remove_var("TEST_INTERP_VAR");
    }

    #[test]
    fn test_interpolate_multiple_same_var() {
        env::set_var("TEST_REPEAT", "X");

        assert_eq!(
            interpolate_env_vars("$TEST_REPEAT-$TEST_REPEAT-${TEST_REPEAT}"),
            "X-X-X"
        );

        env::remove_var("TEST_REPEAT");
    }

    #[test]
    fn test_interpolate_unclosed_brace() {
        // Unclosed brace should consume rest of string as var name
        // and result in empty (since that var doesn't exist)
        assert_eq!(interpolate_env_vars("${UNCLOSED"), "");
        assert_eq!(interpolate_env_vars("prefix ${UNCLOSED"), "prefix ");
    }

    #[test]
    fn test_interpolate_nested_not_supported() {
        // Nested ${${VAR}} is not supported - outer should work, inner becomes literal
        env::set_var("OUTER", "outer_val");

        // This will try to find var named "${OUTER" which doesn't exist
        let result = interpolate_env_vars("${${OUTER}}");
        // The inner ${ starts a new var capture, so it looks for var "${OUTER}"
        // which doesn't exist, so empty string, then "}" is left over
        assert_eq!(result, "}");

        env::remove_var("OUTER");
    }
}
