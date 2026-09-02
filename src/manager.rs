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
    /// died mid-run. The empty reservation file is inert and should be
    /// LEFT IN PLACE — it is what keeps the id from being re-minted for
    /// an unrelated future run.
    RunNotPersisted(RunId),
    /// The run has not completed yet.
    RunInProgress(RunId),
    /// No tests matched the given configuration.
    NoTestsMatched,
    /// include_ids named tests that are not registered — a typo must
    /// error, never silently shrink the run.
    UnknownTestIds(Vec<TestId>),
    /// A tag criterion matched no registered test. exclude=false is an
    /// include_tags typo (would silently shrink the run); exclude=true
    /// is an exclude_tags typo (would silently WIDEN it). Typed, so each
    /// surface can render its own vocabulary (--tag/--exclude at the
    /// console) without rewriting Display strings.
    ZeroMatchTags { exclude: bool, tags: Vec<String> },
    /// name_pattern matched no registered test — same typo class.
    ZeroMatchPattern(String),
    /// A test registration failed.
    RegistrationFailed(String),
    /// The configuration requests something this build cannot do.
    UnsupportedConfig(String),
    /// The operation SUCCEEDED in memory (a run executed and is queryable
    /// via get_results, or a test registered) but could not be written to
    /// storage. The first field names what was being persisted: a run id,
    /// a test id, or a registration-batch descriptor.
    PersistFailed(String, String),
    /// The run could NOT start — storage failed while claiming a run id,
    /// so NO tests were executed. Distinct from PersistFailed, where the
    /// run did execute; here a retry is safe and expected.
    RunStartFailed(String),
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
                 mid-run; leave runs/{}.json in place — the empty reservation \
                 is inert, and deleting it would let an unrelated future run \
                 be minted under this same id",
                id, id
            ),
            ManagerError::RunInProgress(id) => {
                write!(f, "run '{}' has not completed yet", id)
            }
            ManagerError::NoTestsMatched => {
                write!(f, "no tests matched the given configuration")
            }
            ManagerError::UnknownTestIds(ids) => {
                write!(f, "include_ids named tests that are not registered: {:?}", ids)
            }
            ManagerError::ZeroMatchTags { exclude: false, tags } => {
                write!(f, "include_tags {:?} match no registered test", tags)
            }
            ManagerError::ZeroMatchTags { exclude: true, tags } => write!(
                f,
                "exclude_tags {:?} match no registered test — a typo here \
                 would silently run the tests it meant to exclude; if the \
                 tag was intentionally retired, remove it from exclude_tags",
                tags
            ),
            ManagerError::ZeroMatchPattern(pattern) => {
                write!(f, "name_pattern {:?} matches no registered test", pattern)
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
            ManagerError::RunStartFailed(msg) => {
                write!(f, "the run did not start: {}", msg)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_match_tags_display_distinguishes_include_from_exclude() {
        // INVARIANT: the two ZeroMatchTags Display arms render their OWN
        // field's vocabulary — an include typo (would silently shrink the
        // run) must speak of include_tags, an exclude typo (would silently
        // WIDEN it) must speak of exclude_tags and carry the would-run-
        // excluded-tests guidance. Swapping the arms would hand the wrong
        // recovery advice to the MCP/JSON surface, where this Display is
        // the only rendering.
        let include = ManagerError::ZeroMatchTags {
            exclude: false,
            tags: vec!["fast".into()],
        };
        assert_eq!(
            format!("{}", include),
            "include_tags [\"fast\"] match no registered test"
        );
        let exclude = ManagerError::ZeroMatchTags {
            exclude: true,
            tags: vec!["gone".into()],
        };
        assert_eq!(
            format!("{}", exclude),
            "exclude_tags [\"gone\"] match no registered test — a typo here \
             would silently run the tests it meant to exclude; if the \
             tag was intentionally retired, remove it from exclude_tags"
        );
    }

    #[test]
    fn corrupt_run_and_read_failed_display_carry_their_own_guidance() {
        // INVARIANT: CorruptRun and ReadFailed carry identical (id, msg)
        // payloads but OPPOSITE recovery advice — corruption says the data
        // is damaged (do not expect a retry to help), a read failure says
        // nothing about the data (a retry may succeed). Swapped bodies
        // would invite deleting an intact record or endlessly retrying a
        // damaged one.
        let corrupt = ManagerError::CorruptRun("run_0042".into(), "bad json".into());
        assert_eq!(
            format!("{}", corrupt),
            "run 'run_0042' happened but its record is damaged or written by an \
             incompatible version: bad json"
        );
        let unreadable = ManagerError::ReadFailed("run_0042".into(), "permission denied".into());
        assert_eq!(
            format!("{}", unreadable),
            "run 'run_0042' exists but could not be read (permission denied); \
             this says nothing about the data — a retry may succeed"
        );
    }

    #[test]
    fn run_in_progress_display_says_not_completed() {
        // INVARIANT: a live same-session run renders as still-executing —
        // not as any of the storage-shaped outcomes (unknown, reserved by
        // another session, damaged) — so a poller knows to wait rather
        // than re-run or clean up.
        let live = ManagerError::RunInProgress("run_0007".into());
        assert_eq!(format!("{}", live), "run 'run_0007' has not completed yet");
    }

    #[test]
    fn zero_match_pattern_display_echoes_the_pattern_in_json_vocabulary() {
        // INVARIANT: the pattern-typo error names the JSON field
        // (name_pattern) and echoes the offending pattern verbatim
        // (Debug-quoted), so an agent reading the MCP surface can see
        // WHICH pattern missed without re-deriving it from the request.
        let miss = ManagerError::ZeroMatchPattern("zz_nomatch_*".into());
        assert_eq!(
            format!("{}", miss),
            "name_pattern \"zz_nomatch_*\" matches no registered test"
        );
    }

    #[test]
    fn persist_failed_display_states_memory_success() {
        // INVARIANT: PersistFailed's generic rendering — the only one
        // library callers see on registration-path failures — names WHAT
        // was being persisted and states the operation already succeeded
        // in memory, so a caller retries the WRITE rather than redoing
        // (and possibly double-applying) the operation itself.
        let stranded = ManagerError::PersistFailed("t1".into(), "disk full".into());
        assert_eq!(
            format!("{}", stranded),
            "'t1' succeeded in memory but could not be written to storage: disk full"
        );
    }
}
