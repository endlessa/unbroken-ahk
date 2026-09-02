//! Core types shared across the test platform.
//!
//! These are the fundamental data structures that every component
//! speaks in terms of. No behavior here — just shapes of data.

/// Unique identifier for a test.
pub type TestId = String;

/// Unique identifier for a test run.
pub type RunId = String;

/// Timestamp in milliseconds since epoch.
pub type Timestamp = u64;

/// Duration in milliseconds.
pub type DurationMs = u64;

// ---------------------------------------------------------------------------
// Test Definition
// ---------------------------------------------------------------------------

/// A single test's identity and metadata as known to the registry.
/// This is what discovery produces and what callers see when they search.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestDefinition {
    /// Unique identifier for this test.
    pub id: TestId,
    /// Human-readable name.
    pub name: String,
    /// Free-form tags for filtering (e.g. "smoke", "auth", "slow").
    pub tags: Vec<String>,
    /// Optional logical group (e.g. "authentication", "networking").
    pub group: Option<String>,
    /// Optional description of what this test verifies.
    pub description: Option<String>,
    /// Arbitrary key-value metadata.
    pub metadata: Vec<(String, String)>,
}

// ---------------------------------------------------------------------------
// Run Configuration (JSON input)
// ---------------------------------------------------------------------------

/// What the caller sends to request a test run.
#[derive(Debug, Clone)]
pub struct RunConfig {
    /// If true, start from every registered test. exclude_tags still
    /// applies — "run everything except the slow tag" honors the
    /// exclusion. Combining run_all with include filters is contradictory
    /// intent and rejected by start_run; the filter layer itself would
    /// ignore the includes, but callers never get that far.
    pub run_all: bool,
    /// Run only these specific test IDs.
    pub include_ids: Vec<TestId>,
    /// Run only tests that have ALL of these tags.
    pub include_tags: Vec<String>,
    /// Exclude tests that have ANY of these tags.
    pub exclude_tags: Vec<String>,
    /// Glob/substring pattern matched against test names.
    pub name_pattern: Option<String>,
    /// Stop the entire run on the first failure.
    pub fail_fast: bool,
    /// Per-test timeout. None means no timeout.
    pub timeout_ms: Option<DurationMs>,
    /// Execution strategy.
    pub execution_model: ExecutionModel,
}

/// How tests should be executed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionModel {
    /// One test at a time, in order.
    Sequential,
    /// Up to N tests concurrently.
    Parallel { max_concurrency: u32 },
}

impl RunConfig {
    /// True when any include-side filter is set. The single source of the
    /// "no includes selected" predicate — shared by the filter engine and
    /// the console parser so the layers cannot drift. (The JSON layer
    /// necessarily tests key PRESENCE instead, before parsing collapses
    /// an explicitly-empty list and an absent one into the same value.)
    pub fn has_include_filters(&self) -> bool {
        !self.include_ids.is_empty()
            || !self.include_tags.is_empty()
            || self.name_pattern.is_some()
    }
}

impl Default for RunConfig {
    fn default() -> Self {
        Self {
            run_all: true,
            include_ids: Vec::new(),
            include_tags: Vec::new(),
            exclude_tags: Vec::new(),
            name_pattern: None,
            fail_fast: false,
            timeout_ms: None,
            execution_model: ExecutionModel::Sequential,
        }
    }
}

// ---------------------------------------------------------------------------
// Test Results
// ---------------------------------------------------------------------------

/// Outcome status of a single test.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestStatus {
    Passed,
    Failed,
    Error,
    Skipped,
}

/// Result of executing a single test.
#[derive(Debug, Clone)]
pub struct TestResult {
    /// Which test produced this result.
    pub test_id: TestId,
    /// Outcome.
    pub status: TestStatus,
    /// How long the test took.
    pub duration_ms: DurationMs,
    /// Human-readable outcome message (failure reason, error detail, etc.).
    pub message: Option<String>,
    /// Captured standard output.
    pub stdout: Option<String>,
    /// Captured standard error.
    pub stderr: Option<String>,
}

impl TestResult {
    /// The Error recorded for a selected definition with no registered
    /// runnable. ONE constructor: the manager builds this in two places,
    /// and a drifting message or shape would make records inconsistent
    /// within a single run summary.
    pub fn ghost_error(id: &str) -> TestResult {
        TestResult {
            test_id: id.into(),
            status: TestStatus::Error,
            duration_ms: 0,
            message: Some(format!(
                "no runnable registered for test '{}' (definition only)",
                id
            )),
            stdout: None,
            stderr: None,
        }
    }

    /// The Skipped result for tests cut off by fail_fast — shared by the
    /// executor (its own early stop) and the manager (the remainder
    /// after a definition-only Error), so the two sites cannot diverge.
    pub fn fail_fast_skip(id: &str) -> TestResult {
        TestResult {
            test_id: id.into(),
            status: TestStatus::Skipped,
            duration_ms: 0,
            message: Some("Skipped due to fail_fast".into()),
            stdout: None,
            stderr: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Progress Tracking
// ---------------------------------------------------------------------------

/// A point-in-time snapshot of a run's progress.
/// Returned when a caller checks in on a running suite.
#[derive(Debug, Clone)]
pub struct RunProgress {
    pub run_id: RunId,
    pub total: u32,
    pub completed: u32,
    pub passed: u32,
    pub failed: u32,
    pub errored: u32,
    pub skipped: u32,
    pub running: u32,
    pub percent_complete: f64,
    pub elapsed_ms: DurationMs,
    /// True once the run has finished executing. THIS is the poll-until
    /// signal — percent_complete alone cannot distinguish a live
    /// 60%-done run from a finished legacy run whose file holds fewer
    /// results than selected tests (which reports its truthful <100%).
    pub finished: bool,
}

// ---------------------------------------------------------------------------
// Run Summary
// ---------------------------------------------------------------------------

/// Final packaged result of a completed run.
/// This is what gets sent back to the requesting AI or human.
#[derive(Debug, Clone)]
pub struct RunSummary {
    pub run_id: RunId,
    pub config: RunConfig,
    pub results: Vec<TestResult>,
    pub total: u32,
    pub passed: u32,
    pub failed: u32,
    pub skipped: u32,
    pub errored: u32,
    pub total_duration_ms: DurationMs,
    pub started_at: Timestamp,
    pub completed_at: Timestamp,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::RunnableTest;
    use crate::impl_manager::PlatformManager;
    use crate::manager::TestManager;

    struct EchoTest {
        id: String,
        pass: bool,
    }

    impl RunnableTest for EchoTest {
        fn id(&self) -> &str {
            &self.id
        }
        fn run(&self, _timeout: Option<DurationMs>) -> TestResult {
            TestResult {
                test_id: self.id.clone(),
                status: if self.pass { TestStatus::Passed } else { TestStatus::Failed },
                duration_ms: 1,
                message: if self.pass { None } else { Some("assertion failed".into()) },
                stdout: None,
                stderr: None,
            }
        }
    }

    #[test]
    fn default_config_is_not_fail_fast() {
        // INVARIANT: RunConfig::default() must NOT be fail-fast — a bare
        // console `run` or programmatic default run keeps executing after
        // a failure. If the default flipped to true, a failing test that
        // sorts BEFORE a passing one would silently skip the remainder of
        // the suite; every other default-config fixture in the crate puts
        // its failing test last, so only this test observes the flip.
        assert!(!RunConfig::default().fail_fast);

        let dir = crate::test_util::temp_storage_dir("types-default-ff");
        let mut mgr = PlatformManager::new(&dir);
        // Failing test registered FIRST so it executes first in
        // insertion/selection order, ahead of the passing test.
        mgr.register_runnable(
            crate::test_util::def("t1"),
            Box::new(EchoTest { id: "t1".into(), pass: false }),
        )
        .unwrap();
        mgr.register_runnable(
            crate::test_util::def("t2"),
            Box::new(EchoTest { id: "t2".into(), pass: true }),
        )
        .unwrap();

        let run_id = mgr.start_run(RunConfig::default()).unwrap();
        let summary = mgr.get_results(&run_id).unwrap();
        // The test AFTER the failure still executed — nothing was skipped.
        assert_eq!(summary.results[1].test_id, "t2");
        assert_eq!(summary.results[1].status, TestStatus::Passed);
        assert_eq!(summary.skipped, 0);
        assert_eq!(summary.failed, 1);
        assert_eq!(summary.passed, 1);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn fail_fast_skip_carries_the_shared_skip_reason() {
        // INVARIANT: the ONE fail_fast-skip constructor shared by the
        // executor's early stop and the manager's post-ghost remainder
        // emits a Skipped record that EXPLAINS itself — the message is
        // persisted to run files and shown to MCP/console consumers, so
        // it must exist and name fail_fast, not regress to None or drift
        // in wording between the two call sites.
        let r = TestResult::fail_fast_skip("t9");
        assert_eq!(r.test_id, "t9");
        assert_eq!(r.status, TestStatus::Skipped);
        assert_eq!(r.duration_ms, 0);
        assert_eq!(r.message.as_deref(), Some("Skipped due to fail_fast"));
        assert_eq!(r.stdout, None);
        assert_eq!(r.stderr, None);
    }

    #[test]
    fn ghost_error_message_names_the_offending_test() {
        // INVARIANT: the ghost-Error record's message identifies WHICH
        // test is definition-only, so the diagnostic survives contexts
        // where the message is read in isolation (flattened logs, an
        // agent quoting the error) without the surrounding test_id field.
        let r = TestResult::ghost_error("t2");
        assert_eq!(r.test_id, "t2");
        assert_eq!(r.status, TestStatus::Error);
        assert_eq!(r.duration_ms, 0);
        assert_eq!(
            r.message.as_deref(),
            Some("no runnable registered for test 't2' (definition only)")
        );
        assert_eq!(r.stdout, None);
        assert_eq!(r.stderr, None);
    }
}
