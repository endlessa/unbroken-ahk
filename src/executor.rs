//! Test Executor — runs individual tests and captures their output.
//!
//! This is the boundary where test code actually executes. In the WASM
//! world, this may run tests in the manager's container or spawn them
//! into isolated containers.

use crate::types::DurationMs;
use crate::types::TestId;
use crate::types::TestResult;

/// A callable test. Implementations wrap the actual test logic.
///
/// Each test registered with the platform must provide an implementation
/// of this trait. The executor calls `run()` and gets back a result.
pub trait RunnableTest {
    /// The unique ID of this test (must match its TestDefinition).
    fn id(&self) -> &str;

    /// Execute the test and return its result.
    ///
    /// Implementations should:
    /// - Capture stdout/stderr if possible
    /// - Return `TestStatus::Passed` if all assertions hold
    /// - Return `TestStatus::Failed` with a message on assertion failure
    /// - Return `TestStatus::Error` on unexpected panics/crashes
    /// - Respect the timeout if one is provided
    fn run(&self, timeout_ms: Option<DurationMs>) -> TestResult;
}

/// The executor manages running a batch of tests according to
/// the execution model and reports results as they complete.
pub trait TestExecutor {
    /// Execute a set of tests.
    ///
    /// For each test that completes, the executor calls the `on_result`
    /// callback so progress can be tracked in real time.
    ///
    /// Returns all results when the batch is complete, or stops early
    /// if `fail_fast` is true and a test fails.
    fn execute(
        &self,
        tests: &[&dyn RunnableTest],
        timeout_ms: Option<DurationMs>,
        fail_fast: bool,
        on_result: &mut dyn FnMut(&TestResult),
    ) -> Vec<TestResult>;
}

/// Errors that can occur during test execution.
#[derive(Debug, Clone)]
pub enum ExecutionError {
    /// The test exceeded its timeout.
    Timeout { test_id: TestId, limit_ms: DurationMs },
    /// The test panicked or crashed.
    Crash { test_id: TestId, message: String },
    /// The run was aborted (e.g. fail_fast triggered).
    Aborted { completed: u32, remaining: u32 },
}
