# SWE-bench Evaluation Documentation

This documentation provides complete transparency into how Qbit runs the SWE-bench Lite benchmark. We've designed our evaluation to be:

- **Reproducible** - Anyone can verify results using the same methodology
- **Fair** - The agent operates under the same constraints as human developers
- **Auditable** - All steps are logged and can be independently verified

## Quick Reference

| Aspect | Details |
|--------|---------|
| **Dataset** | SWE-bench Lite (300 instances) from Princeton NLP |
| **Source** | HuggingFace: `princeton-nlp/SWE-bench_Lite` |
| **Evaluation** | Official SWE-bench harness (`swebench.harness.run_evaluation`) |
| **Docker Images** | Epoch AI optimized images (`ghcr.io/epoch-research/swe-bench.eval.*`) |
| **Pass Criteria** | All FAIL_TO_PASS tests pass, zero PASS_TO_PASS regressions |

## Documentation Index

| Document | Description |
|----------|-------------|
| [Evaluation Methodology](./evaluation-methodology.md) | Step-by-step breakdown of the evaluation process |
| [Dataset Source](./dataset-source.md) | Where the benchmark data comes from and how it's loaded |
| [Docker Environment](./docker-environment.md) | Container setup, isolation, and resource limits |
| [Agent Constraints](./agent-constraints.md) | What the agent can and cannot do during evaluation |
| [Verification Guide](./verification.md) | How to independently verify results |
| [FAQ](./faq.md) | Common questions about evaluation integrity |

## The SWE-bench Benchmark

[SWE-bench](https://www.swebench.com/) is a benchmark created by Princeton NLP to evaluate AI systems on real-world software engineering tasks. Each instance consists of:

1. **A real GitHub issue** - Actual bug reports or feature requests from popular Python repositories
2. **A base commit** - The exact state of the codebase when the issue was filed
3. **Test cases** - Tests that fail before the fix and pass after (FAIL_TO_PASS)
4. **Regression tests** - Tests that should continue to pass (PASS_TO_PASS)
5. **A gold patch** - The actual fix committed by the repository maintainers (hidden from the agent)

### SWE-bench Lite

SWE-bench Lite is a curated subset of 300 instances that are:
- More reliably solvable
- Better documented
- Faster to evaluate
- Representative of the full benchmark

## Running the Benchmark

```bash
# Setup (one-time)
just swebench-setup

# Run evaluation
just swebench                           # All 300 instances with defaults
just swebench 0-49                      # First 50 instances
just swebench 0-9 vertex-claude claude-sonnet-4-20250514  # Custom model
```

### CLI Options

```bash
cargo run --no-default-features --features evals --bin qbit-cli -- \
    --swebench \
    --problems 0-49 \
    --eval-provider vertex-claude \
    --eval-model claude-opus-4-5@20251101 \
    --output ./results.json \
    --results-dir ./swebench-results \
    --json -v
```

| Flag | Description |
|------|-------------|
| `--problems` | Instance filter: `0-49`, `django__django-11133`, or `django/django` |
| `--eval-provider` | LLM provider to use |
| `--eval-model` | Model identifier |
| `--output` | Path for JSON results |
| `--results-dir` | Directory for detailed results |
| `--json` | Machine-readable output |
| `-v` | Verbose logging |

## Key Principles

### 1. No Data Leakage

The agent cannot access:
- The gold patch (the actual fix)
- Git history (which contains the fix commit)
- Solutions from other instances

### 2. Identical Test Environment

Tests run in the same Docker containers used by the official SWE-bench evaluation, ensuring:
- Correct Python version
- Correct package dependencies
- Repository-specific test runners

### 3. Official Evaluation

When available, we use the official SWE-bench Python harness:
```python
python -m swebench.harness.run_evaluation
```

This is the same evaluation used by academic papers and the SWE-bench leaderboard.

### 4. Strict Pass Criteria

An instance is marked "resolved" only when:
- **ALL** FAIL_TO_PASS tests pass (the tests that verify the fix)
- **ALL** PASS_TO_PASS tests pass (no regressions introduced)

Partial fixes are tracked separately and do not count toward the pass rate.

## Transparency Commitments

1. **Open Source** - All evaluation code is in `backend/crates/qbit-swebench/`
2. **Logged Outputs** - Full agent transcripts available for review
3. **Reproducible** - Same inputs produce same evaluation results
4. **No Cherry-picking** - All attempted instances are reported

## Contact

For questions about our evaluation methodology, please open an issue on GitHub or reach out to the maintainers.
