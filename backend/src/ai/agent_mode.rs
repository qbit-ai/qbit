//! Agent mode definitions for controlling tool approval behavior.
//!
//! This module defines the different modes that control how the AI agent
//! handles tool approvals:
//!
//! - `Default`: Normal HITL (Human-in-the-Loop) behavior based on tool policy
//! - `AutoApprove`: All tool calls are automatically approved without prompting
//! - `Planning`: Only read-only tools are allowed; write operations are denied

use serde::{Deserialize, Serialize};

/// Agent mode determines how tool approvals are handled.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgentMode {
    /// Default mode: Tool approval required based on policy (normal HITL).
    /// Uses the configured tool policy to determine which tools need approval.
    #[default]
    Default,

    /// Auto-approve mode: All tool calls are automatically approved.
    /// Useful for unattended operations or when user trusts all tool calls.
    AutoApprove,

    /// Planning mode: Only read-only tools are allowed.
    /// Write operations (edit_file, write_file, etc.) are denied.
    /// Useful for exploration and planning without making changes.
    Planning,
}

impl AgentMode {
    /// Returns true if this is the default mode.
    pub fn is_default(&self) -> bool {
        matches!(self, AgentMode::Default)
    }

    /// Returns true if this mode auto-approves all tools.
    pub fn is_auto_approve(&self) -> bool {
        matches!(self, AgentMode::AutoApprove)
    }

    /// Returns true if this is planning mode (read-only).
    pub fn is_planning(&self) -> bool {
        matches!(self, AgentMode::Planning)
    }
}

impl std::fmt::Display for AgentMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentMode::Default => write!(f, "default"),
            AgentMode::AutoApprove => write!(f, "auto-approve"),
            AgentMode::Planning => write!(f, "planning"),
        }
    }
}

impl std::str::FromStr for AgentMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "default" => Ok(AgentMode::Default),
            "auto-approve" => Ok(AgentMode::AutoApprove),
            "planning" => Ok(AgentMode::Planning),
            _ => Err(format!("Invalid agent mode: {}", s)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_mode_default() {
        assert_eq!(AgentMode::default(), AgentMode::Default);
    }

    #[test]
    fn test_agent_mode_display() {
        assert_eq!(format!("{}", AgentMode::Default), "default");
        assert_eq!(format!("{}", AgentMode::AutoApprove), "auto-approve");
        assert_eq!(format!("{}", AgentMode::Planning), "planning");
    }

    #[test]
    fn test_agent_mode_from_str() {
        assert_eq!("default".parse::<AgentMode>().unwrap(), AgentMode::Default);
        assert_eq!(
            "auto-approve".parse::<AgentMode>().unwrap(),
            AgentMode::AutoApprove
        );
        assert_eq!(
            "planning".parse::<AgentMode>().unwrap(),
            AgentMode::Planning
        );
        assert!("invalid".parse::<AgentMode>().is_err());
    }

    #[test]
    fn test_agent_mode_serde() {
        let mode = AgentMode::AutoApprove;
        let json = serde_json::to_string(&mode).unwrap();
        assert_eq!(json, "\"auto-approve\"");

        let parsed: AgentMode = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, AgentMode::AutoApprove);
    }

    #[test]
    fn test_agent_mode_checks() {
        assert!(AgentMode::Default.is_default());
        assert!(!AgentMode::Default.is_auto_approve());
        assert!(!AgentMode::Default.is_planning());

        assert!(!AgentMode::AutoApprove.is_default());
        assert!(AgentMode::AutoApprove.is_auto_approve());
        assert!(!AgentMode::AutoApprove.is_planning());

        assert!(!AgentMode::Planning.is_default());
        assert!(!AgentMode::Planning.is_auto_approve());
        assert!(AgentMode::Planning.is_planning());
    }
}
