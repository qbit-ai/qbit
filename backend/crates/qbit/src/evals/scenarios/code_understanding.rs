//! Code understanding evaluation scenario.
//!
//! Tests the agent's ability to explain code accurately.

use async_trait::async_trait;

use crate::evals::metrics::{LlmJudgeMetric, Metric};
use crate::evals::scenarios::Scenario;

/// Scenario: Explain code and answer questions about it.
pub struct CodeUnderstandingScenario;

#[async_trait]
impl Scenario for CodeUnderstandingScenario {
    fn name(&self) -> &str {
        "code-understanding"
    }

    fn description(&self) -> &str {
        "Read and explain a binary heap implementation"
    }

    fn testbed(&self) -> &str {
        "rust-understanding"
    }

    fn prompt(&self) -> &str {
        "Read src/lib.rs and answer these questions:\n\
         1. What data structure is implemented?\n\
         2. What is the time complexity of extract_min?\n\
         3. Why does heapify_down compare with children?"
    }

    fn metrics(&self) -> Vec<Box<dyn Metric>> {
        vec![
            Box::new(LlmJudgeMetric::new(
                "identifies_heap",
                "Response correctly identifies this as a binary min-heap or priority queue",
                0.8,
            )),
            Box::new(LlmJudgeMetric::new(
                "correct_complexity",
                "Response correctly states O(log n) time complexity for extract_min",
                0.8,
            )),
            Box::new(LlmJudgeMetric::new(
                "explains_heapify",
                "Response correctly explains heapify_down maintains heap property by moving larger elements down",
                0.7,
            )),
        ]
    }
}

/// Testbed files for the code-understanding scenario.
pub fn testbed_files() -> Vec<(String, String)> {
    vec![
        (
            "Cargo.toml".to_string(),
            r#"[package]
name = "understanding-testbed"
version = "0.1.0"
edition = "2021"

[dependencies]
"#
            .to_string(),
        ),
        (
            "src/lib.rs".to_string(),
            r#"/// A min-heap implementation.
pub struct MinHeap<T> {
    data: Vec<T>,
}

impl<T: Ord> MinHeap<T> {
    /// Create a new empty heap.
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    /// Check if the heap is empty.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Get the number of elements.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Insert an element into the heap.
    pub fn insert(&mut self, value: T) {
        self.data.push(value);
        self.heapify_up(self.data.len() - 1);
    }

    /// Extract the minimum element.
    pub fn extract_min(&mut self) -> Option<T> {
        if self.data.is_empty() {
            return None;
        }

        let last_idx = self.data.len() - 1;
        self.data.swap(0, last_idx);
        let min = self.data.pop();

        if !self.data.is_empty() {
            self.heapify_down(0);
        }

        min
    }

    /// Peek at the minimum element without removing it.
    pub fn peek(&self) -> Option<&T> {
        self.data.first()
    }

    /// Restore heap property upward from index.
    fn heapify_up(&mut self, mut idx: usize) {
        while idx > 0 {
            let parent = (idx - 1) / 2;
            if self.data[idx] < self.data[parent] {
                self.data.swap(idx, parent);
                idx = parent;
            } else {
                break;
            }
        }
    }

    /// Restore heap property downward from index.
    fn heapify_down(&mut self, mut idx: usize) {
        loop {
            let left = 2 * idx + 1;
            let right = 2 * idx + 2;
            let mut smallest = idx;

            if left < self.data.len() && self.data[left] < self.data[smallest] {
                smallest = left;
            }

            if right < self.data.len() && self.data[right] < self.data[smallest] {
                smallest = right;
            }

            if smallest != idx {
                self.data.swap(idx, smallest);
                idx = smallest;
            } else {
                break;
            }
        }
    }
}

impl<T: Ord> Default for MinHeap<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_extract() {
        let mut heap = MinHeap::new();
        heap.insert(5);
        heap.insert(3);
        heap.insert(7);
        heap.insert(1);

        assert_eq!(heap.extract_min(), Some(1));
        assert_eq!(heap.extract_min(), Some(3));
        assert_eq!(heap.extract_min(), Some(5));
        assert_eq!(heap.extract_min(), Some(7));
        assert_eq!(heap.extract_min(), None);
    }
}
"#
            .to_string(),
        ),
    ]
}
