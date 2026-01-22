# Frequently Asked Questions

This FAQ addresses common questions and concerns about our SWE-bench evaluation methodology.

## General Questions

### Q: What is SWE-bench?

**A:** SWE-bench is a benchmark created by Princeton NLP to evaluate AI systems on real-world software engineering tasks. Each instance is a real GitHub issue from a popular Python repository, with test cases that verify whether the issue has been fixed correctly.

### Q: What is SWE-bench Lite?

**A:** SWE-bench Lite is a curated subset of 300 instances (from the full ~2,000+) that are more reliably solvable and faster to evaluate. It's widely used for comparing AI coding assistants.

### Q: Why should I trust your results?

**A:** We encourage verification rather than trust:
- Our evaluation code is open source
- We use the official SWE-bench harness
- We document our exact methodology
- Anyone can re-run evaluations
- See our [Verification Guide](./verification.md)

---

## Data & Dataset Questions

### Q: Where does the benchmark data come from?

**A:** We download the dataset directly from HuggingFace:
- **Dataset ID:** `princeton-nlp/SWE-bench_Lite`
- **API:** HuggingFace Datasets Server
- **Cache:** `~/.qbit/benchmarks/swebench/datasets/lite.json`

You can verify this matches the official dataset. See [Dataset Source](./dataset-source.md).

### Q: Could you have modified the dataset?

**A:** We don't modify the dataset in any way:
- Direct download from HuggingFace
- Cached as-is in JSON format
- Instance IDs match official benchmark
- You can compare byte-for-byte with official download

### Q: Does the agent see the gold patch (the actual fix)?

**A:** No. The `patch` field from each instance is never shown to the agent. The agent only sees:
- Problem statement (the GitHub issue description)
- Repository name and version
- Names of the failing tests
- Optional hints (if present in the dataset)

---

## Evaluation Process Questions

### Q: How do you determine if an instance is "solved"?

**A:** An instance is marked solved only when:
1. **ALL FAIL_TO_PASS tests pass** - Every test that should verify the fix
2. **ALL PASS_TO_PASS tests pass** - No regressions introduced

Partial fixes (some tests pass) do NOT count as solved.

### Q: What test environment do you use?

**A:** We use the same Docker images used by the official SWE-bench evaluation:
- **Primary:** Epoch AI optimized images (`ghcr.io/epoch-research/swe-bench.eval.*`)
- **Fallback:** Official SWE-bench images (`swebench/sweb.eval.*`)

Each image has the correct Python version, dependencies, and repository state.

### Q: What if a Docker image isn't available?

**A:** If no Docker image exists for an instance, we mark it as "skipped" (not failed). This is tracked separately and doesn't inflate the pass rate.

### Q: How long does each evaluation take?

**A:** Varies by instance:
- Simple fixes: 2-5 minutes
- Complex problems: 10-20 minutes
- Timeout limit: 10 minutes for test execution

---

## Agent Capability Questions

### Q: What can the agent do during evaluation?

**A:** The agent can:
- ✓ Read all source files in the repository
- ✓ Modify source files to implement a fix
- ✓ Run tests via the `run_swebench_test` tool
- ✓ Create new source files if needed

### Q: What can't the agent do?

**A:** The agent cannot:
- ✗ See the gold patch (the actual fix)
- ✗ Access git history (contains fix commits)
- ✗ Modify test files (read-only protection)
- ✗ Run arbitrary shell commands
- ✗ Access the network

### Q: Could the agent cheat by accessing git history?

**A:** No. We prevent this multiple ways:
1. The agent uses `run_swebench_test`, not direct docker exec
2. Git commands aren't provided or hinted in the prompt
3. Even if attempted, the working directory is isolated

### Q: Could the agent modify tests to make them pass?

**A:** No. Test files are:
1. Made read-only at the filesystem level
2. Excluded from workspace→testbed synchronization
3. Final evaluation runs in a fresh container

---

## Comparison & Methodology Questions

### Q: How does your evaluation compare to the official SWE-bench leaderboard?

**A:** When the `swebench` Python package is installed, we use the exact same evaluation harness as the official leaderboard:

```python
python -m swebench.harness.run_evaluation
```

Results should be comparable within statistical variance.

### Q: Do you use any special prompting or tools?

**A:** We provide:
- A structured problem description
- The `run_swebench_test` tool for running tests
- Standard file reading/editing capabilities

No special "tricks" or SWE-bench-specific optimizations.

### Q: Why use Epoch AI images instead of official ones?

**A:** Epoch AI images are:
- ~10x smaller (faster to download)
- Available for ARM64 (Apple Silicon) and x86_64
- Functionally equivalent to official images

We fall back to official images if Epoch AI images aren't available.

### Q: Are your results reproducible?

**A:** The evaluation process is reproducible, but LLM outputs are non-deterministic. Running the same evaluation twice may produce different patches (both potentially correct). The test results for a specific patch are fully reproducible.

---

## Skeptic Questions

### Q: How do I know you're not cherry-picking results?

**A:** We report results for all attempted instances:
- Solved: All tests pass
- Partial: Some tests pass, no regressions
- Failed: Tests didn't pass or had regressions
- Error: Execution problems
- Skipped: No Docker image (not counted in %)

The full breakdown is included in output files.

### Q: Could you have fine-tuned on SWE-bench data?

**A:** We use standard commercial LLMs (Claude, etc.) that we don't control the training of. We have no ability to fine-tune these models. The same models are available to anyone for verification.

### Q: How do I verify a specific result?

**A:**
1. Get the patch from our results
2. Run the official SWE-bench harness with that patch
3. Compare the outcome

See [Verification Guide](./verification.md) for detailed steps.

### Q: What if I find a problem with your methodology?

**A:** Please let us know! Open a GitHub issue with:
- What you found
- Steps to reproduce
- Suggested fix (if any)

We commit to investigating and correcting legitimate issues.

---

## Technical Questions

### Q: What happens if tests timeout?

**A:** Test execution has a 10-minute timeout. If exceeded:
- The container is killed
- The instance is marked as failed (not error)
- Timeout is logged for debugging

### Q: How do you handle flaky tests?

**A:** We don't retry tests. If a test fails due to flakiness:
- It counts as a failure
- This matches official SWE-bench methodology
- Known flaky tests are rare in the Lite subset

### Q: What Python versions are used?

**A:** Each repository has its own Python version in the Docker image, matching what the original developers used. Examples:
- Django 3.0: Python 3.7
- Astropy 4.x: Python 3.8
- Recent projects: Python 3.9-3.11

### Q: How do you handle repository-specific test runners?

**A:** We use the same test commands as official SWE-bench:

| Repo | Test Command |
|------|--------------|
| django/django | `./tests/runtests.py --verbosity 2 --settings=test_sqlite` |
| astropy/astropy | `pytest -rA -vv` |
| sympy/sympy | `bin/test -C --verbose` |
| Default | `pytest --no-header -rA` |

---

## Result Interpretation Questions

### Q: What does "X% resolved" mean?

**A:** It means X% of attempted instances had all FAIL_TO_PASS tests pass with no regressions. Skipped instances (no Docker image) are excluded from the denominator.

### Q: Is partial credit counted?

**A:** No. Partial fixes are tracked separately but don't contribute to the resolved percentage. This matches official SWE-bench scoring.

### Q: How does this compare to human developers?

**A:** SWE-bench doesn't include human baselines, but the problems are real issues that human developers originally solved. The benchmark measures whether AI can match human capability on these tasks.

---

## Getting Help

### Q: Where can I learn more about SWE-bench?

**A:**
- [SWE-bench Website](https://www.swebench.com/)
- [SWE-bench Paper](https://arxiv.org/abs/2310.06770)
- [SWE-bench GitHub](https://github.com/princeton-nlp/SWE-bench)

### Q: How do I report a bug in the evaluation?

**A:** Open a GitHub issue with:
- Instance ID affected
- Expected vs actual behavior
- Steps to reproduce
- Relevant logs or output

### Q: Can I contribute to this documentation?

**A:** Yes! Documentation improvements are welcome. Submit a PR with your changes.
