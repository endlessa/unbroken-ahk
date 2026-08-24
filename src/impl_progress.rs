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
    errored: u32,
    skipped: u32,
    running: u32,
    started_ms: u64,
    /// Set by finish_run so elapsed_ms freezes at the run's duration
    /// instead of growing forever after completion.
    finished_ms: Option<u64>,
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

    /// Build the progress snapshot for one run state — single source of
    /// truth for the percent-complete contract (finished runs report 100%).
    fn snapshot(&self, state: &RunState) -> RunProgress {
        RunProgress {
            run_id: state.run_id.clone(),
            total: state.total,
            completed: state.completed,
            passed: state.passed,
            failed: state.failed,
            errored: state.errored,
            skipped: state.skipped,
            running: state.running,
            percent_complete: if state.finished_ms.is_some() {
                100.0
            } else if state.total > 0 {
                (state.completed as f64 / state.total as f64) * 100.0
            } else {
                0.0
            },
            // Frozen at finish; live while running.
            elapsed_ms: state
                .finished_ms
                .unwrap_or_else(|| self.now())
                .saturating_sub(state.started_ms),
            finished: state.finished_ms.is_some(),
        }
    }

    /// Drop a run's tracked state. Called once its summary is durable on
    /// disk — the manager's storage fallback then serves progress for it,
    /// so keeping the state would only grow memory for the session's
    /// lifetime.
    pub fn remove_run(&mut self, run_id: &str) {
        self.runs.retain(|r| r.run_id != run_id);
    }

    /// Serialize all tracked runs to JSON for debugging.
    pub fn to_json_string(&self) -> String {
        let runs: Vec<JsonValue> =
            self.runs.iter().map(|r| self.snapshot(r).to_json()).collect();
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
            errored: 0,
            skipped: 0,
            running: 0,
            started_ms: self.now(),
            finished_ms: None,
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
                // Tracked separately so progress and final results agree
                // on failed-vs-errored counts.
                TestStatus::Error => state.errored += 1,
                TestStatus::Skipped => state.skipped += 1,
            }
        }
    }

    fn get_progress(&self, run_id: &str) -> Option<RunProgress> {
        self.find(run_id).map(|state| self.snapshot(state))
    }

    fn finish_run(&mut self, run_id: &str) {
        let now = self.now();
        if let Some(state) = self.find_mut(run_id) {
            if state.finished_ms.is_none() {
                state.finished_ms = Some(now.max(state.started_ms));
            }
            state.running = 0;
        }
    }

    fn active_runs(&self) -> Vec<RunId> {
        self.runs
            .iter()
            .filter(|r| r.finished_ms.is_none())
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

    use std::sync::atomic::{AtomicU64, Ordering};
    static FAKE_NOW: AtomicU64 = AtomicU64::new(0);
    fn fake_clock() -> u64 {
        FAKE_NOW.load(Ordering::Relaxed)
    }

    #[test]
    fn elapsed_freezes_when_run_finishes() {
        let mut tracker = InMemoryProgressTracker::new().with_clock(fake_clock);
        FAKE_NOW.store(1_000, Ordering::Relaxed);
        tracker.start_run("run1".into(), 1);
        tracker.test_completed("run1", &result("t1", TestStatus::Passed));
        FAKE_NOW.store(1_500, Ordering::Relaxed);
        tracker.finish_run("run1");
        // Long after the run finished, elapsed reports the run's duration,
        // not the age of the run.
        FAKE_NOW.store(9_999_999, Ordering::Relaxed);
        let prog = tracker.get_progress("run1").unwrap();
        assert_eq!(prog.elapsed_ms, 500);
    }

    #[test]
    fn error_status_counted_separately_from_failed() {
        let mut tracker = InMemoryProgressTracker::new();
        tracker.start_run("run1".into(), 2);
        tracker.test_completed("run1", &result("t1", TestStatus::Failed));
        tracker.test_completed("run1", &result("t2", TestStatus::Error));
        let prog = tracker.get_progress("run1").unwrap();
        assert_eq!(prog.failed, 1);
        assert_eq!(prog.errored, 1);
    }

    #[test]
    fn finish_forces_hundred_percent() {
        // Trait contract: after finish_run, get_progress shows 100% even if
        // fewer results arrived than expected.
        let mut tracker = InMemoryProgressTracker::new();
        tracker.start_run("run1".into(), 2);
        tracker.test_completed("run1", &result("t1", TestStatus::Passed));
        tracker.finish_run("run1");
        let prog = tracker.get_progress("run1").unwrap();
        assert_eq!(prog.percent_complete, 100.0);
        // Counts stay truthful.
        assert_eq!(prog.completed, 1);
        assert_eq!(prog.total, 2);
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
