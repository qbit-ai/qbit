//! Plan management for agent task tracking.
//!
//! This module provides a simple planning system that allows the AI agent
//! to create and update multi-step plans. Based on vtcode-core's implementation.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

/// Maximum number of steps allowed in a plan.
const MAX_PLAN_STEPS: usize = 12;

/// Minimum number of steps required in a plan.
const MIN_PLAN_STEPS: usize = 1;

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

/// Arguments for the update_plan tool.
#[derive(Debug, Clone, Deserialize)]
pub struct UpdatePlanArgs {
    /// Optional explanation/summary of the plan.
    pub explanation: Option<String>,
    /// The plan steps.
    pub plan: Vec<PlanStepInput>,
}

/// Input format for a plan step (from tool arguments).
#[derive(Debug, Clone, Deserialize)]
pub struct PlanStepInput {
    /// Description of the step.
    pub step: String,
    /// Status of the step.
    #[serde(default)]
    pub status: StepStatus,
}

/// Error type for plan validation.
#[derive(Debug, thiserror::Error)]
pub enum PlanError {
    #[error("Plan must have between {MIN_PLAN_STEPS} and {MAX_PLAN_STEPS} steps, got {0}")]
    InvalidStepCount(usize),

    #[error("Step {0} has empty description")]
    EmptyStepDescription(usize),

    #[error("Only one step can be in_progress at a time, found {0}")]
    MultipleInProgress(usize),
}

/// Manager for task plans.
///
/// Provides thread-safe access to the current plan with validation.
pub struct PlanManager {
    plan: Arc<RwLock<TaskPlan>>,
}

impl Default for PlanManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PlanManager {
    /// Create a new PlanManager with an empty plan.
    pub fn new() -> Self {
        Self {
            plan: Arc::new(RwLock::new(TaskPlan::default())),
        }
    }

    /// Get a snapshot of the current plan.
    pub async fn snapshot(&self) -> TaskPlan {
        self.plan.read().await.clone()
    }

    /// Check if the plan is empty.
    pub async fn is_empty(&self) -> bool {
        self.plan.read().await.is_empty()
    }

    /// Update the plan with new steps.
    ///
    /// Validates the input and updates the plan atomically.
    pub async fn update_plan(&self, args: UpdatePlanArgs) -> Result<TaskPlan, PlanError> {
        // Validate step count
        let step_count = args.plan.len();
        if !(MIN_PLAN_STEPS..=MAX_PLAN_STEPS).contains(&step_count) {
            return Err(PlanError::InvalidStepCount(step_count));
        }

        // Validate steps and count in_progress
        let mut in_progress_count = 0;
        for (i, step) in args.plan.iter().enumerate() {
            // Check for empty descriptions
            let trimmed = step.step.trim();
            if trimmed.is_empty() {
                return Err(PlanError::EmptyStepDescription(i + 1));
            }

            // Count in_progress steps
            if step.status == StepStatus::InProgress {
                in_progress_count += 1;
            }
        }

        // Ensure at most one in_progress
        if in_progress_count > 1 {
            return Err(PlanError::MultipleInProgress(in_progress_count));
        }

        // Convert input to plan steps
        let steps: Vec<PlanStep> = args
            .plan
            .into_iter()
            .map(|input| PlanStep {
                step: input.step.trim().to_string(),
                status: input.status,
            })
            .collect();

        // Calculate summary
        let summary = PlanSummary::from_steps(&steps);

        // Update the plan
        let mut plan = self.plan.write().await;
        plan.explanation = args.explanation.map(|s| s.trim().to_string());
        plan.steps = steps;
        plan.summary = summary;
        plan.version += 1;
        plan.updated_at = Utc::now();

        tracing::info!(
            version = plan.version,
            total = plan.summary.total,
            completed = plan.summary.completed,
            "Plan updated"
        );

        Ok(plan.clone())
    }

    /// Clear the plan.
    pub async fn clear(&self) {
        let mut plan = self.plan.write().await;
        *plan = TaskPlan::default();
        tracing::info!("Plan cleared");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // StepStatus Tests
    // ========================================================================

    #[test]
    fn test_step_status_default() {
        let status = StepStatus::default();
        assert_eq!(status, StepStatus::Pending);
    }

    #[test]
    fn test_step_status_display() {
        assert_eq!(format!("{}", StepStatus::Pending), "pending");
        assert_eq!(format!("{}", StepStatus::InProgress), "in_progress");
        assert_eq!(format!("{}", StepStatus::Completed), "completed");
    }

    #[test]
    fn test_step_status_serialization() {
        assert_eq!(
            serde_json::to_string(&StepStatus::Pending).unwrap(),
            "\"pending\""
        );
        assert_eq!(
            serde_json::to_string(&StepStatus::InProgress).unwrap(),
            "\"in_progress\""
        );
        assert_eq!(
            serde_json::to_string(&StepStatus::Completed).unwrap(),
            "\"completed\""
        );
    }

    #[test]
    fn test_step_status_deserialization() {
        assert_eq!(
            serde_json::from_str::<StepStatus>("\"pending\"").unwrap(),
            StepStatus::Pending
        );
        assert_eq!(
            serde_json::from_str::<StepStatus>("\"in_progress\"").unwrap(),
            StepStatus::InProgress
        );
        assert_eq!(
            serde_json::from_str::<StepStatus>("\"completed\"").unwrap(),
            StepStatus::Completed
        );
    }

    // ========================================================================
    // PlanSummary Tests
    // ========================================================================

    #[test]
    fn test_plan_summary_default() {
        let summary = PlanSummary::default();
        assert_eq!(summary.total, 0);
        assert_eq!(summary.completed, 0);
        assert_eq!(summary.in_progress, 0);
        assert_eq!(summary.pending, 0);
    }

    #[test]
    fn test_plan_summary_from_empty_steps() {
        let summary = PlanSummary::from_steps(&[]);
        assert_eq!(summary.total, 0);
        assert_eq!(summary.completed, 0);
        assert_eq!(summary.in_progress, 0);
        assert_eq!(summary.pending, 0);
    }

    #[test]
    fn test_plan_summary_from_mixed_steps() {
        let steps = vec![
            PlanStep {
                step: "Step 1".to_string(),
                status: StepStatus::Completed,
            },
            PlanStep {
                step: "Step 2".to_string(),
                status: StepStatus::Completed,
            },
            PlanStep {
                step: "Step 3".to_string(),
                status: StepStatus::InProgress,
            },
            PlanStep {
                step: "Step 4".to_string(),
                status: StepStatus::Pending,
            },
            PlanStep {
                step: "Step 5".to_string(),
                status: StepStatus::Pending,
            },
        ];

        let summary = PlanSummary::from_steps(&steps);
        assert_eq!(summary.total, 5);
        assert_eq!(summary.completed, 2);
        assert_eq!(summary.in_progress, 1);
        assert_eq!(summary.pending, 2);
    }

    #[test]
    fn test_plan_summary_all_completed() {
        let steps = vec![
            PlanStep {
                step: "Done 1".to_string(),
                status: StepStatus::Completed,
            },
            PlanStep {
                step: "Done 2".to_string(),
                status: StepStatus::Completed,
            },
        ];

        let summary = PlanSummary::from_steps(&steps);
        assert_eq!(summary.total, 2);
        assert_eq!(summary.completed, 2);
        assert_eq!(summary.in_progress, 0);
        assert_eq!(summary.pending, 0);
    }

    // ========================================================================
    // TaskPlan Tests
    // ========================================================================

    #[test]
    fn test_task_plan_default() {
        let plan = TaskPlan::default();
        assert!(plan.explanation.is_none());
        assert!(plan.steps.is_empty());
        assert_eq!(plan.version, 0);
        assert!(plan.is_empty());
    }

    #[test]
    fn test_task_plan_is_empty() {
        let mut plan = TaskPlan::default();
        assert!(plan.is_empty());

        plan.steps.push(PlanStep {
            step: "Test".to_string(),
            status: StepStatus::Pending,
        });
        assert!(!plan.is_empty());
    }

    // ========================================================================
    // PlanStep Serialization Tests
    // ========================================================================

    #[test]
    fn test_plan_step_serialization() {
        let step = PlanStep {
            step: "Read the file".to_string(),
            status: StepStatus::InProgress,
        };

        let json = serde_json::to_string(&step).unwrap();
        assert!(json.contains("\"step\":\"Read the file\""));
        assert!(json.contains("\"status\":\"in_progress\""));
    }

    #[test]
    fn test_plan_step_input_deserialization_with_status() {
        let json = r#"{"step": "Do something", "status": "completed"}"#;
        let input: PlanStepInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.step, "Do something");
        assert_eq!(input.status, StepStatus::Completed);
    }

    #[test]
    fn test_plan_step_input_deserialization_without_status() {
        let json = r#"{"step": "Do something"}"#;
        let input: PlanStepInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.step, "Do something");
        assert_eq!(input.status, StepStatus::Pending); // Default
    }

    // ========================================================================
    // UpdatePlanArgs Deserialization Tests
    // ========================================================================

    #[test]
    fn test_update_plan_args_full() {
        let json = r#"{
            "explanation": "My plan",
            "plan": [
                {"step": "Step 1", "status": "completed"},
                {"step": "Step 2", "status": "in_progress"},
                {"step": "Step 3"}
            ]
        }"#;

        let args: UpdatePlanArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.explanation, Some("My plan".to_string()));
        assert_eq!(args.plan.len(), 3);
        assert_eq!(args.plan[0].status, StepStatus::Completed);
        assert_eq!(args.plan[1].status, StepStatus::InProgress);
        assert_eq!(args.plan[2].status, StepStatus::Pending);
    }

    #[test]
    fn test_update_plan_args_minimal() {
        let json = r#"{"plan": [{"step": "Only step"}]}"#;

        let args: UpdatePlanArgs = serde_json::from_str(json).unwrap();
        assert!(args.explanation.is_none());
        assert_eq!(args.plan.len(), 1);
    }

    // ========================================================================
    // PlanError Tests
    // ========================================================================

    #[test]
    fn test_plan_error_display() {
        let err = PlanError::InvalidStepCount(15);
        assert!(err.to_string().contains("15"));
        assert!(err.to_string().contains("1"));
        assert!(err.to_string().contains("12"));

        let err = PlanError::EmptyStepDescription(3);
        assert!(err.to_string().contains("Step 3"));
        assert!(err.to_string().contains("empty"));

        let err = PlanError::MultipleInProgress(2);
        assert!(err.to_string().contains("2"));
        assert!(err.to_string().contains("one"));
    }

    // ========================================================================
    // PlanManager Unit Tests
    // ========================================================================

    #[tokio::test]
    async fn test_plan_manager_new_is_empty() {
        let manager = PlanManager::new();
        assert!(manager.is_empty().await);
    }

    #[tokio::test]
    async fn test_plan_manager_default_is_empty() {
        let manager = PlanManager::default();
        assert!(manager.is_empty().await);
    }

    #[tokio::test]
    async fn test_plan_manager_update() {
        let manager = PlanManager::new();

        let args = UpdatePlanArgs {
            explanation: Some("Test plan".to_string()),
            plan: vec![
                PlanStepInput {
                    step: "Step 1".to_string(),
                    status: StepStatus::Completed,
                },
                PlanStepInput {
                    step: "Step 2".to_string(),
                    status: StepStatus::InProgress,
                },
                PlanStepInput {
                    step: "Step 3".to_string(),
                    status: StepStatus::Pending,
                },
            ],
        };

        let plan = manager.update_plan(args).await.unwrap();

        assert_eq!(plan.version, 1);
        assert_eq!(plan.steps.len(), 3);
        assert_eq!(plan.summary.completed, 1);
        assert_eq!(plan.summary.in_progress, 1);
        assert_eq!(plan.summary.pending, 1);
        assert_eq!(plan.explanation, Some("Test plan".to_string()));
    }

    #[tokio::test]
    async fn test_plan_manager_version_increments() {
        let manager = PlanManager::new();

        for i in 1..=5 {
            let args = UpdatePlanArgs {
                explanation: None,
                plan: vec![PlanStepInput {
                    step: format!("Step version {}", i),
                    status: StepStatus::Pending,
                }],
            };

            let plan = manager.update_plan(args).await.unwrap();
            assert_eq!(plan.version, i);
        }
    }

    #[tokio::test]
    async fn test_plan_manager_snapshot() {
        let manager = PlanManager::new();

        let args = UpdatePlanArgs {
            explanation: Some("Snapshot test".to_string()),
            plan: vec![PlanStepInput {
                step: "Test step".to_string(),
                status: StepStatus::Pending,
            }],
        };

        manager.update_plan(args).await.unwrap();

        let snapshot = manager.snapshot().await;
        assert_eq!(snapshot.explanation, Some("Snapshot test".to_string()));
        assert_eq!(snapshot.steps.len(), 1);
        assert_eq!(snapshot.version, 1);
    }

    #[tokio::test]
    async fn test_plan_manager_clear() {
        let manager = PlanManager::new();

        let args = UpdatePlanArgs {
            explanation: Some("Will be cleared".to_string()),
            plan: vec![PlanStepInput {
                step: "Step".to_string(),
                status: StepStatus::InProgress,
            }],
        };

        manager.update_plan(args).await.unwrap();
        assert!(!manager.is_empty().await);

        manager.clear().await;
        assert!(manager.is_empty().await);

        let snapshot = manager.snapshot().await;
        assert!(snapshot.explanation.is_none());
        assert!(snapshot.steps.is_empty());
        // Version is reset on clear
        assert_eq!(snapshot.version, 0);
    }

    #[tokio::test]
    async fn test_plan_manager_trims_whitespace() {
        let manager = PlanManager::new();

        let args = UpdatePlanArgs {
            explanation: Some("  Trimmed explanation  ".to_string()),
            plan: vec![PlanStepInput {
                step: "  Trimmed step  ".to_string(),
                status: StepStatus::Pending,
            }],
        };

        let plan = manager.update_plan(args).await.unwrap();
        assert_eq!(plan.explanation, Some("Trimmed explanation".to_string()));
        assert_eq!(plan.steps[0].step, "Trimmed step");
    }

    #[tokio::test]
    async fn test_plan_manager_rejects_empty_steps() {
        let manager = PlanManager::new();

        let args = UpdatePlanArgs {
            explanation: None,
            plan: vec![PlanStepInput {
                step: "  ".to_string(), // Empty after trim
                status: StepStatus::Pending,
            }],
        };

        let result = manager.update_plan(args).await;
        assert!(matches!(result, Err(PlanError::EmptyStepDescription(1))));
    }

    #[tokio::test]
    async fn test_plan_manager_rejects_multiple_in_progress() {
        let manager = PlanManager::new();

        let args = UpdatePlanArgs {
            explanation: None,
            plan: vec![
                PlanStepInput {
                    step: "Step 1".to_string(),
                    status: StepStatus::InProgress,
                },
                PlanStepInput {
                    step: "Step 2".to_string(),
                    status: StepStatus::InProgress,
                },
            ],
        };

        let result = manager.update_plan(args).await;
        assert!(matches!(result, Err(PlanError::MultipleInProgress(2))));
    }

    #[tokio::test]
    async fn test_plan_manager_allows_zero_in_progress() {
        let manager = PlanManager::new();

        let args = UpdatePlanArgs {
            explanation: None,
            plan: vec![
                PlanStepInput {
                    step: "Step 1".to_string(),
                    status: StepStatus::Completed,
                },
                PlanStepInput {
                    step: "Step 2".to_string(),
                    status: StepStatus::Pending,
                },
            ],
        };

        let result = manager.update_plan(args).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_plan_manager_allows_one_in_progress() {
        let manager = PlanManager::new();

        let args = UpdatePlanArgs {
            explanation: None,
            plan: vec![
                PlanStepInput {
                    step: "Step 1".to_string(),
                    status: StepStatus::InProgress,
                },
                PlanStepInput {
                    step: "Step 2".to_string(),
                    status: StepStatus::Pending,
                },
            ],
        };

        let result = manager.update_plan(args).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().summary.in_progress, 1);
    }

    #[tokio::test]
    async fn test_plan_manager_rejects_too_many_steps() {
        let manager = PlanManager::new();

        let steps: Vec<PlanStepInput> = (0..15)
            .map(|i| PlanStepInput {
                step: format!("Step {}", i),
                status: StepStatus::Pending,
            })
            .collect();

        let args = UpdatePlanArgs {
            explanation: None,
            plan: steps,
        };

        let result = manager.update_plan(args).await;
        assert!(matches!(result, Err(PlanError::InvalidStepCount(15))));
    }

    #[tokio::test]
    async fn test_plan_manager_rejects_zero_steps() {
        let manager = PlanManager::new();

        let args = UpdatePlanArgs {
            explanation: Some("Empty plan".to_string()),
            plan: vec![],
        };

        let result = manager.update_plan(args).await;
        assert!(matches!(result, Err(PlanError::InvalidStepCount(0))));
    }

    #[tokio::test]
    async fn test_plan_manager_accepts_boundary_step_counts() {
        let manager = PlanManager::new();

        // Test minimum (1 step)
        let args = UpdatePlanArgs {
            explanation: None,
            plan: vec![PlanStepInput {
                step: "Single step".to_string(),
                status: StepStatus::Pending,
            }],
        };
        assert!(manager.update_plan(args).await.is_ok());

        // Test maximum (12 steps)
        let steps: Vec<PlanStepInput> = (0..12)
            .map(|i| PlanStepInput {
                step: format!("Step {}", i + 1),
                status: StepStatus::Pending,
            })
            .collect();

        let args = UpdatePlanArgs {
            explanation: None,
            plan: steps,
        };
        let result = manager.update_plan(args).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().steps.len(), 12);
    }

    #[tokio::test]
    async fn test_plan_manager_rejects_just_over_max() {
        let manager = PlanManager::new();

        // Test 13 steps (just over max)
        let steps: Vec<PlanStepInput> = (0..13)
            .map(|i| PlanStepInput {
                step: format!("Step {}", i + 1),
                status: StepStatus::Pending,
            })
            .collect();

        let args = UpdatePlanArgs {
            explanation: None,
            plan: steps,
        };

        let result = manager.update_plan(args).await;
        assert!(matches!(result, Err(PlanError::InvalidStepCount(13))));
    }

    #[tokio::test]
    async fn test_plan_manager_empty_description_at_various_positions() {
        let manager = PlanManager::new();

        // Empty at position 1
        let args = UpdatePlanArgs {
            explanation: None,
            plan: vec![
                PlanStepInput {
                    step: "".to_string(),
                    status: StepStatus::Pending,
                },
                PlanStepInput {
                    step: "Valid".to_string(),
                    status: StepStatus::Pending,
                },
            ],
        };
        let result = manager.update_plan(args).await;
        assert!(matches!(result, Err(PlanError::EmptyStepDescription(1))));

        // Empty at position 2
        let args = UpdatePlanArgs {
            explanation: None,
            plan: vec![
                PlanStepInput {
                    step: "Valid".to_string(),
                    status: StepStatus::Pending,
                },
                PlanStepInput {
                    step: "\t\n".to_string(), // Whitespace only
                    status: StepStatus::Pending,
                },
            ],
        };
        let result = manager.update_plan(args).await;
        assert!(matches!(result, Err(PlanError::EmptyStepDescription(2))));
    }

    // ========================================================================
    // Property-Based Tests
    // ========================================================================

    mod property_tests {
        use super::*;
        use proptest::prelude::*;

        /// Strategy for generating a valid step status
        fn status_strategy() -> impl Strategy<Value = StepStatus> {
            prop_oneof![
                Just(StepStatus::Pending),
                Just(StepStatus::InProgress),
                Just(StepStatus::Completed),
            ]
        }

        /// Strategy for generating a non-empty step description
        fn step_description_strategy() -> impl Strategy<Value = String> {
            "[a-zA-Z0-9 ]{1,50}"
                .prop_filter("must not be empty after trim", |s| !s.trim().is_empty())
        }

        /// Strategy for generating a valid plan step input
        fn plan_step_input_strategy() -> impl Strategy<Value = PlanStepInput> {
            (step_description_strategy(), status_strategy())
                .prop_map(|(step, status)| PlanStepInput { step, status })
        }

        /// Strategy for generating a plan with valid step count (1-12)
        fn valid_plan_strategy() -> impl Strategy<Value = Vec<PlanStepInput>> {
            prop::collection::vec(plan_step_input_strategy(), 1..=12)
        }

        proptest! {
            /// Property: Summary counts always sum to total
            #[test]
            fn summary_counts_sum_to_total(steps in valid_plan_strategy()) {
                let plan_steps: Vec<PlanStep> = steps
                    .into_iter()
                    .map(|input| PlanStep {
                        step: input.step,
                        status: input.status,
                    })
                    .collect();

                let summary = PlanSummary::from_steps(&plan_steps);

                prop_assert_eq!(
                    summary.completed + summary.in_progress + summary.pending,
                    summary.total,
                    "Summary counts don't sum to total"
                );
            }

            /// Property: Summary total equals step count
            #[test]
            fn summary_total_equals_step_count(steps in valid_plan_strategy()) {
                let plan_steps: Vec<PlanStep> = steps
                    .into_iter()
                    .map(|input| PlanStep {
                        step: input.step,
                        status: input.status,
                    })
                    .collect();

                let summary = PlanSummary::from_steps(&plan_steps);

                prop_assert_eq!(
                    summary.total,
                    plan_steps.len(),
                    "Summary total doesn't equal step count"
                );
            }

            /// Property: Step status serialization round-trips correctly
            #[test]
            fn step_status_serialization_roundtrip(status in status_strategy()) {
                let json = serde_json::to_string(&status).unwrap();
                let parsed: StepStatus = serde_json::from_str(&json).unwrap();
                prop_assert_eq!(status, parsed);
            }

            /// Property: PlanStep serialization round-trips correctly
            #[test]
            fn plan_step_serialization_roundtrip(
                description in step_description_strategy(),
                status in status_strategy()
            ) {
                let step = PlanStep {
                    step: description,
                    status,
                };

                let json = serde_json::to_string(&step).unwrap();
                let parsed: PlanStep = serde_json::from_str(&json).unwrap();

                prop_assert_eq!(step.step, parsed.step);
                prop_assert_eq!(step.status, parsed.status);
            }

            /// Property: Valid plans always succeed
            #[test]
            fn valid_plans_succeed(
                steps in prop::collection::vec(plan_step_input_strategy(), 1..=12)
                    .prop_filter("at most one in_progress", |steps| {
                        steps.iter().filter(|s| s.status == StepStatus::InProgress).count() <= 1
                    })
            ) {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    let manager = PlanManager::new();
                    let args = UpdatePlanArgs {
                        explanation: None,
                        plan: steps,
                    };

                    let result = manager.update_plan(args).await;
                    prop_assert!(result.is_ok(), "Valid plan should succeed: {:?}", result);
                    Ok(())
                })?;
            }

            /// Property: Invalid step counts always fail
            #[test]
            fn invalid_step_count_fails(count in (13usize..100)) {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    let manager = PlanManager::new();
                    let steps: Vec<PlanStepInput> = (0..count)
                        .map(|i| PlanStepInput {
                            step: format!("Step {}", i),
                            status: StepStatus::Pending,
                        })
                        .collect();

                    let args = UpdatePlanArgs {
                        explanation: None,
                        plan: steps,
                    };

                    let result = manager.update_plan(args).await;
                    prop_assert!(matches!(result, Err(PlanError::InvalidStepCount(_))));
                    Ok(())
                })?;
            }

            /// Property: Multiple in_progress always fails
            #[test]
            fn multiple_in_progress_fails(extra_in_progress in 2usize..5) {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    let manager = PlanManager::new();
                    let steps: Vec<PlanStepInput> = (0..extra_in_progress)
                        .map(|i| PlanStepInput {
                            step: format!("In progress step {}", i),
                            status: StepStatus::InProgress,
                        })
                        .collect();

                    let args = UpdatePlanArgs {
                        explanation: None,
                        plan: steps,
                    };

                    let result = manager.update_plan(args).await;
                    prop_assert!(matches!(result, Err(PlanError::MultipleInProgress(_))));
                    Ok(())
                })?;
            }

            /// Property: Version always increments on successful update
            #[test]
            fn version_increments(num_updates in 1usize..10) {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    let manager = PlanManager::new();

                    for expected_version in 1..=num_updates {
                        let args = UpdatePlanArgs {
                            explanation: None,
                            plan: vec![PlanStepInput {
                                step: format!("Step for update {}", expected_version),
                                status: StepStatus::Pending,
                            }],
                        };

                        let plan = manager.update_plan(args).await.unwrap();
                        prop_assert_eq!(
                            plan.version as usize,
                            expected_version,
                            "Version should be {} but was {}",
                            expected_version,
                            plan.version
                        );
                    }
                    Ok(())
                })?;
            }

            /// Property: Whitespace is trimmed from step descriptions
            #[test]
            fn whitespace_is_trimmed(
                prefix_spaces in "[ \\t]{0,5}",
                content in "[a-zA-Z0-9]{1,20}",
                suffix_spaces in "[ \\t]{0,5}"
            ) {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    let manager = PlanManager::new();
                    let step_with_whitespace = format!("{}{}{}", prefix_spaces, content, suffix_spaces);

                    let args = UpdatePlanArgs {
                        explanation: None,
                        plan: vec![PlanStepInput {
                            step: step_with_whitespace,
                            status: StepStatus::Pending,
                        }],
                    };

                    let plan = manager.update_plan(args).await.unwrap();
                    prop_assert_eq!(
                        &plan.steps[0].step,
                        &content,
                        "Step description should be trimmed"
                    );
                    Ok(())
                })?;
            }

            /// Property: Explanation whitespace is trimmed
            #[test]
            fn explanation_whitespace_is_trimmed(
                prefix_spaces in "[ \\t]{0,5}",
                content in "[a-zA-Z0-9]{1,20}",
                suffix_spaces in "[ \\t]{0,5}"
            ) {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    let manager = PlanManager::new();
                    let explanation_with_whitespace = format!("{}{}{}", prefix_spaces, content, suffix_spaces);

                    let args = UpdatePlanArgs {
                        explanation: Some(explanation_with_whitespace),
                        plan: vec![PlanStepInput {
                            step: "Test step".to_string(),
                            status: StepStatus::Pending,
                        }],
                    };

                    let plan = manager.update_plan(args).await.unwrap();
                    prop_assert_eq!(
                        plan.explanation,
                        Some(content),
                        "Explanation should be trimmed"
                    );
                    Ok(())
                })?;
            }

            /// Property: Clear resets plan to default state
            #[test]
            fn clear_resets_to_default(steps in valid_plan_strategy()) {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    let manager = PlanManager::new();

                    // First update with some data
                    let in_progress_count = steps.iter().filter(|s| s.status == StepStatus::InProgress).count();
                    if in_progress_count <= 1 {
                        let args = UpdatePlanArgs {
                            explanation: Some("Will be cleared".to_string()),
                            plan: steps,
                        };
                        let _ = manager.update_plan(args).await;
                    }

                    // Clear
                    manager.clear().await;

                    // Verify default state
                    let snapshot = manager.snapshot().await;
                    prop_assert!(snapshot.is_empty());
                    prop_assert!(snapshot.explanation.is_none());
                    prop_assert_eq!(snapshot.version, 0);
                    Ok(())
                })?;
            }

            /// Property: Snapshot returns consistent data
            #[test]
            fn snapshot_is_consistent(
                steps in prop::collection::vec(plan_step_input_strategy(), 1..=12)
                    .prop_filter("at most one in_progress", |steps| {
                        steps.iter().filter(|s| s.status == StepStatus::InProgress).count() <= 1
                    }),
                explanation in prop::option::of("[a-zA-Z0-9 ]{1,30}")
            ) {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    let manager = PlanManager::new();
                    let step_count = steps.len();

                    let args = UpdatePlanArgs {
                        explanation: explanation.clone(),
                        plan: steps,
                    };

                    manager.update_plan(args).await.unwrap();

                    let snapshot1 = manager.snapshot().await;
                    let snapshot2 = manager.snapshot().await;

                    // Snapshots should be equal
                    prop_assert_eq!(snapshot1.steps.len(), snapshot2.steps.len());
                    prop_assert_eq!(snapshot1.version, snapshot2.version);
                    prop_assert_eq!(snapshot1.explanation, snapshot2.explanation);
                    prop_assert_eq!(snapshot1.steps.len(), step_count);

                    Ok(())
                })?;
            }
        }
    }
}
