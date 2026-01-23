# Planning System

The planning system allows the AI agent to create, track, and display multi-step task plans. This provides visibility into the agent's progress and helps organize complex tasks.

## Overview

When the AI receives a complex task, it can create a plan with discrete steps and update progress as it works. The plan is:
- Displayed in the UI as a collapsible progress row/panel **only while the plan is active** (hidden when there is no plan, when there are no steps, or when the plan is complete)
- Updated in real-time via events
- Persisted per-session (in memory)

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         Frontend                                 │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐  │
│  │ InlineTaskPlan  │  │  useAiEvents    │  │  Zustand Store  │  │
│  │   Component     │◄─│  (event handler)│◄─│  (plan state)   │  │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘  │
│           ▲                    ▲                    ▲            │
└───────────┼────────────────────┼────────────────────┼────────────┘
            │                    │                    │
            │              ai-event              invoke
            │           (plan_updated)         (get_plan)
            │                    │                    │
┌───────────┼────────────────────┼────────────────────┼────────────┐
│           │                    │                    │            │
│  ┌────────┴────────┐  ┌────────┴────────┐  ┌───────┴─────────┐  │
│  │  update_plan    │  │  PlanUpdated    │  │  get_plan       │  │
│  │  tool executor  │─►│  event emission │  │  command        │  │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘  │
│           │                                         │            │
│           ▼                                         ▼            │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │                      PlanManager                            │ │
│  │  - Arc<RwLock<TaskPlan>> for thread-safe access             │ │
│  │  - Validation (1-12 steps, one in_progress)                 │ │
│  │  - Version tracking                                          │ │
│  └─────────────────────────────────────────────────────────────┘ │
│                           Backend                                │
└─────────────────────────────────────────────────────────────────┘
```

## Data Model

### TaskPlan

```typescript
interface TaskPlan {
  explanation?: string;      // Optional high-level summary
  steps: PlanStep[];         // List of plan steps (1-12)
  summary: PlanSummary;      // Computed statistics
  version: number;           // Increments on each update
  updated_at: string;        // ISO timestamp
}
```

### PlanStep

```typescript
interface PlanStep {
  step: string;              // Step description
  status: StepStatus;        // Current status
}

type StepStatus = "pending" | "in_progress" | "completed";
```

### PlanSummary

```typescript
interface PlanSummary {
  total: number;             // Total step count
  completed: number;         // Completed steps
  in_progress: number;       // In-progress steps (0 or 1)
  pending: number;           // Pending steps
}
```

## Backend

### Tool: update_plan

The AI uses this tool to create or update its task plan.

**Arguments:**
```json
{
  "explanation": "Optional plan summary",
  "plan": [
    { "step": "Step description", "status": "pending" }
  ]
}
```

**Constraints:**
- 1-12 steps allowed
- Only ONE step can be `in_progress` at a time
- Step descriptions cannot be empty (whitespace is trimmed)

**Response:**
```json
{
  "success": true,
  "version": 1,
  "summary": {
    "total": 3,
    "completed": 1,
    "in_progress": 1,
    "pending": 1
  }
}
```

### Command: get_plan

Query the current plan state for a session.

```typescript
const plan = await invoke('get_plan', { sessionId: 'session-id' });
```

### Event: plan_updated

Emitted when the plan changes.

```typescript
{
  type: "plan_updated",
  version: 1,
  summary: { total: 3, completed: 1, in_progress: 1, pending: 1 },
  steps: [
    { step: "Analyze code", status: "completed" },
    { step: "Implement changes", status: "in_progress" },
    { step: "Run tests", status: "pending" }
  ],
  explanation: "Implementation plan for feature X"
}
```

### Files

| File | Purpose |
|------|---------|
| `backend/src/tools/planner/mod.rs` | `PlanManager`, `TaskPlan`, validation logic |
| `backend/src/tools/definitions.rs` | Tool JSON schema |
| `backend/src/ai/tool_executors.rs` | `execute_plan_tool` function |
| `backend/src/ai/commands/plan.rs` | `get_plan` Tauri command |
| `backend/src/ai/events.rs` | `PlanUpdated` event definition |
| `backend/src/ai/system_prompt.rs` | AI instructions for using plans |

## Frontend

### Store Integration

The plan is stored per-session in the Zustand store:

```typescript
// Get plan for current session
const plan = useStore((state) => state.sessions[sessionId]?.plan);

// Update plan (called by event handler)
const setPlan = useStore((state) => state.setPlan);
setPlan(sessionId, newPlan);
```

### Event Handling

The `useAiEvents` hook handles `plan_updated` events:

```typescript
case "plan_updated": {
  const { version, summary, steps, explanation } = event;
  setPlan(sessionId, {
    version,
    summary,
    steps,
    explanation,
    updated_at: new Date().toISOString(),
  });
  break;
}
```

### Plan UI Component

The task plan is rendered as a collapsible row. It is only shown when a plan exists, has at least one step, and is not complete (`summary.total > 0 && summary.completed === summary.total`).

A collapsible component that displays:

1. **Header** (always visible)
   - "Task Plan" title with step count (e.g., "3/7 steps")
   - Progress percentage
   - Progress bar
   - Collapse/expand chevron

2. **Content** (collapsible)
   - Optional explanation text
   - Current step highlight (the in-progress step)
   - Full step list with status indicators

**Status Indicators:**
- ✓ Completed: Green checkmark, strikethrough text
- → In Progress: Blue spinner (animated), highlighted background
- ○ Pending: Gray circle, muted text

### Files

| File | Purpose |
|------|---------|
| `frontend/store/index.ts` | Types and state management |
| `frontend/hooks/useAiEvents.ts` | Event handler for plan_updated |
| `frontend/lib/ai.ts` | `getPlan()` command wrapper |
| `frontend/components/InlineTaskPlan/InlineTaskPlan.tsx` | UI component (task plan row shown above the input) |
| `frontend/components/UnifiedInput/UnifiedInput.tsx` | Integration point |
| `frontend/components/PlanProgress/PlanProgress.tsx` | Alternate UI component (not currently wired) |

## AI Behavior

The system prompt instructs the AI on when and how to use plans:

### When to Use
- Complex tasks with 3+ steps
- Multi-file changes
- Tasks requiring progress tracking
- Sequential operations

### When NOT to Use
- Single-step tasks
- Trivial operations
- Quick lookups

### Best Practices
- Create plans proactively for non-trivial tasks
- Use clear, actionable step descriptions
- Mark steps `in_progress` when starting work
- Mark steps `completed` immediately after finishing
- Update plans as scope changes
- Include verification steps (tests, validation)

## Example Flow

1. **User**: "Add a new API endpoint for user preferences"

2. **AI creates plan**:
   ```json
   {
     "explanation": "Add user preferences endpoint",
     "plan": [
       { "step": "Create preferences model", "status": "in_progress" },
       { "step": "Add database migration", "status": "pending" },
       { "step": "Implement GET /preferences endpoint", "status": "pending" },
       { "step": "Implement PUT /preferences endpoint", "status": "pending" },
       { "step": "Add input validation", "status": "pending" },
       { "step": "Write tests", "status": "pending" }
     ]
   }
   ```

3. **UI displays** collapsible plan with progress bar (0/6 steps)

4. **AI completes step 1, updates plan**:
   ```json
   {
     "plan": [
       { "step": "Create preferences model", "status": "completed" },
       { "step": "Add database migration", "status": "in_progress" },
       ...
     ]
   }
   ```

5. **UI updates** progress bar (1/6 steps, 17%)

6. Process continues until all steps are completed
7. Once complete, the task plan row is hidden

## Testing

The planner module includes 44 tests:

```bash
cargo test --lib tools::planner
```

### Unit Tests
- Status serialization/deserialization
- Summary calculation
- Plan validation (step count, empty descriptions, multiple in_progress)
- Version incrementing
- Whitespace trimming
- Clear functionality

### Property-Based Tests
- Summary counts sum to total
- Valid plans always succeed
- Invalid plans always fail appropriately
- Serialization round-trips correctly
- Whitespace is consistently trimmed
