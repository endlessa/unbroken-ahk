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
            let result = test.run(timeout_ms);
            on_result(&result);

            let should_stop = fail_fast
                && matches!(result.status, TestStatus::Failed | TestStatus::Error);

            results.push(result);

            if should_stop {
                // Mark remaining tests as skipped
                for remaining in &tests[results.len()..] {
                    let skipped = TestResult {
                        test_id: remaining.id().to_string(),
                        status: TestStatus::Skipped,
                        duration_ms: 0,
                        message: Some("Skipped due to fail_fast".into()),
                        stdout: None,
                        stderr: None,
                    };
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

    #[test]
    fn fail_fast_skips_remaining() {
        let t1 = SimpleTest { id: "a".into(), pass: false };
        let t2 = SimpleTest { id: "b".into(), pass: true };
        let t3 = SimpleTest { id: "c".into(), pass: true };
        let tests: Vec<&dyn RunnableTest> = vec![&t1, &t2, &t3];
        let results = SequentialExecutor::new().execute(&tests, None, true, &mut |_| {});
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].status, TestStatus::Failed);
        assert_eq!(results[1].status, TestStatus::Skipped);
        assert_eq!(results[2].status, TestStatus::Skipped);
    }
}
