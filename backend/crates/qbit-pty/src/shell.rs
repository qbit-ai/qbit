//! Shell detection and configuration for multi-shell support.
//!
//! This module provides shell type detection from paths and settings,
//! supporting zsh, bash, and fish shells.
//!
//! ## Automatic Shell Integration
//!
//! The `ShellIntegration` struct provides automatic shell integration injection
//! using the ZDOTDIR approach for zsh. This allows OSC 133 sequences to be emitted
//! without requiring users to manually edit their `.zshrc` files.

use std::fs;
use std::path::{Path, PathBuf};

use qbit_settings::schema::TerminalSettings;

/// Supported shell types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellType {
    Zsh,
    Bash,
    Fish,
    Unknown,
}

impl ShellType {
    /// Get login shell arguments for this shell type
    pub fn login_args(&self) -> Vec<&'static str> {
        match self {
            ShellType::Zsh | ShellType::Bash | ShellType::Fish => vec!["-l"],
            ShellType::Unknown => vec![],
        }
    }
}

/// Shell detection and configuration
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellInfo {
    /// Path to the shell executable
    pub path: PathBuf,
    shell_type: ShellType,
}

impl ShellInfo {
    /// Create a new ShellInfo from a shell path
    pub fn new(path: impl AsRef<Path>) -> Self {
        let path = path.as_ref().to_path_buf();
        let shell_type = Self::detect_type(&path);
        Self { path, shell_type }
    }

    /// Get the detected shell type
    pub fn shell_type(&self) -> ShellType {
        self.shell_type
    }

    /// Get login shell arguments
    pub fn login_args(&self) -> Vec<&'static str> {
        self.shell_type.login_args()
    }

    /// Detect shell type from path
    fn detect_type(path: &Path) -> ShellType {
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        match file_name {
            "zsh" => ShellType::Zsh,
            "bash" => ShellType::Bash,
            "fish" => ShellType::Fish,
            _ => ShellType::Unknown,
        }
    }
}

/// Detect shell from settings or environment.
///
/// Priority:
/// 1. `settings.terminal.shell` (user override)
/// 2. `shell_env` ($SHELL environment variable)
/// 3. Fallback to `/bin/sh`
pub fn detect_shell(settings: Option<&TerminalSettings>, shell_env: Option<&str>) -> ShellInfo {
    // Priority 1: Settings override
    if let Some(settings) = settings {
        if let Some(ref shell) = settings.shell {
            return ShellInfo::new(shell);
        }
    }

    // Priority 2: Environment variable
    if let Some(shell) = shell_env {
        return ShellInfo::new(shell);
    }

    // Priority 3: Fallback
    ShellInfo::new("/bin/sh")
}

// =============================================================================
// Shell Integration - Automatic OSC 133 injection via ZDOTDIR
// =============================================================================

/// The zsh integration script that emits OSC 133 sequences.
/// This is embedded in the binary to avoid file path dependencies.
const ZSH_INTEGRATION_SCRIPT: &str = r#"# Qbit Shell Integration (auto-injected)
# Emits OSC 133 sequences for command tracking

# Debug: confirm script is being sourced
[[ -n "$QBIT_DEBUG" ]] && echo "[qbit-integration] Loading integration script..."

# Guard against double-sourcing (use unique var to avoid conflict with old integration)
if [[ -n "$__QBIT_OSC133_LOADED" ]]; then
    [[ -n "$QBIT_DEBUG" ]] && echo "[qbit-integration] Already loaded, skipping"
    return
fi
export __QBIT_OSC133_LOADED=1

[[ -n "$QBIT_DEBUG" ]] && echo "[qbit-integration] Registering hooks..."

# ============ OSC Helpers ============

__qbit_osc() {
    printf '\e]133;%s\a' "$1"
}

__qbit_report_cwd() {
    printf '\e]7;file://%s%s\a' "${HOST:-$(hostname)}" "$PWD"
}

__qbit_report_venv() {
    if [[ -n "$VIRTUAL_ENV" ]]; then
        local venv_name="${VIRTUAL_ENV##*/}"
        printf '\e]1337;VirtualEnv=%s\a' "$venv_name"
    else
        printf '\e]1337;VirtualEnv=\a'
    fi
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
}

__qbit_cmd_end() {
    local exit_code=${1:-0}
    __qbit_osc "D;$exit_code"
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

/// The bash integration script that emits OSC 133 sequences.
/// Uses PROMPT_COMMAND for precmd and DEBUG trap for preexec.
/// IMPORTANT: The DEBUG trap is installed lazily on the first prompt to avoid
/// capturing commands from .bashrc during shell startup.
///
/// Note on OSC 133;B (PromptEnd): We emit B immediately after A in precmd.
/// This means:
/// - A→B transition happens atomically (Prompt region is effectively empty)
/// - PS1 renders in Input region (visible in terminal, filtered from timeline)
/// - PS2 continuation prompts are in Input region (visible)
/// - User input is in Input region (visible)
/// - C is emitted in preexec when command actually starts
/// - Command output is in Output region (shown in timeline)
const BASH_INTEGRATION_SCRIPT: &str = r#"# Qbit Shell Integration for Bash (auto-injected)
# Emits OSC 133 sequences for command tracking

# Guard against double-sourcing
if [[ -n "$__QBIT_OSC133_LOADED" ]]; then
    return 0 2>/dev/null || exit 0
fi
export __QBIT_OSC133_LOADED=1

# ============ State Variables ============

# Track whether we're at the start of a command (to avoid duplicate preexec)
__qbit_at_prompt=0
# Flag to install DEBUG trap on first prompt (avoids capturing .bashrc commands)
__qbit_trap_installed=0

# ============ OSC Helpers ============

__qbit_osc() {
    printf '\e]133;%s\a' "$1"
}

__qbit_report_cwd() {
    printf '\e]7;file://%s%s\a' "${HOSTNAME:-$(hostname)}" "$PWD"
}

__qbit_report_venv() {
    if [[ -n "$VIRTUAL_ENV" ]]; then
        printf '\e]1337;VirtualEnv=%s\a' "${VIRTUAL_ENV##*/}"
    else
        printf '\e]1337;VirtualEnv=\a'
    fi
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
}

__qbit_cmd_end() {
    __qbit_osc "D;${1:-0}"
}

# ============ Hook Functions ============

# Preexec: called before each command via DEBUG trap
__qbit_preexec() {
    # Only run if we're at a prompt (not during prompt rendering or subshells)
    [[ "$__qbit_at_prompt" != "1" ]] && return

    # Get the command being executed
    local cmd="$BASH_COMMAND"

    # Skip our own functions
    [[ "$cmd" == *"__qbit_"* ]] && return

    # Skip shell internals (return from functions, etc.)
    [[ "$cmd" == "return"* ]] && return

    __qbit_at_prompt=0
    # Only emit CommandStart here - PromptEnd (B) was already emitted in precmd
    __qbit_cmd_start "$cmd"
}

# Precmd: called before each prompt via PROMPT_COMMAND
__qbit_precmd() {
    local exit_code=$?

    # Install DEBUG trap on first prompt (after .bashrc has finished)
    if [[ "$__qbit_trap_installed" == "0" ]]; then
        __qbit_trap_installed=1
        # Chain with any existing DEBUG trap
        local existing_trap
        existing_trap=$(trap -p DEBUG 2>/dev/null | sed "s/trap -- '\\(.*\\)' DEBUG/\\1/")
        if [[ -n "$existing_trap" && "$existing_trap" != "__qbit_preexec" ]]; then
            eval "__qbit_orig_debug_trap() { $existing_trap; }"
            trap '__qbit_preexec; __qbit_orig_debug_trap' DEBUG
        else
            trap '__qbit_preexec' DEBUG
        fi
    fi

    # Emit command end if we ran a command
    if [[ "$__qbit_at_prompt" != "1" ]]; then
        __qbit_cmd_end $exit_code
    fi

    __qbit_report_cwd
    __qbit_report_venv
    __qbit_prompt_start
    # Emit PromptEnd immediately after PromptStart
    # This makes the Prompt region effectively empty, and puts PS1/PS2/input
    # in the Input region where they are visible in the terminal but filtered
    # from command block output (which only shows Output region: C to D)
    __qbit_prompt_end

    __qbit_at_prompt=1
    return $exit_code
}

# ============ Setup ============

# Install PROMPT_COMMAND (DEBUG trap is installed lazily on first prompt)
if [[ -z "$PROMPT_COMMAND" ]]; then
    PROMPT_COMMAND="__qbit_precmd"
elif [[ "$PROMPT_COMMAND" != *"__qbit_precmd"* ]]; then
    PROMPT_COMMAND="__qbit_precmd; $PROMPT_COMMAND"
fi
"#;

/// The wrapper .zshrc that sources our integration BEFORE user's config.
/// This ensures our hooks run even if user's .zshrc has old integration lines.
const ZSH_WRAPPER_ZSHRC: &str = r#"# Qbit ZDOTDIR wrapper - sources integration + user config

# Debug: confirm wrapper is being sourced
[[ -n "$QBIT_DEBUG" ]] && echo "[qbit-wrapper] ZDOTDIR wrapper .zshrc loading..."
[[ -n "$QBIT_DEBUG" ]] && echo "[qbit-wrapper] QBIT_INTEGRATION_PATH=$QBIT_INTEGRATION_PATH"

# Source Qbit integration FIRST (before user config)
# This ensures our OSC 133 hooks are always registered, even if user's
# .zshrc has an old integration line that would set QBIT_INTEGRATION_LOADED
if [[ -f "$QBIT_INTEGRATION_PATH" ]]; then
    source "$QBIT_INTEGRATION_PATH"
fi

# Now source the user's original .zshrc
# If it has an old integration line, the guard will skip it (QBIT_INTEGRATION_LOADED=1)
if [[ -n "$QBIT_REAL_ZDOTDIR" && "$QBIT_REAL_ZDOTDIR" != "$ZDOTDIR" ]]; then
    # Guard: skip sourcing when QBIT_REAL_ZDOTDIR points back at this wrapper
    # dir (nested Qbit). Without this check we'd source ourselves infinitely.
    if [[ -f "$QBIT_REAL_ZDOTDIR/.zshrc" ]]; then
        ZDOTDIR="$QBIT_REAL_ZDOTDIR"
        source "$QBIT_REAL_ZDOTDIR/.zshrc"
    fi
elif [[ -z "$QBIT_REAL_ZDOTDIR" && -f "$HOME/.zshrc" ]]; then
    source "$HOME/.zshrc"
fi
"#;

/// Manages shell integration files for automatic OSC 133 injection.
///
/// For zsh, uses the ZDOTDIR approach:
/// 1. Creates a wrapper `.zshrc` in a config directory
/// 2. This wrapper sources the user's real `.zshrc` AND our integration script
/// 3. Sets ZDOTDIR to point to this wrapper directory
///
/// For bash, uses the BASH_ENV approach:
/// 1. Creates an integration script in a config directory
/// 2. Sets BASH_ENV to point to this script (sourced for non-interactive shells)
/// 3. Also sources via --rcfile mechanism for interactive shells
///
/// This allows shell integration to work without modifying user config files.
pub struct ShellIntegration {
    /// The shell type this integration is for
    shell_type: ShellType,
    /// Directory containing shell integration files
    config_dir: PathBuf,
    /// Path to the integration script
    integration_path: PathBuf,
}

impl ShellIntegration {
    /// Set up shell integration for the given shell type.
    ///
    /// Returns `None` for unsupported shells.
    pub fn setup(shell_type: ShellType) -> Option<Self> {
        match shell_type {
            ShellType::Zsh => Self::setup_zsh(),
            ShellType::Bash => Self::setup_bash(),
            // TODO: Add fish support via conf.d
            _ => None,
        }
    }

    /// Set up zsh integration using ZDOTDIR.
    fn setup_zsh() -> Option<Self> {
        // Use ~/.config/qbit/shell as our ZDOTDIR
        let config_dir = dirs::config_dir()?.join("qbit").join("shell");

        // Create directories
        if fs::create_dir_all(&config_dir).is_err() {
            tracing::warn!("Failed to create shell integration directory");
            return None;
        }

        // Write the integration script
        let integration_path = config_dir.join("integration.zsh");
        if let Err(e) = fs::write(&integration_path, ZSH_INTEGRATION_SCRIPT) {
            tracing::warn!("Failed to write integration script: {}", e);
            return None;
        }

        // Write the wrapper .zshrc
        let zshrc_path = config_dir.join(".zshrc");
        if let Err(e) = fs::write(&zshrc_path, ZSH_WRAPPER_ZSHRC) {
            tracing::warn!("Failed to write wrapper .zshrc: {}", e);
            return None;
        }

        tracing::debug!(
            zdotdir = %config_dir.display(),
            integration = %integration_path.display(),
            "Zsh integration configured"
        );

        Some(Self {
            shell_type: ShellType::Zsh,
            config_dir,
            integration_path,
        })
    }

    /// Set up bash integration using --rcfile.
    ///
    /// For bash, we create a wrapper script that:
    /// 1. Sources our integration script
    /// 2. Sources the user's ~/.bashrc
    ///
    /// Then we use `--rcfile wrapper.bash` when spawning bash.
    fn setup_bash() -> Option<Self> {
        // Use ~/.config/qbit/shell/bash for bash integration
        let config_dir = dirs::config_dir()?.join("qbit").join("shell").join("bash");

        // Create directories
        if fs::create_dir_all(&config_dir).is_err() {
            tracing::warn!("Failed to create bash integration directory");
            return None;
        }

        // Write the integration script
        let integration_path = config_dir.join("integration.bash");
        if let Err(e) = fs::write(&integration_path, BASH_INTEGRATION_SCRIPT) {
            tracing::warn!("Failed to write bash integration script: {}", e);
            return None;
        }

        // Write a wrapper script that sources integration + user's bashrc
        let wrapper_path = config_dir.join("wrapper.bash");
        let wrapper_content = format!(
            r#"# Qbit Bash Wrapper (auto-generated)
# Sources Qbit integration before user's bashrc

# Source Qbit integration first
if [[ -f "{integration}" ]]; then
    source "{integration}"
fi

# Source user's bashrc
if [[ -f "$HOME/.bashrc" ]]; then
    source "$HOME/.bashrc"
fi
"#,
            integration = integration_path.to_string_lossy()
        );
        if let Err(e) = fs::write(&wrapper_path, wrapper_content) {
            tracing::warn!("Failed to write bash wrapper script: {}", e);
            return None;
        }

        tracing::debug!(
            config_dir = %config_dir.display(),
            integration = %integration_path.display(),
            wrapper = %wrapper_path.display(),
            "Bash integration configured"
        );

        Some(Self {
            shell_type: ShellType::Bash,
            config_dir,
            integration_path,
        })
    }

    /// Get environment variables to set for the shell process.
    ///
    /// Returns a list of (key, value) pairs to set in the PTY environment.
    pub fn env_vars(&self) -> Vec<(&'static str, String)> {
        match self.shell_type {
            ShellType::Zsh => {
                let mut vars = vec![
                    ("ZDOTDIR", self.config_dir.to_string_lossy().to_string()),
                    (
                        "QBIT_INTEGRATION_PATH",
                        self.integration_path.to_string_lossy().to_string(),
                    ),
                ];

                // Preserve user's original ZDOTDIR if set, but only when it
                // differs from our wrapper dir. When a nested Qbit inherits
                // ZDOTDIR pointing at the wrapper, forwarding it as
                // QBIT_REAL_ZDOTDIR would cause the wrapper .zshrc to source
                // itself, leading to infinite recursion ("job table full").
                if let Ok(original) = std::env::var("ZDOTDIR") {
                    let wrapper_dir = self.config_dir.to_string_lossy();
                    if original != wrapper_dir.as_ref() {
                        vars.push(("QBIT_REAL_ZDOTDIR", original));
                    }
                }

                vars
            }
            ShellType::Bash => {
                vec![(
                    "QBIT_INTEGRATION_PATH",
                    self.integration_path.to_string_lossy().to_string(),
                )]
            }
            _ => vec![],
        }
    }

    /// Get additional arguments to pass to the shell.
    ///
    /// For bash, this returns `["--rcfile", "/path/to/wrapper.bash"]`.
    /// For other shells, returns empty.
    pub fn shell_args(&self) -> Vec<String> {
        match self.shell_type {
            ShellType::Bash => {
                let wrapper_path = self.config_dir.join("wrapper.bash");
                vec![
                    "--rcfile".to_string(),
                    wrapper_path.to_string_lossy().to_string(),
                ]
            }
            _ => vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // ShellType Tests
    // =========================================================================

    #[test]
    fn test_shell_type_login_args_zsh() {
        assert_eq!(ShellType::Zsh.login_args(), vec!["-l"]);
    }

    #[test]
    fn test_shell_type_login_args_bash() {
        assert_eq!(ShellType::Bash.login_args(), vec!["-l"]);
    }

    #[test]
    fn test_shell_type_login_args_fish() {
        assert_eq!(ShellType::Fish.login_args(), vec!["-l"]);
    }

    #[test]
    fn test_shell_type_login_args_unknown() {
        assert_eq!(ShellType::Unknown.login_args(), Vec::<&str>::new());
    }

    // =========================================================================
    // ShellInfo Detection Tests
    // =========================================================================

    #[test]
    fn test_shell_info_identifies_zsh() {
        let info = ShellInfo::new("/usr/local/bin/zsh");
        assert_eq!(info.shell_type(), ShellType::Zsh);
    }

    #[test]
    fn test_shell_info_identifies_zsh_standard_path() {
        let info = ShellInfo::new("/bin/zsh");
        assert_eq!(info.shell_type(), ShellType::Zsh);
    }

    #[test]
    fn test_shell_info_identifies_bash() {
        let info = ShellInfo::new("/bin/bash");
        assert_eq!(info.shell_type(), ShellType::Bash);
    }

    #[test]
    fn test_shell_info_identifies_bash_usr_bin() {
        let info = ShellInfo::new("/usr/bin/bash");
        assert_eq!(info.shell_type(), ShellType::Bash);
    }

    #[test]
    fn test_shell_info_identifies_fish() {
        let info = ShellInfo::new("/opt/homebrew/bin/fish");
        assert_eq!(info.shell_type(), ShellType::Fish);
    }

    #[test]
    fn test_shell_info_identifies_fish_usr_bin() {
        let info = ShellInfo::new("/usr/bin/fish");
        assert_eq!(info.shell_type(), ShellType::Fish);
    }

    #[test]
    fn test_shell_info_unknown_shell_tcsh() {
        let info = ShellInfo::new("/bin/tcsh");
        assert_eq!(info.shell_type(), ShellType::Unknown);
    }

    #[test]
    fn test_shell_info_unknown_shell_sh() {
        let info = ShellInfo::new("/bin/sh");
        assert_eq!(info.shell_type(), ShellType::Unknown);
    }

    #[test]
    fn test_shell_info_unknown_shell_ksh() {
        let info = ShellInfo::new("/bin/ksh");
        assert_eq!(info.shell_type(), ShellType::Unknown);
    }

    #[test]
    fn test_shell_info_login_args_from_zsh() {
        let info = ShellInfo::new("/bin/zsh");
        assert_eq!(info.login_args(), vec!["-l"]);
    }

    #[test]
    fn test_shell_info_login_args_from_bash() {
        let info = ShellInfo::new("/bin/bash");
        assert_eq!(info.login_args(), vec!["-l"]);
    }

    #[test]
    fn test_shell_info_login_args_from_fish() {
        let info = ShellInfo::new("/usr/bin/fish");
        assert_eq!(info.login_args(), vec!["-l"]);
    }

    #[test]
    fn test_shell_info_preserves_path() {
        let path = "/opt/homebrew/bin/zsh";
        let info = ShellInfo::new(path);
        assert_eq!(info.path, PathBuf::from(path));
    }

    // =========================================================================
    // detect_shell() Priority Tests
    // =========================================================================

    #[test]
    fn test_detect_shell_from_settings_override() {
        let settings = TerminalSettings {
            shell: Some("/bin/bash".into()),
            ..Default::default()
        };
        let info = detect_shell(Some(&settings), Some("/bin/zsh"));
        assert_eq!(info.path, PathBuf::from("/bin/bash"));
        assert_eq!(info.shell_type(), ShellType::Bash);
    }

    #[test]
    fn test_detect_shell_from_env_when_no_settings_shell() {
        let settings = TerminalSettings {
            shell: None,
            ..Default::default()
        };
        let info = detect_shell(Some(&settings), Some("/bin/fish"));
        assert_eq!(info.path, PathBuf::from("/bin/fish"));
        assert_eq!(info.shell_type(), ShellType::Fish);
    }

    #[test]
    fn test_detect_shell_from_env_when_no_settings() {
        let info = detect_shell(None, Some("/bin/zsh"));
        assert_eq!(info.path, PathBuf::from("/bin/zsh"));
        assert_eq!(info.shell_type(), ShellType::Zsh);
    }

    #[test]
    fn test_detect_shell_fallback_to_sh() {
        let info = detect_shell(None, None);
        assert_eq!(info.path, PathBuf::from("/bin/sh"));
        assert_eq!(info.shell_type(), ShellType::Unknown);
    }

    #[test]
    fn test_detect_shell_fallback_when_settings_shell_is_none() {
        let settings = TerminalSettings {
            shell: None,
            ..Default::default()
        };
        let info = detect_shell(Some(&settings), None);
        assert_eq!(info.path, PathBuf::from("/bin/sh"));
    }

    // =========================================================================
    // Error Handling Tests
    // =========================================================================

    #[test]
    fn test_detect_shell_handles_nonexistent_path() {
        let settings = TerminalSettings {
            shell: Some("/nonexistent/shell".into()),
            ..Default::default()
        };
        let info = detect_shell(Some(&settings), None);
        // Should still create ShellInfo, but with Unknown type
        assert_eq!(info.shell_type(), ShellType::Unknown);
        assert_eq!(info.path, PathBuf::from("/nonexistent/shell"));
    }

    #[test]
    fn test_shell_info_handles_empty_path() {
        let info = ShellInfo::new("");
        assert_eq!(info.shell_type(), ShellType::Unknown);
    }

    #[test]
    fn test_shell_info_handles_directory_path() {
        let info = ShellInfo::new("/bin/");
        assert_eq!(info.shell_type(), ShellType::Unknown);
    }

    // =========================================================================
    // Property-Based Tests
    // =========================================================================

    mod prop_tests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            /// Shell detection is idempotent - calling new() twice gives same result
            #[test]
            fn prop_shell_detection_is_idempotent(
                path in prop_oneof![
                    Just("/bin/bash"),
                    Just("/bin/zsh"),
                    Just("/usr/bin/fish"),
                    Just("/opt/homebrew/bin/fish"),
                    Just("/usr/local/bin/zsh"),
                    Just("/bin/sh"),
                    Just("/bin/tcsh"),
                    Just("/nonexistent/shell"),
                ]
            ) {
                let info1 = ShellInfo::new(path);
                let info2 = ShellInfo::new(path);
                prop_assert_eq!(info1.shell_type(), info2.shell_type());
                prop_assert_eq!(info1.login_args(), info2.login_args());
                prop_assert_eq!(info1.path, info2.path);
            }

            /// All known shells produce valid login args
            #[test]
            fn prop_login_args_are_valid(
                shell_type in prop_oneof![
                    Just(ShellType::Zsh),
                    Just(ShellType::Bash),
                    Just(ShellType::Fish),
                    Just(ShellType::Unknown),
                ]
            ) {
                let args = shell_type.login_args();
                // Login args should be non-empty for known shells
                if shell_type != ShellType::Unknown {
                    prop_assert!(!args.is_empty(), "Known shells should have login args");
                }
                // Args should not contain spaces (they're individual args)
                for arg in &args {
                    prop_assert!(!arg.contains(' '), "Args should not contain spaces: {}", arg);
                }
            }

            /// Settings override always takes precedence over environment
            #[test]
            fn prop_settings_override_precedence(
                settings_shell in prop_oneof![
                    Just("/bin/bash"),
                    Just("/bin/zsh"),
                    Just("/usr/bin/fish"),
                ],
                env_shell in prop_oneof![
                    Just("/bin/zsh"),
                    Just("/bin/bash"),
                    Just("/usr/bin/fish"),
                ]
            ) {
                let settings = TerminalSettings {
                    shell: Some(settings_shell.to_string()),
                    ..Default::default()
                };
                let result = detect_shell(Some(&settings), Some(env_shell));

                // Settings should always win
                prop_assert_eq!(
                    result.path.to_str().unwrap(),
                    settings_shell,
                    "Settings should override environment"
                );
            }

            /// Environment is used when settings.shell is None
            #[test]
            fn prop_env_used_when_no_settings_shell(
                env_shell in prop_oneof![
                    Just("/bin/zsh"),
                    Just("/bin/bash"),
                    Just("/usr/bin/fish"),
                ]
            ) {
                let settings = TerminalSettings {
                    shell: None,
                    ..Default::default()
                };
                let result = detect_shell(Some(&settings), Some(env_shell));

                prop_assert_eq!(
                    result.path.to_str().unwrap(),
                    env_shell,
                    "Environment should be used when settings.shell is None"
                );
            }

            /// Shell type detection is consistent with path suffix
            #[test]
            fn prop_shell_type_matches_basename(
                (path, expected_type) in prop_oneof![
                    (Just("/any/path/to/zsh"), Just(ShellType::Zsh)),
                    (Just("/any/path/to/bash"), Just(ShellType::Bash)),
                    (Just("/any/path/to/fish"), Just(ShellType::Fish)),
                    (Just("/any/path/to/other"), Just(ShellType::Unknown)),
                ]
            ) {
                let info = ShellInfo::new(path);
                prop_assert_eq!(info.shell_type(), expected_type);
            }
        }
    }
}
