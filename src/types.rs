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
#[derive(Debug, Clone)]
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
    /// If true, execute every registered test. Filters are ignored.
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
    pub skipped: u32,
    pub running: u32,
    pub percent_complete: f64,
    pub elapsed_ms: DurationMs,
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
