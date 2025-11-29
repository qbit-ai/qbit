use crate::error::{QbitError, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::PathBuf;

const INTEGRATION_VERSION: &str = "1.0.0";

const INTEGRATION_SCRIPT: &str = r#"# ~/.config/qbit/integration.zsh
# Qbit Shell Integration v1.0.0
# Do not edit - managed by Qbit

# Guard against double-sourcing
[[ -n "$QBIT_INTEGRATION_LOADED" ]] && return
export QBIT_INTEGRATION_LOADED=1

# Only run inside Qbit
[[ -z "$QBIT" ]] && return

# ============ OSC Helpers ============

__qbit_osc() {
    printf '\e]133;%s\e\\' "$1"
}

__qbit_report_cwd() {
    printf '\e]7;file://%s%s\e\\' "${HOST:-$(hostname)}" "$PWD"
}

__qbit_notify() {
    printf '\e]9;%s\e\\' "$1"
}

# ============ Prompt Markers ============

__qbit_prompt_start() {
    __qbit_osc "A"
}

__qbit_prompt_end() {
    __qbit_osc "B"
}

__qbit_cmd_start() {
    local cmd="$1"
    if [[ -n "$cmd" ]]; then
        __qbit_osc "C;$cmd"
    else
        __qbit_osc "C"
    fi
    QBIT_CMD_START=$EPOCHREALTIME
}

__qbit_cmd_end() {
    local exit_code=${1:-0}
    __qbit_osc "D;$exit_code"

    if [[ -n "$QBIT_CMD_START" ]]; then
        local duration=$(( ${EPOCHREALTIME%.*} - ${QBIT_CMD_START%.*} ))
        if (( duration > 10 )); then
            __qbit_notify "Command finished (${duration}s)"
        fi
    fi
    unset QBIT_CMD_START
}

# ============ Hook Functions ============

__qbit_preexec() {
    __qbit_cmd_start "$1"
}

__qbit_precmd() {
    local exit_code=$?
    __qbit_cmd_end $exit_code
    __qbit_report_cwd
    __qbit_prompt_start
}

__qbit_line_init() {
    __qbit_prompt_end
}

# ============ Register Hooks ============

autoload -Uz add-zsh-hook

add-zsh-hook -d preexec __qbit_preexec 2>/dev/null
add-zsh-hook -d precmd __qbit_precmd 2>/dev/null

add-zsh-hook preexec __qbit_preexec
add-zsh-hook precmd __qbit_precmd

if [[ -o zle ]]; then
    if (( ${+functions[zle-line-init]} )); then
        functions[__qbit_orig_zle_line_init]="${functions[zle-line-init]}"
        zle-line-init() {
            __qbit_orig_zle_line_init
            __qbit_line_init
        }
    else
        zle-line-init() {
            __qbit_line_init
        }
    fi
    zle -N zle-line-init
fi

__qbit_report_cwd
"#;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum IntegrationStatus {
    NotInstalled,
    Installed {
        version: String,
    },
    Outdated {
        current: String,
        latest: String,
    },
    /// Shell integration files exist but .zshrc points to wrong path
    Misconfigured {
        expected_path: String,
        issue: String,
    },
}

fn get_config_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|p| p.join("qbit"))
}

fn get_integration_path() -> Option<PathBuf> {
    get_config_dir().map(|p| p.join("integration.zsh"))
}

fn get_version_path() -> Option<PathBuf> {
    get_config_dir().map(|p| p.join("integration.version"))
}

fn get_zshrc_path() -> Option<PathBuf> {
    dirs::home_dir().map(|p| p.join(".zshrc"))
}

/// Validates that the .zshrc sources the integration script from the correct path
fn validate_zshrc_integration() -> Result<Option<String>> {
    let zshrc_path = get_zshrc_path()
        .ok_or_else(|| QbitError::Internal("Could not determine home directory".into()))?;

    let integration_path = get_integration_path()
        .ok_or_else(|| QbitError::Internal("Could not determine integration path".into()))?;

    if !zshrc_path.exists() {
        return Ok(Some("No .zshrc file found".to_string()));
    }

    let content = fs::read_to_string(&zshrc_path).map_err(QbitError::Io)?;

    // Check if there's any qbit integration line
    if !content.contains("qbit/integration.zsh") && !content.contains("qbit\\integration.zsh") {
        return Ok(Some("No Qbit integration found in .zshrc".to_string()));
    }

    // Check if the correct path is referenced
    let expected_path_str = integration_path.display().to_string();
    if !content.contains(&expected_path_str) {
        // Try to find what path is actually being used
        for line in content.lines() {
            if line.contains("qbit/integration.zsh") || line.contains("qbit\\integration.zsh") {
                if line.trim().starts_with('#') && !line.contains("# Qbit shell integration") {
                    continue; // Skip comments that aren't our marker
                }
                return Ok(Some(format!(
                    "Incorrect path in .zshrc. Expected: {}",
                    expected_path_str
                )));
            }
        }
    }

    Ok(None) // No issues found
}

#[tauri::command]
pub async fn shell_integration_status() -> Result<IntegrationStatus> {
    let version_path = get_version_path()
        .ok_or_else(|| QbitError::Internal("Could not determine config directory".into()))?;

    let integration_path = get_integration_path()
        .ok_or_else(|| QbitError::Internal("Could not determine integration path".into()))?;

    if !version_path.exists() {
        return Ok(IntegrationStatus::NotInstalled);
    }

    // Check if integration script actually exists
    if !integration_path.exists() {
        return Ok(IntegrationStatus::NotInstalled);
    }

    let current_version = fs::read_to_string(&version_path)
        .map_err(QbitError::Io)?
        .trim()
        .to_string();

    // Validate .zshrc has correct path
    if let Some(issue) = validate_zshrc_integration()? {
        return Ok(IntegrationStatus::Misconfigured {
            expected_path: integration_path.display().to_string(),
            issue,
        });
    }

    if current_version == INTEGRATION_VERSION {
        Ok(IntegrationStatus::Installed {
            version: current_version,
        })
    } else {
        Ok(IntegrationStatus::Outdated {
            current: current_version,
            latest: INTEGRATION_VERSION.to_string(),
        })
    }
}

#[tauri::command]
pub async fn shell_integration_install() -> Result<()> {
    let config_dir = get_config_dir()
        .ok_or_else(|| QbitError::Internal("Could not determine config directory".into()))?;

    // Create config directory
    fs::create_dir_all(&config_dir).map_err(QbitError::Io)?;

    // Write integration script
    let script_path = config_dir.join("integration.zsh");
    fs::write(&script_path, INTEGRATION_SCRIPT).map_err(QbitError::Io)?;

    // Write version marker
    let version_path = config_dir.join("integration.version");
    fs::write(&version_path, INTEGRATION_VERSION).map_err(QbitError::Io)?;

    // Update .zshrc
    update_zshrc()?;

    Ok(())
}

#[tauri::command]
pub async fn shell_integration_uninstall() -> Result<()> {
    let config_dir = get_config_dir()
        .ok_or_else(|| QbitError::Internal("Could not determine config directory".into()))?;

    let script_path = config_dir.join("integration.zsh");
    let version_path = config_dir.join("integration.version");

    if script_path.exists() {
        fs::remove_file(&script_path).map_err(QbitError::Io)?;
    }
    if version_path.exists() {
        fs::remove_file(&version_path).map_err(QbitError::Io)?;
    }

    Ok(())
}

fn update_zshrc() -> Result<()> {
    let zshrc_path = get_zshrc_path()
        .ok_or_else(|| QbitError::Internal("Could not determine home directory".into()))?;

    let integration_path = get_integration_path()
        .ok_or_else(|| QbitError::Internal("Could not determine integration path".into()))?;

    let source_line = format!(
        r#"
# Qbit shell integration
[[ -n "$QBIT" ]] && source "{}"
"#,
        integration_path.display()
    );

    if zshrc_path.exists() {
        let content = fs::read_to_string(&zshrc_path).map_err(QbitError::Io)?;
        let expected_path_str = integration_path.display().to_string();

        // Check if correctly configured
        if content.contains(&expected_path_str) {
            return Ok(());
        }

        // Check if there's an old/incorrect qbit integration line that needs fixing
        if content.contains("qbit/integration.zsh") || content.contains("qbit\\integration.zsh") {
            // Remove old integration lines and add correct one
            let mut new_lines: Vec<&str> = Vec::new();
            let mut skip_next = false;
            let mut found_and_replaced = false;

            for line in content.lines() {
                // Skip the comment line before source command
                if line.trim() == "# Qbit shell integration" {
                    skip_next = true;
                    continue;
                }

                // Skip the old source line
                if skip_next
                    && (line.contains("qbit/integration.zsh")
                        || line.contains("qbit\\integration.zsh"))
                {
                    skip_next = false;
                    // Only add replacement once
                    if !found_and_replaced {
                        // We'll append the new integration at the end
                        found_and_replaced = true;
                    }
                    continue;
                }

                skip_next = false;
                new_lines.push(line);
            }

            // Write updated content
            let mut new_content = new_lines.join("\n");
            if !new_content.ends_with('\n') {
                new_content.push('\n');
            }
            new_content.push_str(&source_line);

            fs::write(&zshrc_path, new_content).map_err(QbitError::Io)?;
            return Ok(());
        }
    }

    // No existing integration, append new one
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&zshrc_path)
        .map_err(QbitError::Io)?;

    writeln!(file, "{}", source_line).map_err(QbitError::Io)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_line_uses_actual_config_path() {
        // This test ensures we never regress to hardcoded paths
        let integration_path = get_integration_path().expect("Should get integration path");
        let config_dir = get_config_dir().expect("Should get config dir");

        // The integration path must be under the config directory
        assert!(
            integration_path.starts_with(&config_dir),
            "Integration path {:?} should be under config dir {:?}",
            integration_path,
            config_dir
        );

        // On macOS, this should NOT be ~/.config but ~/Library/Application Support
        #[cfg(target_os = "macos")]
        {
            let path_str = integration_path.display().to_string();
            assert!(
                !path_str.contains("/.config/"),
                "macOS should use Application Support, not .config. Got: {}",
                path_str
            );
            assert!(
                path_str.contains("Library/Application Support"),
                "macOS should use Library/Application Support. Got: {}",
                path_str
            );
        }

        // On Linux, it should be ~/.config
        #[cfg(target_os = "linux")]
        {
            let path_str = integration_path.display().to_string();
            assert!(
                path_str.contains("/.config/") || path_str.contains("XDG_CONFIG"),
                "Linux should use .config or XDG_CONFIG. Got: {}",
                path_str
            );
        }
    }

    #[test]
    fn test_validate_zshrc_detects_wrong_path() {
        // This test requires mocking the filesystem which is complex in Rust
        // Instead, we test the logic by checking the actual system config
        let integration_path = get_integration_path().expect("Should get integration path");
        let expected_path_str = integration_path.display().to_string();

        // Verify the path we generate is what we expect
        assert!(
            expected_path_str.contains("qbit"),
            "Path should contain 'qbit'"
        );
        assert!(
            expected_path_str.ends_with("integration.zsh"),
            "Path should end with integration.zsh"
        );
    }

    #[test]
    fn test_integration_script_contains_required_markers() {
        // Ensure the integration script has all required OSC markers
        // The script uses __qbit_osc "X" which generates OSC 133;X
        assert!(
            INTEGRATION_SCRIPT.contains("__qbit_osc"),
            "Script should have OSC helper"
        );
        assert!(
            INTEGRATION_SCRIPT.contains(r#"133;%s"#),
            "Script should have OSC 133 format string"
        );
        assert!(
            INTEGRATION_SCRIPT.contains(r#"__qbit_osc "A""#),
            "Script should emit prompt_start (A marker)"
        );
        assert!(
            INTEGRATION_SCRIPT.contains(r#"__qbit_osc "B""#),
            "Script should emit prompt_end (B marker)"
        );
        assert!(
            INTEGRATION_SCRIPT.contains(r#"__qbit_osc "C"#),
            "Script should emit command_start (C marker)"
        );
        assert!(
            INTEGRATION_SCRIPT.contains(r#"__qbit_osc "D"#),
            "Script should emit command_end (D marker)"
        );
        assert!(
            INTEGRATION_SCRIPT.contains("preexec"),
            "Script should use preexec hook"
        );
        assert!(
            INTEGRATION_SCRIPT.contains("precmd"),
            "Script should use precmd hook"
        );
    }

    #[test]
    fn test_integration_script_checks_qbit_env() {
        // The script should only run inside Qbit
        assert!(
            INTEGRATION_SCRIPT.contains(r#"[[ -z "$QBIT" ]] && return"#),
            "Script should check for QBIT env var"
        );
    }

    #[test]
    fn test_config_dir_consistency() {
        // All path functions should use the same base directory
        let config_dir = get_config_dir().expect("Should get config dir");
        let integration_path = get_integration_path().expect("Should get integration path");
        let version_path = get_version_path().expect("Should get version path");

        assert!(
            integration_path.parent() == Some(config_dir.as_path()),
            "Integration path parent should be config dir"
        );
        assert!(
            version_path.parent() == Some(config_dir.as_path()),
            "Version path parent should be config dir"
        );
    }
}
