//! Concrete implementation of ProgressTracker.

use crate::json::{JsonValue, ToJson, to_json_pretty};
use crate::progress::ProgressTracker;
use crate::types::{RunId, RunProgress, TestResult, TestStatus};

/// Internal state for a single tracked run.
struct RunState {
    run_id: RunId,
    total: u32,
    completed: u32,
    passed: u32,
    failed: u32,
    skipped: u32,
    running: u32,
    started_ms: u64,
    finished: bool,
}

/// In-memory progress tracker that can be serialized to JSON.
pub struct InMemoryProgressTracker {
    runs: Vec<RunState>,
    /// Simple monotonic counter used when no real clock is available (WASM).
    /// Callers can set this via `set_clock` to provide real timestamps.
    clock_fn: fn() -> u64,
}

/// Default clock returns 0 (no real time in WASM).
fn zero_clock() -> u64 {
    0
}

impl InMemoryProgressTracker {
    pub fn new() -> Self {
        Self {
            runs: Vec::new(),
            clock_fn: zero_clock,
        }
    }

    /// Set a clock function for timestamps (e.g. in non-WASM environments).
    pub fn with_clock(mut self, clock: fn() -> u64) -> Self {
        self.clock_fn = clock;
        self
    }

    fn now(&self) -> u64 {
        (self.clock_fn)()
    }

    fn find(&self, run_id: &str) -> Option<&RunState> {
        self.runs.iter().find(|r| r.run_id == run_id)
    }

    fn find_mut(&mut self, run_id: &str) -> Option<&mut RunState> {
        self.runs.iter_mut().find(|r| r.run_id == run_id)
    }

    /// Serialize all tracked runs to JSON for debugging.
    pub fn to_json_string(&self) -> String {
        let runs: Vec<JsonValue> = self.runs.iter().map(|r| {
            let progress = RunProgress {
                run_id: r.run_id.clone(),
                total: r.total,
                completed: r.completed,
                passed: r.passed,
                failed: r.failed,
                skipped: r.skipped,
                running: r.running,
                percent_complete: if r.total > 0 {
                    (r.completed as f64 / r.total as f64) * 100.0
                } else {
                    0.0
                },
                elapsed_ms: self.now().saturating_sub(r.started_ms),
            };
            progress.to_json()
        }).collect();
        to_json_pretty(&JsonValue::Array(runs))
    }
}

impl ProgressTracker for InMemoryProgressTracker {
    fn start_run(&mut self, run_id: RunId, total_tests: u32) {
        self.runs.push(RunState {
            run_id,
            total: total_tests,
            completed: 0,
            passed: 0,
            failed: 0,
            skipped: 0,
            running: 0,
            started_ms: self.now(),
            finished: false,
        });
    }

    fn test_started(&mut self, run_id: &str, _test_id: &str) {
        if let Some(state) = self.find_mut(run_id) {
            state.running += 1;
        }
    }

    fn test_completed(&mut self, run_id: &str, result: &TestResult) {
        if let Some(state) = self.find_mut(run_id) {
            if state.running > 0 {
                state.running -= 1;
            }
            state.completed += 1;
            match result.status {
                TestStatus::Passed => state.passed += 1,
                TestStatus::Failed => state.failed += 1,
                TestStatus::Error => state.failed += 1,
                TestStatus::Skipped => state.skipped += 1,
            }
        }
    }

    fn get_progress(&self, run_id: &str) -> Option<RunProgress> {
        let state = self.find(run_id)?;
        Some(RunProgress {
            run_id: state.run_id.clone(),
            total: state.total,
            completed: state.completed,
            passed: state.passed,
            failed: state.failed,
            skipped: state.skipped,
            running: state.running,
            percent_complete: if state.total > 0 {
                (state.completed as f64 / state.total as f64) * 100.0
            } else {
                0.0
            },
            elapsed_ms: self.now().saturating_sub(state.started_ms),
        })
    }

    fn finish_run(&mut self, run_id: &str) {
        if let Some(state) = self.find_mut(run_id) {
            state.finished = true;
            state.running = 0;
        }
    }

    fn active_runs(&self) -> Vec<RunId> {
        self.runs
            .iter()
            .filter(|r| !r.finished)
            .map(|r| r.run_id.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::TestStatus;

    fn result(id: &str, status: TestStatus) -> TestResult {
        TestResult {
            test_id: id.into(),
            status,
            duration_ms: 10,
            message: None,
            stdout: None,
            stderr: None,
        }
    }

    #[test]
    fn tracks_progress() {
        let mut tracker = InMemoryProgressTracker::new();
        tracker.start_run("run1".into(), 3);

        tracker.test_started("run1", "t1");
        tracker.test_completed("run1", &result("t1", TestStatus::Passed));

        let prog = tracker.get_progress("run1").unwrap();
        assert_eq!(prog.completed, 1);
        assert_eq!(prog.passed, 1);
        assert_eq!(prog.total, 3);
        assert!((prog.percent_complete - 33.333).abs() < 1.0);
    }

    #[test]
    fn finish_removes_from_active() {
        let mut tracker = InMemoryProgressTracker::new();
        tracker.start_run("run1".into(), 1);
        assert_eq!(tracker.active_runs().len(), 1);
        tracker.finish_run("run1");
        assert_eq!(tracker.active_runs().len(), 0);
        // Progress still available after finish
        assert!(tracker.get_progress("run1").is_some());
    }
}
