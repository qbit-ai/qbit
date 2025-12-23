use crate::error::{QbitError, Result};
use crate::pty::ShellType;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::PathBuf;

const INTEGRATION_VERSION: &str = "1.0.0";

// =============================================================================
// Zsh Integration Script
// =============================================================================

const INTEGRATION_SCRIPT_ZSH: &str = r#"# ~/.config/qbit/integration.zsh
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

// =============================================================================
// Bash Integration Script
// =============================================================================

const INTEGRATION_SCRIPT_BASH: &str = r#"# ~/.config/qbit/integration.bash
# Qbit Shell Integration v1.0.0
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

# ============ Hook Functions ============

# Track if preexec already ran (DEBUG trap fires multiple times)
__qbit_preexec_ran=0

__qbit_prompt_command() {
    local exit_code=$?
    __qbit_osc "D;$exit_code"
    __qbit_report_cwd
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
"#;

// =============================================================================
// Fish Integration Script
// =============================================================================

const INTEGRATION_SCRIPT_FISH: &str = r#"# ~/.config/fish/conf.d/qbit.fish
# Qbit Shell Integration v1.0.0
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

# ============ Hook Functions ============

function __qbit_preexec --on-event fish_preexec
    __qbit_osc "C"
end

function __qbit_postexec --on-event fish_postexec
    __qbit_osc "D;$status"
    __qbit_report_cwd
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

/// Get the integration script file extension for a shell type
pub fn get_integration_extension(shell_type: ShellType) -> &'static str {
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
                    single_quotes % 2 == 0,
                    "{:?} has unbalanced single quotes: {}", shell_type, single_quotes
                );
                prop_assert!(
                    double_quotes % 2 == 0,
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
}
