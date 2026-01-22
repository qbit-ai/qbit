# Dataset Source

This document explains exactly where SWE-bench data comes from and how we load it.

## Primary Source: HuggingFace

We download the SWE-bench Lite dataset directly from HuggingFace:

| Field | Value |
|-------|-------|
| **API Endpoint** | `https://datasets-server.huggingface.co/rows` |
| **Dataset ID** | `princeton-nlp/SWE-bench_Lite` |
| **Split** | `test` |
| **Total Instances** | 300 |

### Direct Verification

You can verify the dataset independently:

```python
# Using the HuggingFace datasets library
from datasets import load_dataset

dataset = load_dataset("princeton-nlp/SWE-bench_Lite", split="test")
print(f"Total instances: {len(dataset)}")  # Should be 300
```

Or via the API:

```bash
curl "https://datasets-server.huggingface.co/rows?\
dataset=princeton-nlp/SWE-bench_Lite&config=default&split=test&offset=0&length=10"
```

## Dataset Schema

Each instance contains these fields:

```rust
// From types.rs
pub struct SWEBenchInstance {
    /// Unique identifier (e.g., "django__django-11133")
    pub instance_id: String,

    /// Repository in owner/name format (e.g., "django/django")
    pub repo: String,

    /// Base commit hash to checkout before fixing
    pub base_commit: String,

    /// The problem description from the GitHub issue
    pub problem_statement: String,

    /// The gold patch - THE ACTUAL FIX (hidden from agent)
    pub patch: String,

    /// Test patch - adds FAIL_TO_PASS tests
    pub test_patch: String,

    /// Tests that should fail before fix, pass after
    #[serde(rename = "FAIL_TO_PASS")]
    pub fail_to_pass: String,  // JSON array

    /// Tests that should pass both before and after
    #[serde(rename = "PASS_TO_PASS")]
    pub pass_to_pass: String,  // JSON array

    /// Software version (e.g., "3.0")
    pub version: String,

    /// Commit hash for environment setup
    pub environment_setup_commit: String,

    /// Optional hints (not always present)
    pub hints_text: Option<String>,
}
```

### Example Instance

```json
{
  "instance_id": "django__django-11133",
  "repo": "django/django",
  "base_commit": "e7fd69d051eaa67cb17f172a39b57253e9cb831a",
  "problem_statement": "HttpResponse doesn't handle memoryview objects...",
  "patch": "diff --git a/django/http/response.py...",
  "test_patch": "diff --git a/tests/httpwrappers/tests.py...",
  "FAIL_TO_PASS": "[\"httpwrappers.tests.HttpResponseTests.test_memoryview_content\"]",
  "PASS_TO_PASS": "[\"httpwrappers.tests.HttpResponseTests.test_streaming_response\", ...]",
  "version": "3.0",
  "environment_setup_commit": "419a78300f7cd27611196e1e464d50fd0385ff27"
}
```

## What the Agent Sees vs. What's Hidden

| Field | Shown to Agent | Purpose |
|-------|----------------|---------|
| `instance_id` | ✓ Yes | Identification |
| `repo` | ✓ Yes | Context |
| `base_commit` | ✗ Internal only | Checkout point |
| `problem_statement` | ✓ Yes | **The task description** |
| `patch` | ✗ **HIDDEN** | Gold solution |
| `test_patch` | ✗ Internal only | Applied automatically |
| `FAIL_TO_PASS` | ✓ Yes | Tests to make pass |
| `PASS_TO_PASS` | ✗ Internal only | Regression checking |
| `version` | ✓ Yes | Context |
| `hints_text` | ✓ Yes (if present) | Optional hints |

**Critical:** The `patch` field (the actual fix) is NEVER shown to the agent.

## Loading Process

### 1. Download from HuggingFace

```rust
// From loader.rs
async fn download_and_cache(&self) -> Result<Vec<SWEBenchInstance>> {
    let mut all_instances = Vec::new();
    let mut offset = 0;

    loop {
        let url = format!(
            "{}?dataset={}&config=default&split=test&offset={}&length={}",
            HUGGINGFACE_DATASETS_API, DATASET_ID, offset, ROWS_PER_PAGE
        );

        let response = self.client.get(&url).send().await?;
        let hf_response: HuggingFaceResponse = response.json().await?;

        for row in hf_response.rows {
            all_instances.push(row.row);
        }

        if all_instances.len() >= hf_response.num_rows_total {
            break;
        }
        offset += ROWS_PER_PAGE;  // 100 per page
    }

    Ok(all_instances)
}
```

### 2. Local Caching

To avoid repeated downloads, we cache locally:

```
~/.qbit/benchmarks/swebench/datasets/lite.json
```

```rust
// From loader.rs
pub async fn load_lite(&self) -> Result<Vec<SWEBenchInstance>> {
    let cache_path = self.cache_path();

    if cache_path.exists() {
        return self.load_from_file(&cache_path);  // Use cache
    }

    self.download_and_cache().await  // Download fresh
}
```

### 3. Instance Filtering

Users can filter instances by:

```rust
// From loader.rs
pub fn parse_instance_filter(filter: &str) -> InstanceFilter {
    // Instance ID: "django__django-11133"
    if filter.contains("__") {
        return InstanceFilter::ById(filter.to_string());
    }

    // Repository: "django/django"
    if filter.contains('/') {
        return InstanceFilter::ByRepo(filter.to_string());
    }

    // Numeric range: "0-49", "10,20,30"
    InstanceFilter::ByIndex(parse_numeric_range(filter))
}
```

## Repository Distribution

The 300 instances come from these repositories:

| Repository | Count | Type |
|------------|-------|------|
| django/django | ~50 | Web framework |
| sympy/sympy | ~40 | Symbolic math |
| scikit-learn/scikit-learn | ~35 | Machine learning |
| matplotlib/matplotlib | ~25 | Plotting |
| astropy/astropy | ~20 | Astronomy |
| sphinx-doc/sphinx | ~20 | Documentation |
| pytest-dev/pytest | ~15 | Testing framework |
| pallets/flask | ~15 | Web framework |
| psf/requests | ~15 | HTTP library |
| And others... | ~65 | Various |

## Problem Types

SWE-bench instances include:

- **Bug fixes** - Incorrect behavior that needs correction
- **Edge cases** - Handling of unusual inputs
- **Compatibility** - Supporting new Python versions or dependencies
- **Performance** - Fixing slow or resource-intensive code
- **API improvements** - Better interfaces or error messages

## Original Data Creation

The SWE-bench dataset was created by:

1. **Mining GitHub** - Finding merged PRs that fix issues
2. **Filtering** - Selecting PRs with clear issue→fix relationships
3. **Test Extraction** - Identifying tests added by the PR
4. **Validation** - Ensuring tests fail before and pass after the fix

See the [original SWE-bench paper](https://arxiv.org/abs/2310.06770) for methodology details.

## Integrity Guarantees

1. **No modification** - We use the dataset exactly as published
2. **Consistent ordering** - Instance indices are stable
3. **Complete dataset** - All 300 instances are available
4. **Cache freshness** - Delete `~/.qbit/benchmarks/` to re-download

## Verifying Dataset Integrity

```python
# Compare with official dataset
from datasets import load_dataset
import hashlib

dataset = load_dataset("princeton-nlp/SWE-bench_Lite", split="test")

# Check instance count
assert len(dataset) == 300

# Verify specific instance
django_instance = next(d for d in dataset if d['instance_id'] == 'django__django-11133')
assert 'memoryview' in django_instance['problem_statement'].lower()
```

## Troubleshooting

### Dataset Download Fails

```bash
# Check connectivity
curl -I https://datasets-server.huggingface.co/

# Clear cache and retry
rm -rf ~/.qbit/benchmarks/swebench/datasets/
just swebench 0-1  # Will re-download
```

### Cache Corruption

```bash
# Validate JSON
python -c "import json; json.load(open('~/.qbit/benchmarks/swebench/datasets/lite.json'))"

# If invalid, delete and re-download
rm ~/.qbit/benchmarks/swebench/datasets/lite.json
```
