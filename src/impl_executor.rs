//! Concrete implementation of TestExecutor.
//!
//! Sequential executor — runs tests one at a time. In WASM we don't
//! have threads, so this is the natural starting point. Parallel
//! execution can be added later using WASM container spawning.

use crate::executor::{RunnableTest, TestExecutor};
use crate::types::{DurationMs, TestResult, TestStatus};

/// Runs tests sequentially, calling the progress callback after each.
pub struct SequentialExecutor;

impl SequentialExecutor {
    pub fn new() -> Self {
        Self
    }
}

impl TestExecutor for SequentialExecutor {
    fn execute(
        &self,
        tests: &[&dyn RunnableTest],
        timeout_ms: Option<DurationMs>,
        fail_fast: bool,
        on_result: &mut dyn FnMut(&TestResult),
    ) -> Vec<TestResult> {
        let mut results = Vec::with_capacity(tests.len());

        for test in tests {
            let mut result = test.run(timeout_ms);
            // The REGISTERED id is authoritative. A buggy runnable
            // reporting a wrong test_id would mis-attribute the record;
            // an empty one would persist a run file the strict loader
            // cannot reload (MissingField -> CorruptRun) — the platform
            // must never write what it cannot read back.
            if result.test_id != test.id() {
                result.test_id = test.id().to_string();
            }
            on_result(&result);

            let should_stop = fail_fast
                && matches!(result.status, TestStatus::Failed | TestStatus::Error);

            results.push(result);

            if should_stop {
                // Mark remaining tests as skipped
                for remaining in &tests[results.len()..] {
                    let skipped = TestResult::fail_fast_skip(remaining.id());
                    on_result(&skipped);
                    results.push(skipped);
                }
                break;
            }
        }

        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct SimpleTest {
        id: String,
        pass: bool,
    }

    impl RunnableTest for SimpleTest {
        fn id(&self) -> &str {
            &self.id
        }

        fn run(&self, _timeout_ms: Option<DurationMs>) -> TestResult {
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

    /// Runnable that always reports TestStatus::Error — the crash/panic
    /// outcome no other in-crate runnable produces.
    struct ErrorTest {
        id: String,
    }

    impl RunnableTest for ErrorTest {
        fn id(&self) -> &str {
            &self.id
        }

        fn run(&self, _timeout_ms: Option<DurationMs>) -> TestResult {
            TestResult {
                test_id: self.id.clone(),
                status: TestStatus::Error,
                duration_ms: 1,
                message: Some("test crashed".into()),
                stdout: None,
                stderr: None,
            }
        }
    }

    /// Runnable that echoes the timeout it was handed back into its
    /// result message, so a test can observe what actually reached run().
    struct TimeoutEchoTest {
        id: String,
    }

    impl RunnableTest for TimeoutEchoTest {
        fn id(&self) -> &str {
            &self.id
        }

        fn run(&self, timeout_ms: Option<DurationMs>) -> TestResult {
            TestResult {
                test_id: self.id.clone(),
                status: TestStatus::Passed,
                duration_ms: 1,
                message: Some(format!("timeout={:?}", timeout_ms)),
                stdout: None,
                stderr: None,
            }
        }
    }

    #[test]
    fn runs_all_tests() {
        let t1 = SimpleTest { id: "a".into(), pass: true };
        let t2 = SimpleTest { id: "b".into(), pass: true };
        let tests: Vec<&dyn RunnableTest> = vec![&t1, &t2];
        let mut count = 0;
        let results = SequentialExecutor::new().execute(&tests, None, false, &mut |_| count += 1);
        assert_eq!(results.len(), 2);
        assert_eq!(count, 2);
    }

    // INVARIANT: on_result fires once per emitted result — INCLUDING the
    // Skipped records the fail_fast stop synthesizes — in execution order,
    // so live progress trackers see every record the run summary will hold.
    #[test]
    fn fail_fast_skips_remaining() {
        let t1 = SimpleTest { id: "a".into(), pass: false };
        let t2 = SimpleTest { id: "b".into(), pass: true };
        let t3 = SimpleTest { id: "c".into(), pass: true };
        let tests: Vec<&dyn RunnableTest> = vec![&t1, &t2, &t3];
        let mut seen: Vec<String> = Vec::new();
        let results = SequentialExecutor::new().execute(&tests, None, true, &mut |r| {
            seen.push(r.test_id.clone())
        });
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].status, TestStatus::Failed);
        assert_eq!(results[1].status, TestStatus::Skipped);
        assert_eq!(results[2].status, TestStatus::Skipped);
        // The callback saw the skipped ids too, not just the executed test.
        assert_eq!(seen, vec!["a".to_string(), "b".to_string(), "c".to_string()]);
        // Skip records explain WHY the tests were cut off.
        assert_eq!(results[1].message.as_deref(), Some("Skipped due to fail_fast"));
        assert_eq!(results[2].message.as_deref(), Some("Skipped due to fail_fast"));
    }

    // INVARIANT: fail_fast stops the suite on TestStatus::Error exactly as
    // it does on Failed — a crashed test must not let the remainder keep
    // executing.
    #[test]
    fn fail_fast_stops_on_error_status() {
        let t1 = ErrorTest { id: "a".into() };
        let t2 = SimpleTest { id: "b".into(), pass: true };
        let t3 = SimpleTest { id: "c".into(), pass: true };
        let tests: Vec<&dyn RunnableTest> = vec![&t1, &t2, &t3];
        let results = SequentialExecutor::new().execute(&tests, None, true, &mut |_| {});
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].status, TestStatus::Error);
        assert_eq!(results[1].status, TestStatus::Skipped);
        assert_eq!(results[2].status, TestStatus::Skipped);
    }

    // INVARIANT: the caller's timeout_ms reaches RunnableTest::run verbatim.
    // Timeout enforcement is delegated ENTIRELY to the runnable, so this one
    // pass-through is the crate's whole timeout mechanism.
    #[test]
    fn timeout_ms_is_forwarded_to_the_runnable() {
        let t = TimeoutEchoTest { id: "a".into() };
        let tests: Vec<&dyn RunnableTest> = vec![&t];
        let results = SequentialExecutor::new().execute(&tests, Some(1234), false, &mut |_| {});
        assert_eq!(results[0].message.as_deref(), Some("timeout=Some(1234)"));
        // And None stays None — the executor must not invent a timeout.
        let results = SequentialExecutor::new().execute(&tests, None, false, &mut |_| {});
        assert_eq!(results[0].message.as_deref(), Some("timeout=None"));
    }
}
