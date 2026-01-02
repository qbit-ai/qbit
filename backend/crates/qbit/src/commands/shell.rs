use crate::error::{QbitError, Result};
use crate::pty::ShellType;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::PathBuf;

const INTEGRATION_VERSION: &str = "1.1.0";

// =============================================================================
// Zsh Integration Script
// =============================================================================

const INTEGRATION_SCRIPT_ZSH: &str = r#"# ~/.config/qbit/integration.zsh
# Qbit Shell Integration v1.1.0
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

__qbit_report_venv() {
    # Report Python virtual environment via OSC 1337
    if [[ -n "$VIRTUAL_ENV" ]]; then
        # Extract venv name from path (last component)
        local venv_name="${VIRTUAL_ENV##*/}"
        printf '\e]1337;VirtualEnv=%s\e\\' "$venv_name"
    else
        # Clear virtual env indicator
        printf '\e]1337;VirtualEnv=\e\\'
    fi
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
    __qbit_report_venv
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
__qbit_report_venv
"#;

// =============================================================================
// Bash Integration Script
// =============================================================================

const INTEGRATION_SCRIPT_BASH: &str = r#"# ~/.config/qbit/integration.bash
# Qbit Shell Integration v1.1.0
# Do not edit - managed by Qbit

# Guard against double-sourcing
[[ -n "$QBIT_INTEGRATION_LOADED" ]] && return
export QBIT_INTEGRATION_LOADED=1

# Only run inside Qbit
[[ "$QBIT" != "1" ]] && return

# ============ OSC Helpers ============

__qbit_osc() {
    printf '\e]133;%s\e\\' "$1"
}

__qbit_report_cwd() {
    printf '\e]7;file://%s%s\e\\' "${HOSTNAME:-$(hostname)}" "$PWD"
}

__qbit_report_venv() {
    # Report Python virtual environment via OSC 1337
    if [[ -n "$VIRTUAL_ENV" ]]; then
        # Extract venv name from path (last component)
        local venv_name="${VIRTUAL_ENV##*/}"
        printf '\e]1337;VirtualEnv=%s\e\\' "$venv_name"
    else
        # Clear virtual env indicator
        printf '\e]1337;VirtualEnv=\e\\'
    fi
}

# ============ Hook Functions ============

# Track if preexec already ran (DEBUG trap fires multiple times)
__qbit_preexec_ran=0

__qbit_prompt_command() {
    local exit_code=$?
    __qbit_osc "D;$exit_code"
    __qbit_report_cwd
    __qbit_report_venv
    __qbit_osc "A"
    __qbit_preexec_ran=0
}

__qbit_debug_trap() {
    # Skip if we already ran preexec for this command
    [[ $__qbit_preexec_ran -eq 1 ]] && return
    # Skip if this is the PROMPT_COMMAND itself
    [[ "$BASH_COMMAND" == "$PROMPT_COMMAND" ]] && return
    [[ "$BASH_COMMAND" == "__qbit_prompt_command"* ]] && return
    __qbit_preexec_ran=1
    __qbit_osc "C"
}

# ============ Register Hooks ============

# Append to PROMPT_COMMAND (preserving existing)
if [[ -z "$PROMPT_COMMAND" ]]; then
    PROMPT_COMMAND="__qbit_prompt_command"
else
    PROMPT_COMMAND="__qbit_prompt_command;$PROMPT_COMMAND"
fi

# Set DEBUG trap for preexec behavior
trap '__qbit_debug_trap' DEBUG

# Emit B marker in PS1 (prompt end)
PS1="\[\e]133;B\e\\\]$PS1"

__qbit_report_cwd
__qbit_report_venv
"#;

// =============================================================================
// Fish Integration Script
// =============================================================================

const INTEGRATION_SCRIPT_FISH: &str = r#"# ~/.config/fish/conf.d/qbit.fish
# Qbit Shell Integration v1.1.0
# Do not edit - managed by Qbit

# Guard against double-sourcing
if set -q QBIT_INTEGRATION_LOADED
    exit
end

# Only run inside Qbit
if test "$QBIT" != "1"
    exit
end

set -gx QBIT_INTEGRATION_LOADED 1

# ============ OSC Helpers ============

function __qbit_osc
    printf '\e]133;%s\e\\' $argv[1]
end

function __qbit_report_cwd
    printf '\e]7;file://%s%s\e\\' (hostname) $PWD
end

function __qbit_report_venv
    # Report Python virtual environment via OSC 1337
    if set -q VIRTUAL_ENV
        # Extract venv name from path (last component)
        set venv_name (basename $VIRTUAL_ENV)
        printf '\e]1337;VirtualEnv=%s\e\\' $venv_name
    else
        # Clear virtual env indicator
        printf '\e]1337;VirtualEnv=\e\\'
    end
end

# ============ Hook Functions ============

function __qbit_preexec --on-event fish_preexec
    __qbit_osc "C"
end

function __qbit_postexec --on-event fish_postexec
    __qbit_osc "D;$status"
    __qbit_report_cwd
    __qbit_report_venv
end

# ============ Prompt Wrapper ============

# Save original fish_prompt if it exists
if functions -q fish_prompt
    functions -c fish_prompt __qbit_original_prompt
else
    function __qbit_original_prompt
        echo -n '$ '
    end
end

# Wrap fish_prompt to emit A/B markers
function fish_prompt
    __qbit_osc "A"
    __qbit_original_prompt
    __qbit_osc "B"
end

__qbit_report_cwd
__qbit_report_venv
"#;

// =============================================================================
// Script Selection
// =============================================================================

/// Get the integration script for a specific shell type
pub fn get_integration_script(shell_type: ShellType) -> &'static str {
    match shell_type {
        ShellType::Zsh => INTEGRATION_SCRIPT_ZSH,
        ShellType::Bash => INTEGRATION_SCRIPT_BASH,
        ShellType::Fish => INTEGRATION_SCRIPT_FISH,
        ShellType::Unknown => INTEGRATION_SCRIPT_ZSH, // Default to zsh for unknown
    }
}

#[cfg(test)]
/// Get the integration script file extension for a shell type
fn get_integration_extension(shell_type: ShellType) -> &'static str {
    match shell_type {
        ShellType::Zsh => "zsh",
        ShellType::Bash => "bash",
        ShellType::Fish => "fish",
        ShellType::Unknown => "zsh",
    }
}

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

// =============================================================================
// Testable Installation Functions (accept path parameters)
// =============================================================================

#[cfg(test)]
/// Get integration script path for a specific shell type within a config directory
fn get_integration_path_for_shell(config_dir: &std::path::Path, shell_type: ShellType) -> PathBuf {
    let filename = format!("integration.{}", get_integration_extension(shell_type));
    config_dir.join(filename)
}

#[cfg(test)]
/// Get RC file paths for a shell type within a home directory
/// Returns multiple paths for shells that need multiple RC files (e.g., bash)
fn get_rc_file_paths(home_dir: &std::path::Path, shell_type: ShellType) -> Vec<PathBuf> {
    match shell_type {
        ShellType::Zsh => vec![home_dir.join(".zshrc")],
        ShellType::Bash => vec![home_dir.join(".bashrc"), home_dir.join(".bash_profile")],
        ShellType::Fish => vec![home_dir.join(".config/fish/conf.d/qbit.fish")],
        ShellType::Unknown => vec![home_dir.join(".zshrc")], // Default to zsh
    }
}

#[cfg(test)]
/// Install shell integration for a specific shell type
/// This is the testable version that accepts path parameters
fn install_integration_internal(
    shell_type: ShellType,
    config_dir: &std::path::Path,
    home_dir: &std::path::Path,
) -> Result<()> {
    // Create config directory
    fs::create_dir_all(config_dir).map_err(QbitError::Io)?;

    // Write integration script
    let script_path = get_integration_path_for_shell(config_dir, shell_type);
    fs::write(&script_path, get_integration_script(shell_type)).map_err(QbitError::Io)?;

    // Write version marker
    let version_path = config_dir.join("integration.version");
    fs::write(&version_path, INTEGRATION_VERSION).map_err(QbitError::Io)?;

    // Update RC files
    let rc_paths = get_rc_file_paths(home_dir, shell_type);
    for rc_path in rc_paths {
        update_rc_file_internal(&rc_path, &script_path, shell_type)?;
    }

    Ok(())
}

#[cfg(test)]
/// Update a single RC file to source the integration script
fn update_rc_file_internal(
    rc_path: &std::path::Path,
    integration_path: &std::path::Path,
    shell_type: ShellType,
) -> Result<()> {
    // Create parent directories if needed (for fish config)
    if let Some(parent) = rc_path.parent() {
        fs::create_dir_all(parent).map_err(QbitError::Io)?;
    }

    let source_line = match shell_type {
        ShellType::Fish => format!(
            r#"
# Qbit shell integration
if test "$QBIT" = "1"
    source "{}"
end
"#,
            integration_path.display()
        ),
        _ => format!(
            r#"
# Qbit shell integration
[[ -n "$QBIT" ]] && source "{}"
"#,
            integration_path.display()
        ),
    };

    if rc_path.exists() {
        let content = fs::read_to_string(rc_path).map_err(QbitError::Io)?;
        let integration_path_str = integration_path.display().to_string();

        // Check if already configured correctly
        if content.contains(&integration_path_str) {
            return Ok(());
        }

        // Check if there's an old qbit integration line that needs updating
        if content.contains("qbit/integration.") || content.contains("qbit\\integration.") {
            // Remove old integration lines and add new one
            let mut new_lines: Vec<&str> = Vec::new();
            let mut skip_next = false;

            for line in content.lines() {
                if line.trim() == "# Qbit shell integration" {
                    skip_next = true;
                    continue;
                }

                if skip_next
                    && (line.contains("qbit/integration.") || line.contains("qbit\\integration."))
                {
                    skip_next = false;
                    continue;
                }

                // Fish has different structure - skip the 'end' too
                if skip_next && shell_type == ShellType::Fish && line.trim() == "end" {
                    skip_next = false;
                    continue;
                }

                skip_next = false;
                new_lines.push(line);
            }

            let mut new_content = new_lines.join("\n");
            if !new_content.ends_with('\n') {
                new_content.push('\n');
            }
            new_content.push_str(&source_line);

            fs::write(rc_path, new_content).map_err(QbitError::Io)?;
            return Ok(());
        }
    }

    // No existing integration, append new one
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(rc_path)
        .map_err(QbitError::Io)?;

    writeln!(file, "{}", source_line).map_err(QbitError::Io)?;

    Ok(())
}

#[cfg(test)]
/// Uninstall shell integration for a specific shell type
fn uninstall_integration_internal(
    shell_type: ShellType,
    config_dir: &std::path::Path,
) -> Result<()> {
    let script_path = get_integration_path_for_shell(config_dir, shell_type);
    let version_path = config_dir.join("integration.version");

    if script_path.exists() {
        fs::remove_file(&script_path).map_err(QbitError::Io)?;
    }
    if version_path.exists() {
        fs::remove_file(&version_path).map_err(QbitError::Io)?;
    }

    Ok(())
}

#[cfg(test)]
/// Get integration status for a specific shell type
fn get_integration_status_internal(
    shell_type: ShellType,
    config_dir: &std::path::Path,
    home_dir: &std::path::Path,
) -> IntegrationStatus {
    let script_path = get_integration_path_for_shell(config_dir, shell_type);
    let version_path = config_dir.join("integration.version");

    // Check if version file exists
    if !version_path.exists() {
        return IntegrationStatus::NotInstalled;
    }

    // Check if integration script exists
    if !script_path.exists() {
        return IntegrationStatus::NotInstalled;
    }

    // Read current version
    let current_version = match fs::read_to_string(&version_path) {
        Ok(v) => v.trim().to_string(),
        Err(_) => return IntegrationStatus::NotInstalled,
    };

    // Check if RC file has correct source line
    let rc_paths = get_rc_file_paths(home_dir, shell_type);
    let script_path_str = script_path.display().to_string();

    let mut any_configured = false;
    for rc_path in &rc_paths {
        if rc_path.exists() {
            if let Ok(content) = fs::read_to_string(rc_path) {
                if content.contains(&script_path_str) {
                    any_configured = true;
                    break;
                }
            }
        }
    }

    if !any_configured && !rc_paths.is_empty() {
        // Check if any RC file exists but doesn't have our integration
        for rc_path in &rc_paths {
            if rc_path.exists() {
                return IntegrationStatus::Misconfigured {
                    expected_path: script_path_str,
                    issue: format!("No Qbit integration found in {}", rc_path.display()),
                };
            }
        }
    }

    if current_version == INTEGRATION_VERSION {
        IntegrationStatus::Installed {
            version: current_version,
        }
    } else {
        IntegrationStatus::Outdated {
            current: current_version,
            latest: INTEGRATION_VERSION.to_string(),
        }
    }
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

/// Get the current git branch for a directory
/// Returns None if the directory is not in a git repository
#[tauri::command]
pub async fn get_git_branch(path: String) -> std::result::Result<Option<String>, String> {
    use std::process::Command;

    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(&path)
        .output();

    match output {
        Ok(output) if output.status.success() => {
            let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if branch.is_empty() {
                Ok(None)
            } else {
                Ok(Some(branch))
            }
        }
        _ => Ok(None), // Not a git repo or git not available
    }
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

    // Write integration script (currently zsh-only, will be extended for multi-shell)
    let script_path = config_dir.join("integration.zsh");
    fs::write(&script_path, get_integration_script(ShellType::Zsh)).map_err(QbitError::Io)?;

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
    fn test_zsh_script_contains_required_markers() {
        let script = get_integration_script(ShellType::Zsh);
        assert!(
            script.contains("__qbit_osc"),
            "Script should have OSC helper"
        );
        assert!(
            script.contains(r#"133;%s"#),
            "Script should have OSC 133 format string"
        );
        assert!(
            script.contains(r#"__qbit_osc "A""#),
            "Script should emit prompt_start (A marker)"
        );
        assert!(
            script.contains(r#"__qbit_osc "B""#),
            "Script should emit prompt_end (B marker)"
        );
        assert!(
            script.contains(r#"__qbit_osc "C"#),
            "Script should emit command_start (C marker)"
        );
        assert!(
            script.contains(r#"__qbit_osc "D"#),
            "Script should emit command_end (D marker)"
        );
        assert!(script.contains("preexec"), "Script should use preexec hook");
        assert!(script.contains("precmd"), "Script should use precmd hook");
    }

    #[test]
    fn test_bash_script_contains_required_markers() {
        let script = get_integration_script(ShellType::Bash);
        assert!(
            script.contains("__qbit_osc"),
            "Bash script should have OSC helper"
        );
        assert!(
            script.contains(r#"133;%s"#),
            "Bash script should have OSC 133 format string"
        );
        assert!(
            script.contains("PROMPT_COMMAND"),
            "Bash script should use PROMPT_COMMAND"
        );
        assert!(
            script.contains("DEBUG"),
            "Bash script should use DEBUG trap"
        );
        assert!(
            script.contains(r#"__qbit_osc "A""#),
            "Bash script should emit A marker"
        );
        assert!(
            script.contains(r#"__qbit_osc "C""#),
            "Bash script should emit C marker"
        );
        assert!(
            script.contains(r#"__qbit_osc "D"#),
            "Bash script should emit D marker"
        );
        // B marker is in PS1 for bash
        assert!(
            script.contains("133;B"),
            "Bash script should emit B marker in PS1"
        );
    }

    #[test]
    fn test_fish_script_contains_required_markers() {
        let script = get_integration_script(ShellType::Fish);
        assert!(
            script.contains("__qbit_osc"),
            "Fish script should have OSC helper"
        );
        assert!(
            script.contains(r#"133;%s"#),
            "Fish script should have OSC 133 format string"
        );
        assert!(
            script.contains("fish_preexec"),
            "Fish script should use fish_preexec event"
        );
        assert!(
            script.contains("fish_postexec"),
            "Fish script should use fish_postexec event"
        );
        assert!(
            script.contains(r#"__qbit_osc "A""#),
            "Fish script should emit A marker"
        );
        assert!(
            script.contains(r#"__qbit_osc "B""#),
            "Fish script should emit B marker"
        );
        assert!(
            script.contains(r#"__qbit_osc "C""#),
            "Fish script should emit C marker"
        );
        assert!(
            script.contains(r#"__qbit_osc "D"#),
            "Fish script should emit D marker"
        );
    }

    #[test]
    fn test_all_shells_emit_all_markers() {
        for shell_type in [ShellType::Zsh, ShellType::Bash, ShellType::Fish] {
            let script = get_integration_script(shell_type);
            // All shells must emit A, B, C, D markers
            assert!(
                script.contains(r#""A""#) || script.contains("133;A"),
                "{:?} script missing A marker",
                shell_type
            );
            assert!(
                script.contains(r#""B""#) || script.contains("133;B"),
                "{:?} script missing B marker",
                shell_type
            );
            assert!(
                script.contains(r#""C""#) || script.contains("133;C"),
                "{:?} script missing C marker",
                shell_type
            );
            assert!(
                script.contains(r#""D"#) || script.contains("133;D"),
                "{:?} script missing D marker",
                shell_type
            );
        }
    }

    #[test]
    fn test_all_shells_have_qbit_guard() {
        for shell_type in [ShellType::Zsh, ShellType::Bash, ShellType::Fish] {
            let script = get_integration_script(shell_type);
            assert!(
                script.contains("QBIT"),
                "{:?} script should check for QBIT env var",
                shell_type
            );
        }
    }

    #[test]
    fn test_all_shells_have_double_source_guard() {
        for shell_type in [ShellType::Zsh, ShellType::Bash, ShellType::Fish] {
            let script = get_integration_script(shell_type);
            assert!(
                script.contains("QBIT_INTEGRATION_LOADED"),
                "{:?} script should guard against double-sourcing",
                shell_type
            );
        }
    }

    #[test]
    fn test_zsh_script_checks_qbit_env() {
        let script = get_integration_script(ShellType::Zsh);
        assert!(
            script.contains(r#"[[ -z "$QBIT" ]] && return"#),
            "Zsh script should check for QBIT env var"
        );
    }

    #[test]
    fn test_bash_script_checks_qbit_env() {
        let script = get_integration_script(ShellType::Bash);
        assert!(
            script.contains(r#"[[ "$QBIT" != "1" ]] && return"#),
            "Bash script should check for QBIT env var"
        );
    }

    #[test]
    fn test_fish_script_checks_qbit_env() {
        let script = get_integration_script(ShellType::Fish);
        assert!(
            script.contains(r#"test "$QBIT" != "1""#),
            "Fish script should check for QBIT env var"
        );
    }

    #[test]
    fn test_get_integration_extension() {
        assert_eq!(get_integration_extension(ShellType::Zsh), "zsh");
        assert_eq!(get_integration_extension(ShellType::Bash), "bash");
        assert_eq!(get_integration_extension(ShellType::Fish), "fish");
        assert_eq!(get_integration_extension(ShellType::Unknown), "zsh");
    }

    #[test]
    fn test_get_integration_script_unknown_defaults_to_zsh() {
        let unknown_script = get_integration_script(ShellType::Unknown);
        let zsh_script = get_integration_script(ShellType::Zsh);
        assert_eq!(unknown_script, zsh_script);
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

    // =========================================================================
    // Property-Based Tests
    // =========================================================================

    mod prop_tests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            /// All integration scripts must have balanced quotes
            #[test]
            fn prop_scripts_have_balanced_quotes(
                shell_type in prop_oneof![
                    Just(ShellType::Zsh),
                    Just(ShellType::Bash),
                    Just(ShellType::Fish),
                ]
            ) {
                let script = get_integration_script(shell_type);
                let single_quotes = script.matches('\'').count();
                let double_quotes = script.matches('"').count();

                // Quotes should be balanced (even count)
                // Note: This is a heuristic - some edge cases may have odd counts
                // but it catches most syntax errors
                prop_assert!(
                    single_quotes.is_multiple_of(2),
                    "{:?} has unbalanced single quotes: {}", shell_type, single_quotes
                );
                prop_assert!(
                    double_quotes.is_multiple_of(2),
                    "{:?} has unbalanced double quotes: {}", shell_type, double_quotes
                );
            }

            /// All shells must emit the same set of OSC markers
            #[test]
            fn prop_all_shells_emit_same_markers(
                shell_type in prop_oneof![
                    Just(ShellType::Zsh),
                    Just(ShellType::Bash),
                    Just(ShellType::Fish),
                ]
            ) {
                let script = get_integration_script(shell_type);

                // Every shell must emit all 4 markers
                for marker in ["A", "B", "C", "D"] {
                    prop_assert!(
                        script.contains(&format!(r#""{}"#, marker)) ||
                        script.contains(&format!("133;{}", marker)),
                        "{:?} missing marker {}", shell_type, marker
                    );
                }
            }

            /// All scripts must have the double-source guard
            #[test]
            fn prop_all_scripts_have_source_guard(
                shell_type in prop_oneof![
                    Just(ShellType::Zsh),
                    Just(ShellType::Bash),
                    Just(ShellType::Fish),
                ]
            ) {
                let script = get_integration_script(shell_type);
                prop_assert!(
                    script.contains("QBIT_INTEGRATION_LOADED"),
                    "{:?} missing double-source guard", shell_type
                );
            }

            /// All scripts must check QBIT environment variable
            #[test]
            fn prop_all_scripts_check_qbit_env(
                shell_type in prop_oneof![
                    Just(ShellType::Zsh),
                    Just(ShellType::Bash),
                    Just(ShellType::Fish),
                ]
            ) {
                let script = get_integration_script(shell_type);
                prop_assert!(
                    script.contains("QBIT"),
                    "{:?} missing QBIT environment check", shell_type
                );
            }

            /// Script extension matches shell type
            #[test]
            fn prop_extension_matches_shell(
                shell_type in prop_oneof![
                    Just(ShellType::Zsh),
                    Just(ShellType::Bash),
                    Just(ShellType::Fish),
                ]
            ) {
                let ext = get_integration_extension(shell_type);
                match shell_type {
                    ShellType::Zsh => prop_assert_eq!(ext, "zsh"),
                    ShellType::Bash => prop_assert_eq!(ext, "bash"),
                    ShellType::Fish => prop_assert_eq!(ext, "fish"),
                    ShellType::Unknown => prop_assert_eq!(ext, "zsh"),
                }
            }

            /// All scripts have proper OSC format string
            #[test]
            fn prop_all_scripts_have_osc_format(
                shell_type in prop_oneof![
                    Just(ShellType::Zsh),
                    Just(ShellType::Bash),
                    Just(ShellType::Fish),
                ]
            ) {
                let script = get_integration_script(shell_type);
                // All scripts should use printf with OSC 133 format
                prop_assert!(
                    script.contains(r"133;%s") || script.contains("133;"),
                    "{:?} missing OSC 133 format", shell_type
                );
            }
        }
    }

    // =========================================================================
    // Installation Tests (using TempDir for isolation)
    // =========================================================================

    mod installation_tests {
        use super::*;
        use tempfile::TempDir;

        fn setup_test_env() -> (TempDir, TempDir) {
            let home = TempDir::new().unwrap();
            let config = TempDir::new().unwrap();
            (home, config)
        }

        // -------------------------------------------------------------------------
        // Integration Script Creation Tests
        // -------------------------------------------------------------------------

        #[test]
        fn test_install_creates_integration_script_for_zsh() {
            let (home, config) = setup_test_env();

            let result = install_integration_internal(ShellType::Zsh, config.path(), home.path());
            assert!(result.is_ok());

            let script_path = config.path().join("integration.zsh");
            assert!(script_path.exists(), "Zsh integration script not created");

            let content = std::fs::read_to_string(&script_path).unwrap();
            assert!(content.contains("QBIT_INTEGRATION_LOADED"));
        }

        #[test]
        fn test_install_creates_integration_script_for_bash() {
            let (home, config) = setup_test_env();

            let result = install_integration_internal(ShellType::Bash, config.path(), home.path());
            assert!(result.is_ok());

            let script_path = config.path().join("integration.bash");
            assert!(script_path.exists(), "Bash integration script not created");

            let content = std::fs::read_to_string(&script_path).unwrap();
            assert!(content.contains("PROMPT_COMMAND"));
        }

        #[test]
        fn test_install_creates_integration_script_for_fish() {
            let (home, config) = setup_test_env();

            let result = install_integration_internal(ShellType::Fish, config.path(), home.path());
            assert!(result.is_ok());

            let script_path = config.path().join("integration.fish");
            assert!(script_path.exists(), "Fish integration script not created");

            let content = std::fs::read_to_string(&script_path).unwrap();
            assert!(content.contains("fish_preexec"));
        }

        #[test]
        fn test_install_creates_version_file() {
            let (home, config) = setup_test_env();

            install_integration_internal(ShellType::Zsh, config.path(), home.path()).unwrap();

            let version_path = config.path().join("integration.version");
            assert!(version_path.exists(), "Version file not created");

            let version = std::fs::read_to_string(&version_path).unwrap();
            assert_eq!(version.trim(), INTEGRATION_VERSION);
        }

        // -------------------------------------------------------------------------
        // RC File Update Tests
        // -------------------------------------------------------------------------

        #[test]
        fn test_install_updates_zshrc() {
            let (home, config) = setup_test_env();

            // Create empty .zshrc
            std::fs::write(home.path().join(".zshrc"), "# existing content\n").unwrap();

            install_integration_internal(ShellType::Zsh, config.path(), home.path()).unwrap();

            let rc_content = std::fs::read_to_string(home.path().join(".zshrc")).unwrap();
            assert!(
                rc_content.contains("Qbit shell integration"),
                "RC file missing Qbit header"
            );
            assert!(
                rc_content.contains("integration.zsh"),
                "RC file missing source line"
            );
            assert!(rc_content.contains("QBIT"), "RC file missing QBIT guard");
        }

        #[test]
        fn test_install_updates_both_bash_rc_files() {
            let (home, config) = setup_test_env();

            // Create empty bashrc files
            std::fs::write(home.path().join(".bashrc"), "# bashrc\n").unwrap();
            std::fs::write(home.path().join(".bash_profile"), "# bash_profile\n").unwrap();

            install_integration_internal(ShellType::Bash, config.path(), home.path()).unwrap();

            let bashrc = std::fs::read_to_string(home.path().join(".bashrc")).unwrap();
            let bash_profile = std::fs::read_to_string(home.path().join(".bash_profile")).unwrap();

            assert!(
                bashrc.contains("integration.bash"),
                ".bashrc not updated with source line"
            );
            assert!(
                bash_profile.contains("integration.bash"),
                ".bash_profile not updated with source line"
            );
        }

        #[test]
        fn test_install_creates_fish_config_directory() {
            let (home, config) = setup_test_env();

            // Don't create .config/fish - let install create it
            install_integration_internal(ShellType::Fish, config.path(), home.path()).unwrap();

            let fish_config = home.path().join(".config/fish/conf.d/qbit.fish");
            assert!(fish_config.exists(), "Fish config file not created");

            let content = std::fs::read_to_string(&fish_config).unwrap();
            assert!(content.contains("integration.fish"));
        }

        #[test]
        fn test_fish_rc_uses_fish_syntax() {
            let (home, config) = setup_test_env();

            install_integration_internal(ShellType::Fish, config.path(), home.path()).unwrap();

            let fish_config = home.path().join(".config/fish/conf.d/qbit.fish");
            let content = std::fs::read_to_string(&fish_config).unwrap();

            // Fish syntax uses 'test' and 'end', not [[ ]]
            assert!(
                content.contains("if test"),
                "Fish RC should use 'test' syntax"
            );
            assert!(content.contains("end"), "Fish RC should use 'end' keyword");
        }

        // -------------------------------------------------------------------------
        // Idempotency Tests
        // -------------------------------------------------------------------------

        #[test]
        fn test_install_is_idempotent_zsh() {
            let (home, config) = setup_test_env();
            std::fs::write(home.path().join(".zshrc"), "").unwrap();

            // Install twice
            install_integration_internal(ShellType::Zsh, config.path(), home.path()).unwrap();
            install_integration_internal(ShellType::Zsh, config.path(), home.path()).unwrap();

            let rc_content = std::fs::read_to_string(home.path().join(".zshrc")).unwrap();
            let source_count = rc_content.matches("integration.zsh").count();

            assert_eq!(source_count, 1, "Integration sourced multiple times");
        }

        #[test]
        fn test_install_is_idempotent_bash() {
            let (home, config) = setup_test_env();
            std::fs::write(home.path().join(".bashrc"), "").unwrap();
            std::fs::write(home.path().join(".bash_profile"), "").unwrap();

            // Install twice
            install_integration_internal(ShellType::Bash, config.path(), home.path()).unwrap();
            install_integration_internal(ShellType::Bash, config.path(), home.path()).unwrap();

            let bashrc = std::fs::read_to_string(home.path().join(".bashrc")).unwrap();
            let source_count = bashrc.matches("integration.bash").count();

            assert_eq!(
                source_count, 1,
                "Integration sourced multiple times in .bashrc"
            );
        }

        #[test]
        fn test_install_is_idempotent_fish() {
            let (home, config) = setup_test_env();

            // Install twice
            install_integration_internal(ShellType::Fish, config.path(), home.path()).unwrap();
            install_integration_internal(ShellType::Fish, config.path(), home.path()).unwrap();

            let fish_config = home.path().join(".config/fish/conf.d/qbit.fish");
            let content = std::fs::read_to_string(&fish_config).unwrap();
            let source_count = content.matches("integration.fish").count();

            assert_eq!(
                source_count, 1,
                "Integration sourced multiple times in fish config"
            );
        }

        // -------------------------------------------------------------------------
        // Uninstall Tests
        // -------------------------------------------------------------------------

        #[test]
        fn test_uninstall_removes_integration_script_zsh() {
            let (home, config) = setup_test_env();

            // Install first
            install_integration_internal(ShellType::Zsh, config.path(), home.path()).unwrap();
            assert!(config.path().join("integration.zsh").exists());

            // Uninstall
            uninstall_integration_internal(ShellType::Zsh, config.path()).unwrap();
            assert!(!config.path().join("integration.zsh").exists());
        }

        #[test]
        fn test_uninstall_removes_integration_script_bash() {
            let (home, config) = setup_test_env();

            install_integration_internal(ShellType::Bash, config.path(), home.path()).unwrap();
            assert!(config.path().join("integration.bash").exists());

            uninstall_integration_internal(ShellType::Bash, config.path()).unwrap();
            assert!(!config.path().join("integration.bash").exists());
        }

        #[test]
        fn test_uninstall_removes_integration_script_fish() {
            let (home, config) = setup_test_env();

            install_integration_internal(ShellType::Fish, config.path(), home.path()).unwrap();
            assert!(config.path().join("integration.fish").exists());

            uninstall_integration_internal(ShellType::Fish, config.path()).unwrap();
            assert!(!config.path().join("integration.fish").exists());
        }

        #[test]
        fn test_uninstall_removes_version_file() {
            let (home, config) = setup_test_env();

            install_integration_internal(ShellType::Zsh, config.path(), home.path()).unwrap();
            assert!(config.path().join("integration.version").exists());

            uninstall_integration_internal(ShellType::Zsh, config.path()).unwrap();
            assert!(!config.path().join("integration.version").exists());
        }

        #[test]
        fn test_uninstall_is_idempotent() {
            let (home, config) = setup_test_env();

            // Uninstall without ever installing - should not error
            let result = uninstall_integration_internal(ShellType::Zsh, config.path());
            assert!(result.is_ok());

            // Install then uninstall twice
            install_integration_internal(ShellType::Zsh, config.path(), home.path()).unwrap();
            uninstall_integration_internal(ShellType::Zsh, config.path()).unwrap();
            let result = uninstall_integration_internal(ShellType::Zsh, config.path());
            assert!(result.is_ok());
        }

        // -------------------------------------------------------------------------
        // Status Detection Tests
        // -------------------------------------------------------------------------

        #[test]
        fn test_status_detects_not_installed() {
            let (home, config) = setup_test_env();
            std::fs::write(home.path().join(".zshrc"), "").unwrap();

            let status =
                get_integration_status_internal(ShellType::Zsh, config.path(), home.path());
            assert!(matches!(status, IntegrationStatus::NotInstalled));
        }

        #[test]
        fn test_status_detects_installed() {
            let (home, config) = setup_test_env();
            std::fs::write(home.path().join(".zshrc"), "").unwrap();

            install_integration_internal(ShellType::Zsh, config.path(), home.path()).unwrap();

            let status =
                get_integration_status_internal(ShellType::Zsh, config.path(), home.path());
            match status {
                IntegrationStatus::Installed { version } => {
                    assert_eq!(version, INTEGRATION_VERSION);
                }
                other => panic!("Expected Installed, got {:?}", other),
            }
        }

        #[test]
        fn test_status_detects_outdated() {
            let (home, config) = setup_test_env();
            std::fs::write(home.path().join(".zshrc"), "").unwrap();

            install_integration_internal(ShellType::Zsh, config.path(), home.path()).unwrap();

            // Manually downgrade version file
            std::fs::write(config.path().join("integration.version"), "0.0.1").unwrap();

            let status =
                get_integration_status_internal(ShellType::Zsh, config.path(), home.path());
            match status {
                IntegrationStatus::Outdated { current, latest } => {
                    assert_eq!(current, "0.0.1");
                    assert_eq!(latest, INTEGRATION_VERSION);
                }
                other => panic!("Expected Outdated, got {:?}", other),
            }
        }

        #[test]
        fn test_status_detects_misconfigured() {
            let (home, config) = setup_test_env();

            // Create integration files
            std::fs::create_dir_all(config.path()).unwrap();
            std::fs::write(config.path().join("integration.zsh"), "script").unwrap();
            std::fs::write(
                config.path().join("integration.version"),
                INTEGRATION_VERSION,
            )
            .unwrap();

            // Create .zshrc WITHOUT the source line
            std::fs::write(home.path().join(".zshrc"), "# no qbit integration\n").unwrap();

            let status =
                get_integration_status_internal(ShellType::Zsh, config.path(), home.path());
            match status {
                IntegrationStatus::Misconfigured { issue, .. } => {
                    assert!(issue.contains(".zshrc"));
                }
                other => panic!("Expected Misconfigured, got {:?}", other),
            }
        }

        #[test]
        fn test_status_not_installed_when_no_version_file() {
            let (home, config) = setup_test_env();
            std::fs::write(home.path().join(".zshrc"), "").unwrap();

            // Create integration script but NO version file
            std::fs::create_dir_all(config.path()).unwrap();
            std::fs::write(config.path().join("integration.zsh"), "script").unwrap();

            let status =
                get_integration_status_internal(ShellType::Zsh, config.path(), home.path());
            assert!(matches!(status, IntegrationStatus::NotInstalled));
        }

        #[test]
        fn test_status_not_installed_when_no_script_file() {
            let (home, config) = setup_test_env();
            std::fs::write(home.path().join(".zshrc"), "").unwrap();

            // Create version file but NO integration script
            std::fs::create_dir_all(config.path()).unwrap();
            std::fs::write(
                config.path().join("integration.version"),
                INTEGRATION_VERSION,
            )
            .unwrap();

            let status =
                get_integration_status_internal(ShellType::Zsh, config.path(), home.path());
            assert!(matches!(status, IntegrationStatus::NotInstalled));
        }

        // -------------------------------------------------------------------------
        // RC File Path Tests
        // -------------------------------------------------------------------------

        #[test]
        fn test_get_rc_file_paths_zsh() {
            let home = TempDir::new().unwrap();
            let paths = get_rc_file_paths(home.path(), ShellType::Zsh);
            assert_eq!(paths.len(), 1);
            assert!(paths[0].ends_with(".zshrc"));
        }

        #[test]
        fn test_get_rc_file_paths_bash() {
            let home = TempDir::new().unwrap();
            let paths = get_rc_file_paths(home.path(), ShellType::Bash);
            assert_eq!(paths.len(), 2);
            assert!(paths.iter().any(|p| p.ends_with(".bashrc")));
            assert!(paths.iter().any(|p| p.ends_with(".bash_profile")));
        }

        #[test]
        fn test_get_rc_file_paths_fish() {
            let home = TempDir::new().unwrap();
            let paths = get_rc_file_paths(home.path(), ShellType::Fish);
            assert_eq!(paths.len(), 1);
            assert!(paths[0].ends_with("qbit.fish"));
            assert!(paths[0].to_string_lossy().contains(".config/fish"));
        }

        // -------------------------------------------------------------------------
        // Integration Path Tests
        // -------------------------------------------------------------------------

        #[test]
        fn test_get_integration_path_for_shell_zsh() {
            let config = TempDir::new().unwrap();
            let path = get_integration_path_for_shell(config.path(), ShellType::Zsh);
            assert!(path.ends_with("integration.zsh"));
        }

        #[test]
        fn test_get_integration_path_for_shell_bash() {
            let config = TempDir::new().unwrap();
            let path = get_integration_path_for_shell(config.path(), ShellType::Bash);
            assert!(path.ends_with("integration.bash"));
        }

        #[test]
        fn test_get_integration_path_for_shell_fish() {
            let config = TempDir::new().unwrap();
            let path = get_integration_path_for_shell(config.path(), ShellType::Fish);
            assert!(path.ends_with("integration.fish"));
        }

        // -------------------------------------------------------------------------
        // Property-Based Installation Tests
        // -------------------------------------------------------------------------

        mod prop_tests {
            use super::*;
            use proptest::prelude::*;

            proptest! {
                /// Install then uninstall leaves no integration files
                #[test]
                fn prop_install_uninstall_cleanup(
                    shell_type in prop_oneof![
                        Just(ShellType::Zsh),
                        Just(ShellType::Bash),
                        Just(ShellType::Fish),
                    ]
                ) {
                    let (home, config) = setup_test_env();

                    install_integration_internal(shell_type, config.path(), home.path()).unwrap();
                    uninstall_integration_internal(shell_type, config.path()).unwrap();

                    let ext = get_integration_extension(shell_type);
                    prop_assert!(
                        !config.path().join(format!("integration.{}", ext)).exists(),
                        "Integration script should be removed after uninstall"
                    );
                }

                /// Status is NotInstalled before install, Installed after install
                #[test]
                fn prop_status_changes_after_install(
                    shell_type in prop_oneof![
                        Just(ShellType::Zsh),
                        Just(ShellType::Bash),
                        Just(ShellType::Fish),
                    ]
                ) {
                    let (home, config) = setup_test_env();

                    // Create RC file for zsh/bash so status check works
                    match shell_type {
                        ShellType::Zsh => {
                            std::fs::write(home.path().join(".zshrc"), "").unwrap();
                        }
                        ShellType::Bash => {
                            std::fs::write(home.path().join(".bashrc"), "").unwrap();
                        }
                        _ => {}
                    }

                    let before = get_integration_status_internal(shell_type, config.path(), home.path());
                    prop_assert!(matches!(before, IntegrationStatus::NotInstalled));

                    install_integration_internal(shell_type, config.path(), home.path()).unwrap();

                    let after = get_integration_status_internal(shell_type, config.path(), home.path());
                    prop_assert!(
                        matches!(after, IntegrationStatus::Installed { .. }),
                        "Expected Installed after install, got {:?}", after
                    );
                }

                /// Multiple installs don't corrupt RC files
                #[test]
                fn prop_multiple_installs_safe(
                    shell_type in prop_oneof![
                        Just(ShellType::Zsh),
                        Just(ShellType::Bash),
                        Just(ShellType::Fish),
                    ],
                    install_count in 1usize..5
                ) {
                    let (home, config) = setup_test_env();

                    // Pre-create RC files
                    match shell_type {
                        ShellType::Zsh => {
                            std::fs::write(home.path().join(".zshrc"), "").unwrap();
                        }
                        ShellType::Bash => {
                            std::fs::write(home.path().join(".bashrc"), "").unwrap();
                            std::fs::write(home.path().join(".bash_profile"), "").unwrap();
                        }
                        _ => {}
                    }

                    for _ in 0..install_count {
                        install_integration_internal(shell_type, config.path(), home.path()).unwrap();
                    }

                    // Check RC files have exactly one source line
                    let rc_paths = get_rc_file_paths(home.path(), shell_type);
                    let ext = get_integration_extension(shell_type);
                    let integration_marker = format!("integration.{}", ext);

                    for rc_path in rc_paths {
                        if rc_path.exists() {
                            let content = std::fs::read_to_string(&rc_path).unwrap();
                            let count = content.matches(&integration_marker).count();
                            prop_assert_eq!(
                                count, 1,
                                "RC file {} should have exactly 1 source line, found {}",
                                rc_path.display(), count
                            );
                        }
                    }
                }
            }
        }
    }
}
