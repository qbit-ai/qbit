# Rig Evals Implementation Notes

This directory contains notes from sub-agents working on tasks from `../rig-evals-implementation.md`.

## File Naming

- `task-X.Y.md` - Notes for Task X.Y (e.g., `task-1.2.md` for Task 1.2)
- `blockers.md` - Cross-task blockers (append-only log)

## What Goes Here

- Detailed error logs
- API design decisions that affect other tasks
- Implementation notes for future reference
- Debugging information

## What Does NOT Go Here

- Code (that goes in `backend/src/evals/`)
- General explanations (not needed)
- Duplicates of information already in the plan

## For Agents

When writing a notes file:

1. Use the exact filename `task-X.Y.md`
2. Keep it concise (under 200 lines)
3. Use headers to organize
4. Reference specific file paths

Example structure:
```markdown
# Task 1.2 Notes

## Decision: Metric Trait Design
Chose to use async trait because...

## Blocker Encountered
rig-core version mismatch, resolved by...

## Files Created
- backend/src/evals/metrics/mod.rs
- backend/src/evals/metrics/code_correctness.rs
```
