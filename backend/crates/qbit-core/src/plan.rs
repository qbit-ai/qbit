//! Core plan types for agent task tracking.
//!
//! This module contains the fundamental types for representing multi-step plans
//! in the AI agent system. These types are used across events and tools subsystems
//! to avoid circular dependencies.
//!
//! These types are intentionally kept minimal and dependency-free (aside from
//! serde and chrono) to serve as the foundation for both:
//! - Event emission (in `backend/src/ai/events.rs`)
//! - Plan management and validation (in `backend/src/tools/planner/`)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Maximum number of steps allowed in a plan.
pub const MAX_PLAN_STEPS: usize = 12;

/// Minimum number of steps required in a plan.
pub const MIN_PLAN_STEPS: usize = 1;

/// Status of a plan step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    /// Step has not been started yet.
    #[default]
    Pending,
    /// Step is currently being worked on.
    InProgress,
    /// Step has been completed.
    Completed,
}

impl std::fmt::Display for StepStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StepStatus::Pending => write!(f, "pending"),
            StepStatus::InProgress => write!(f, "in_progress"),
            StepStatus::Completed => write!(f, "completed"),
        }
    }
}

/// A single step in the plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    /// Description of what this step accomplishes.
    pub step: String,
    /// Current status of this step.
    pub status: StepStatus,
}

/// Summary statistics for a plan.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlanSummary {
    /// Total number of steps.
    pub total: usize,
    /// Number of completed steps.
    pub completed: usize,
    /// Number of in-progress steps.
    pub in_progress: usize,
    /// Number of pending steps.
    pub pending: usize,
}

impl PlanSummary {
    /// Calculate summary from a list of steps.
    pub fn from_steps(steps: &[PlanStep]) -> Self {
        let mut summary = Self {
            total: steps.len(),
            ..Default::default()
        };

        for step in steps {
            match step.status {
                StepStatus::Pending => summary.pending += 1,
                StepStatus::InProgress => summary.in_progress += 1,
                StepStatus::Completed => summary.completed += 1,
            }
        }

        summary
    }
}

/// A complete task plan with steps and metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskPlan {
    /// Optional high-level explanation of the plan.
    pub explanation: Option<String>,
    /// The steps in the plan.
    pub steps: Vec<PlanStep>,
    /// Summary statistics.
    pub summary: PlanSummary,
    /// Version number (increments on each update).
    pub version: u32,
    /// When the plan was last updated.
    pub updated_at: DateTime<Utc>,
}

impl Default for TaskPlan {
    fn default() -> Self {
        Self {
            explanation: None,
            steps: Vec::new(),
            summary: PlanSummary::default(),
            version: 0,
            updated_at: Utc::now(),
        }
    }
}

impl TaskPlan {
    /// Check if the plan is empty (no steps).
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }
}
