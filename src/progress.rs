//! Progress Tracker — real-time visibility into running test suites.
//!
//! Callers can check in at any point during a run to see how far along
//! it is without waiting for the full result.

use crate::types::RunId;
use crate::types::RunProgress;
use crate::types::TestResult;

/// Tracks the state of an in-flight test run and provides snapshots
/// on demand.
pub trait ProgressTracker {
    /// Initialize tracking for a new run.
    fn start_run(&mut self, run_id: RunId, total_tests: u32);

    /// Record that a test has started executing.
    fn test_started(&mut self, run_id: &str, test_id: &str);

    /// Record that a test has completed with a result.
    fn test_completed(&mut self, run_id: &str, result: &TestResult);

    /// Get a snapshot of the current progress for a run.
    /// Returns None if the run_id is not known.
    fn get_progress(&self, run_id: &str) -> Option<RunProgress>;

    /// Mark a run as complete. After this, get_progress still works
    /// but will show 100% complete.
    fn finish_run(&mut self, run_id: &str);

    /// List all run IDs currently being tracked.
    fn active_runs(&self) -> Vec<RunId>;
}
