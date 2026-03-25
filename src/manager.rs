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
    /// The run has not completed yet.
    RunInProgress(RunId),
    /// The run already completed — cannot start again.
    RunAlreadyComplete(RunId),
    /// No tests matched the given configuration.
    NoTestsMatched,
    /// A test registration failed.
    RegistrationFailed(String),
}
