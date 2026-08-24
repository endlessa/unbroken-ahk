//! Test Manager — the top-level orchestrator.
//!
//! This is the entry point that ties everything together. Both the MCP
//! tool interface and the console interface talk to the manager. It
//! coordinates discovery, filtering, execution, progress tracking,
//! and result packaging.

use crate::discovery::DiscoveryQuery;
use crate::discovery::DiscoveryResult;
use crate::discovery::DiscoverySummary;
use crate::types::RunConfig;
use crate::types::RunId;
use crate::types::TestId;
use crate::types::RunProgress;
use crate::types::RunSummary;
use crate::types::TestDefinition;

/// The central orchestration interface for the test platform.
///
/// Callers (AI via MCP or human via console) use this interface for
/// the full lifecycle: discover → configure → run → track → collect results.
pub trait TestManager {
    // -- Discovery ----------------------------------------------------------

    /// Query available tests.
    fn discover(&self, query: &DiscoveryQuery) -> DiscoveryResult;

    /// Get a high-level summary of all available tests.
    fn summary(&self) -> DiscoverySummary;

    // -- Registration -------------------------------------------------------

    /// Register a test with the platform.
    fn register_test(&mut self, definition: TestDefinition) -> Result<(), ManagerError>;

    // -- Execution ----------------------------------------------------------

    /// Start a test run with the given configuration.
    /// Returns a run ID that can be used to check progress.
    fn start_run(&mut self, config: RunConfig) -> Result<RunId, ManagerError>;

    // -- Progress -----------------------------------------------------------

    /// Check on the progress of a running test suite.
    fn check_progress(&self, run_id: &str) -> Result<RunProgress, ManagerError>;

    /// List all currently active runs.
    fn active_runs(&self) -> Vec<RunId>;

    // -- Results ------------------------------------------------------------

    /// Get the final results of a completed run.
    /// Returns an error if the run is still in progress.
    fn get_results(&self, run_id: &str) -> Result<RunSummary, ManagerError>;
}

/// Errors from the test manager.
#[derive(Debug, Clone)]
pub enum ManagerError {
    /// No run exists with this ID.
    UnknownRun(RunId),
    /// A run file exists but could not be parsed — the run happened, its
    /// record is damaged (or written by an incompatible version).
    CorruptRun(RunId, String),
    /// A run file exists but could not be READ (permissions, transient
    /// I/O) — says nothing about the data; retry may succeed.
    ReadFailed(RunId, String),
    /// The run id was claimed by some session but no summary was ever
    /// persisted: still executing in another session, or that session
    /// died mid-run. If it is known dead, delete runs/<id>.json to clear.
    RunNotPersisted(RunId),
    /// The run has not completed yet.
    RunInProgress(RunId),
    /// The run already completed — cannot start again.
    RunAlreadyComplete(RunId),
    /// No tests matched the given configuration.
    NoTestsMatched,
    /// include_ids named tests that are not registered — a typo must
    /// error, never silently shrink the run.
    UnknownTestIds(Vec<TestId>),
    /// A test registration failed.
    RegistrationFailed(String),
    /// The configuration requests something this build cannot do.
    UnsupportedConfig(String),
    /// The operation SUCCEEDED in memory (a run executed and is queryable
    /// via get_results, or a test registered) but could not be written to
    /// storage. The first field names what was being persisted: a run id,
    /// a test id, or a registration-batch descriptor.
    PersistFailed(String, String),
}

/// Human/agent-facing rendering: every variant states what happened AND
/// the recovery it implies, so the generic presentation sites (console,
/// MCP) surface the guidance from the doc comments above instead of a
/// bare Debug dump like 'RunNotPersisted("run_0005")'.
impl std::fmt::Display for ManagerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ManagerError::UnknownRun(id) => write!(f, "no run named '{}' exists", id),
            ManagerError::CorruptRun(id, msg) => write!(
                f,
                "run '{}' happened but its record is damaged or written by an \
                 incompatible version: {}",
                id, msg
            ),
            ManagerError::ReadFailed(id, msg) => write!(
                f,
                "run '{}' exists but could not be read ({}); this says nothing \
                 about the data — a retry may succeed",
                id, msg
            ),
            ManagerError::RunNotPersisted(id) => write!(
                f,
                "run '{}' was claimed but no summary was ever persisted: it is \
                 still executing in another session, or that session died \
                 mid-run; if it is known dead, delete runs/{}.json to clear it",
                id, id
            ),
            ManagerError::RunInProgress(id) => {
                write!(f, "run '{}' has not completed yet", id)
            }
            ManagerError::RunAlreadyComplete(id) => {
                write!(f, "run '{}' already completed and cannot be started again", id)
            }
            ManagerError::NoTestsMatched => {
                write!(f, "no tests matched the given configuration")
            }
            ManagerError::UnknownTestIds(ids) => {
                write!(f, "include_ids named tests that are not registered: {:?}", ids)
            }
            ManagerError::RegistrationFailed(msg) => {
                write!(f, "test registration failed: {}", msg)
            }
            ManagerError::UnsupportedConfig(msg) => {
                write!(f, "unsupported configuration: {}", msg)
            }
            ManagerError::PersistFailed(what, msg) => write!(
                f,
                "'{}' succeeded in memory but could not be written to storage: {}",
                what, msg
            ),
        }
    }
}
