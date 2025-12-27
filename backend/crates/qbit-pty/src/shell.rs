//! Shell detection and configuration for multi-shell support.
//!
//! This module provides shell type detection from paths and settings,
//! supporting zsh, bash, and fish shells.

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
