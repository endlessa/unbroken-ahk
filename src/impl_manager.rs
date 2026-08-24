//! Concrete implementation of TestManager — the top-level orchestrator.
//!
//! Wires together registry, filter, executor, progress, discovery,
//! and reporter. Persists everything to JSON via the storage layer.

use crate::discovery::{DiscoveryQuery, DiscoveryResult, DiscoverySummary, TestDiscovery};
use crate::executor::{RunnableTest, TestExecutor};
use crate::filter::TestFilter;
use crate::impl_discovery::RegistryDiscovery;
use crate::impl_executor::SequentialExecutor;
use crate::impl_filter::StandardFilter;
use crate::impl_progress::InMemoryProgressTracker;
use crate::impl_registry::InMemoryRegistry;
use crate::impl_reporter::StandardReporter;
use crate::manager::{ManagerError, TestManager};
use crate::progress::ProgressTracker;
use crate::registry::TestRegistry;
use crate::reporter::{ReportFormat, TestReporter};
use crate::storage::{self, StoragePaths};
use crate::types::*;

/// The concrete test platform manager.
///
/// Holds all state and coordinates the full lifecycle.
pub struct PlatformManager {
    registry: InMemoryRegistry,
    filter: StandardFilter,
    executor: SequentialExecutor,
    progress: InMemoryProgressTracker,
    reporter: StandardReporter,
    storage: StoragePaths,
    /// Map of test_id -> boxed runnable test
    runnables: Vec<Box<dyn RunnableTest>>,
    /// Completed run summaries (also persisted to JSON)
    completed_runs: Vec<RunSummary>,
    /// Counter for generating unique run IDs
    run_counter: u64,
    /// Ids whose definitions were supplied by THIS session's code (via
    /// register_runnable/register_test), as opposed to restored from
    /// storage — decides whether a differing re-registration is a
    /// definition upgrade (storage-sourced) or a programming error
    /// (session-sourced).
    session_defined: std::collections::HashSet<TestId>,
}

impl PlatformManager {
    pub fn new(storage_dir: &str) -> Self {
        let storage = StoragePaths::new(storage_dir);
        // Run-counter continuity across sessions is handled entirely by
        // next_run_id's rescan at mint time — no seeding needed here.
        let run_counter = 0;
        Self {
            registry: InMemoryRegistry::new(),
            filter: StandardFilter::new(),
            executor: SequentialExecutor::new(),
            progress: InMemoryProgressTracker::new().with_clock(storage::now_ms),
            reporter: StandardReporter::new(),
            storage,
            runnables: Vec::new(),
            completed_runs: Vec::new(),
            run_counter,
            session_defined: std::collections::HashSet::new(),
        }
    }

    /// Ensure a definition is registered in memory. The single shared
    /// implementation behind register_runnable and register_test —
    /// persistence is the caller's next step, ALWAYS attempted so that
    /// success means durable.
    ///
    /// - brand-new id: inserted
    /// - identical to the existing definition: no-op (restart flows and
    ///   retries after PersistFailed)
    /// - differs from a definition RESTORED FROM STORAGE: updated — the
    ///   embedder's code is the source of truth, and this is the normal
    ///   definition-evolved-between-versions upgrade path
    /// - differs from one registered EARLIER THIS SESSION: error — two
    ///   parts of one program disagreeing about a test is a bug
    fn ensure_definition(&mut self, definition: TestDefinition) -> Result<TestId, ManagerError> {
        // A definition the registry accepts must survive its own
        // persistence round-trip: duplicate metadata keys would serialize
        // to a JSON object the strict parser rejects.
        let mut meta_keys = std::collections::HashSet::new();
        for (k, _) in &definition.metadata {
            if !meta_keys.insert(k.as_str()) {
                return Err(ManagerError::RegistrationFailed(format!(
                    "duplicate metadata key '{}' in definition '{}'",
                    k, definition.id
                )));
            }
        }
        let id = definition.id.clone();
        match self.registry.get(&id) {
            Some(existing) if *existing == definition => {}
            Some(_) if self.session_defined.contains(&id) => {
                return Err(ManagerError::RegistrationFailed(format!(
                    "definition for '{}' conflicts with the one registered earlier this session",
                    id
                )));
            }
            Some(_) => {
                // Restored from storage with an older shape — update.
                // Keep the old definition restorable: a failed replacement
                // must not drop the previously-valid entry.
                let old = self.registry.deregister(&id);
                if let Err(e) = self.registry.register(definition) {
                    if let Some(old) = old {
                        let _ = self.registry.register(old);
                    }
                    return Err(ManagerError::RegistrationFailed(format!("{:?}", e)));
                }
            }
            None => {
                self.registry
                    .register(definition)
                    .map_err(|e| ManagerError::RegistrationFailed(format!("{:?}", e)))?;
            }
        }
        self.session_defined.insert(id.clone());
        Ok(id)
    }

    /// Register a runnable test implementation alongside its definition.
    ///
    /// Idempotent for an identical definition: re-registering (a restart
    /// after load_from_storage, or a retry after PersistFailed) keeps the
    /// already-attached runnable, re-attempts persistence, and reports
    /// success only when the definition is durable. A conflicting
    /// definition is an error.
    pub fn register_runnable(
        &mut self,
        definition: TestDefinition,
        runnable: Box<dyn RunnableTest>,
    ) -> Result<(), ManagerError> {
        if runnable.id() != definition.id {
            return Err(ManagerError::RegistrationFailed(
                "runnable ID does not match definition ID".into(),
            ));
        }
        let id = self.ensure_definition(definition)?;
        // REPLACE any already-attached runnable for this id: on a retry
        // or a redeploy with a fixed implementation, the freshly supplied
        // code must win — silently keeping stale code behind an Ok would
        // be worse than either erroring or replacing.
        if let Some(slot) = self.runnables.iter().position(|r| r.id() == id) {
            self.runnables[slot] = runnable;
        } else {
            self.runnables.push(runnable);
        }
        // Persist unconditionally — "already in memory" does not mean
        // "already durable"; a retry after PersistFailed must reach disk
        // before reporting success.
        self.persist_registry()
            .map_err(|e| ManagerError::PersistFailed(id, e))?;
        Ok(())
    }

    /// Format a run summary for output.
    pub fn format_results(&self, run_id: &str, format: ReportFormat) -> Result<String, ManagerError> {
        let summary = self.get_results(run_id)?;
        Ok(self.reporter.format_summary(&summary, format))
    }

    /// Format current progress for output.
    pub fn format_progress(&self, run_id: &str, format: ReportFormat) -> Result<String, ManagerError> {
        let progress = self.check_progress(run_id)?;
        Ok(self.reporter.format_progress(&progress, format))
    }

    /// Load registry from JSON storage.
    ///
    /// Definitions already registered in memory are skipped — that is the
    /// normal restart flow (register_runnable persists every definition,
    /// so stored and live definitions overlap by design). Every valid
    /// stored definition is registered even when some entries are
    /// malformed.
    ///
    /// Returns Ok with per-entry warnings (empty when the load was clean) —
    /// a partially-dirty registry is still a successful load. Err is
    /// reserved for file-level failure (unreadable file, invalid JSON),
    /// where nothing was loaded.
    pub fn load_from_storage(&mut self) -> Result<Vec<String>, String> {
        // No file yet (first launch, or WASM) is not an error.
        if !storage::registry_exists(&self.storage) {
            return Ok(Vec::new());
        }
        let (tests, mut failures) = storage::load_registry(&self.storage)?;
        // Distinguish two kinds of duplicate: an id already in memory
        // BEFORE this load is the normal embedder-re-registered overlap
        // (skip silently); a second occurrence of an id WITHIN the file
        // is corruption whose data would silently vanish — report it.
        let mut seen_in_file: std::collections::HashSet<String> = std::collections::HashSet::new();
        for test in tests {
            let id = test.id.clone();
            if !seen_in_file.insert(id.clone()) {
                failures.push(format!(
                    "{}: duplicate id within registry.json (later definition ignored)",
                    id
                ));
                continue;
            }
            if self.registry.get(&id).is_some() {
                continue;
            }
            if let Err(e) = self.registry.register(test) {
                failures.push(format!("{}: {:?}", id, e));
            }
        }
        Ok(failures)
    }

    /// Get the storage paths for external use.
    pub fn storage_paths(&self) -> &StoragePaths {
        &self.storage
    }

    /// Persist the registry, MERGED with what the file holds RIGHT NOW:
    /// stored definitions not present in memory are preserved. This makes
    /// register-before-load restarts and SEQUENTIALLY interleaved sessions
    /// lossless; two sessions whose read-merge-write windows truly OVERLAP
    /// can still lose the later-loser's entries until that session
    /// persists again — full multi-writer safety would need file locking,
    /// deliberately out of scope for the zero-dependency JSON store.
    /// In-memory definitions win on id conflict. The file is re-read on
    /// every persist — correctness over speed; registering N tests costs
    /// N file reads, acceptable at test-registry sizes. A
    /// corrupt file — whole-file or individual entries — is backed up to
    /// a unique registry.json.corrupt-* name before the rewrite would
    /// destroy the evidence. Write errors are returned, never swallowed.
    /// (If a manager-level deregister is ever added, it must delete from
    /// the file explicitly — this merge would resurrect otherwise.)
    fn persist_registry(&mut self) -> Result<(), String> {
        let stored = if storage::registry_exists(&self.storage) {
            match storage::load_registry(&self.storage) {
                Ok((stored, entry_errors)) => {
                    // In-file duplicate ids are collapsed by the merge
                    // below (first wins) — like malformed entries, the
                    // data being dropped must survive as evidence.
                    let mut ids = std::collections::HashSet::new();
                    let has_dups = stored.iter().any(|t| !ids.insert(t.id.clone()));
                    if !entry_errors.is_empty() || has_dups {
                        let _ = storage::backup_corrupt_registry(&self.storage);
                    }
                    stored
                }
                Err(_) => {
                    // File-level corruption: preserve the evidence
                    // instead of silently overwriting it.
                    let _ = storage::backup_corrupt_registry(&self.storage);
                    Vec::new()
                }
            }
        } else {
            Vec::new()
        };
        let mut all: Vec<TestDefinition> =
            self.registry.list_all().into_iter().cloned().collect();
        // One set covers both filters: skip stored entries shadowed by
        // memory AND collapse in-file duplicate ids (first wins, matching
        // load_from_storage) so the corruption converges instead of being
        // written back verbatim forever.
        let mut written: std::collections::HashSet<String> =
            all.iter().map(|t| t.id.clone()).collect();
        all.extend(
            stored
                .into_iter()
                .filter(|t| written.insert(t.id.clone())),
        );
        let refs: Vec<&TestDefinition> = all.iter().collect();
        storage::save_registry(&self.storage, &refs)
    }

    fn persist_run(&self, summary: &RunSummary) -> Result<(), String> {
        storage::save_run_summary(&self.storage, summary)
    }

    fn next_run_id(&mut self) -> RunId {
        // Mint an id and CLAIM it by atomically creating its file — two
        // sessions racing on the same storage dir can never mint the same
        // id. The directory is scanned only on the first mint of a
        // session, and re-scanned after a failed claim (someone else got
        // there first) — never on the hot path once the counter is ahead.
        loop {
            if self.run_counter == 0 {
                self.run_counter = storage::max_existing_run_number(&self.storage);
            }
            self.run_counter += 1;
            let run_id = format!("run_{:04}", self.run_counter);
            if storage::reserve_run_file(&self.storage, &run_id) {
                return run_id;
            }
            // Claimed by another session: resync with disk, then retry.
            self.run_counter = self
                .run_counter
                .max(storage::max_existing_run_number(&self.storage));
        }
    }
}


impl TestManager for PlatformManager {
    fn discover(&self, query: &DiscoveryQuery) -> DiscoveryResult {
        let disc = RegistryDiscovery::new(&self.registry);
        disc.discover(query)
    }

    fn summary(&self) -> DiscoverySummary {
        let disc = RegistryDiscovery::new(&self.registry);
        disc.summary()
    }

    fn register_test(&mut self, definition: TestDefinition) -> Result<(), ManagerError> {
        let id = self.ensure_definition(definition)?;
        // Persist unconditionally — see register_runnable.
        self.persist_registry()
            .map_err(|e| ManagerError::PersistFailed(id, e))?;
        Ok(())
    }

    fn start_run(&mut self, config: RunConfig) -> Result<RunId, ManagerError> {
        // The sequential executor is the only implementation so far —
        // reject a parallel request instead of silently running sequentially.
        if let ExecutionModel::Parallel { .. } = config.execution_model {
            return Err(ManagerError::UnsupportedConfig(
                "parallel execution is not supported yet; \
                 use \"execution_model\": {\"type\": \"sequential\"}"
                    .into(),
            ));
        }

        // A programmatic exclude-only config with run_all=false selects
        // nothing by construction — point at the fix instead of a bare
        // no-tests-matched (the JSON layer already rejects this shape).
        if !config.run_all
            && !config.has_include_filters()
            && !config.exclude_tags.is_empty()
        {
            return Err(ManagerError::UnsupportedConfig(
                "exclude-only configs must set run_all: true (run everything \
                 except the excluded tags); run_all: false with no include \
                 filters selects nothing"
                    .into(),
            ));
        }

        // A misspelled EXCLUDE tag silently WIDENS the run (the exclusion
        // matches nothing and the tests it meant to skip execute) — the
        // mirror image of the include-typo checks below, and validated
        // even under run_all, where exclusions still apply.
        if !config.exclude_tags.is_empty() {
            let all_defs = self.registry.list_all();
            let unmatched: Vec<&String> = config
                .exclude_tags
                .iter()
                .filter(|tag| !all_defs.iter().any(|t| t.tags.contains(*tag)))
                .collect();
            if !unmatched.is_empty() {
                return Err(ManagerError::UnsupportedConfig(format!(
                    "exclude_tags {:?} match no registered test — a typo here \
                     would silently run the tests it meant to exclude",
                    unmatched
                )));
            }
        }

        // A misspelled include criterion must error, never silently shrink
        // the run while other criteria still match something. Skipped under
        // run_all, where include filters are documented as ignored.
        if !config.run_all {
            let unknown_ids: Vec<String> = config
                .include_ids
                .iter()
                .filter(|id| self.registry.get(id).is_none())
                .cloned()
                .collect();
            if !unknown_ids.is_empty() {
                return Err(ManagerError::UnknownTestIds(unknown_ids));
            }
            // Same symmetry for the other include criteria: a tag set or
            // pattern matching ZERO tests is a typo, not an empty union
            // contribution.
            let all_defs = self.registry.list_all();
            if !config.include_tags.is_empty()
                && !all_defs
                    .iter()
                    .any(|t| crate::filter::matches_all_tags(&config.include_tags, t))
            {
                return Err(ManagerError::UnsupportedConfig(format!(
                    "include_tags {:?} match no registered test",
                    config.include_tags
                )));
            }
            if let Some(ref pattern) = config.name_pattern {
                let pattern_lower = pattern.to_lowercase();
                if !all_defs
                    .iter()
                    .any(|t| crate::filter::name_matches_lower(&pattern_lower, &t.name))
                {
                    return Err(ManagerError::UnsupportedConfig(format!(
                        "name_pattern '{}' matches no registered test",
                        pattern
                    )));
                }
            }
        }

        // Collect selected test IDs upfront to avoid borrow conflicts
        let selected_ids: Vec<String> = {
            let all_defs = self.registry.list_all();
            let all_refs: Vec<&TestDefinition> = all_defs.into_iter().collect();
            let selected = self.filter.apply(&all_refs, &config);
            if selected.is_empty() {
                return Err(ManagerError::NoTestsMatched);
            }
            selected.iter().map(|t| t.id.clone()).collect()
        };

        let run_id = self.next_run_id();
        let total = selected_ids.len() as u32;

        self.progress.start_run(run_id.clone(), total);

        let selected_set: std::collections::HashSet<&str> =
            selected_ids.iter().map(|s| s.as_str()).collect();
        let runnables: Vec<&dyn RunnableTest> = self
            .runnables
            .iter()
            .filter(|r| selected_set.contains(r.id()))
            .map(|r| r.as_ref())
            .collect();

        let started_at = storage::now_ms();

        // Execute, streaming each result into the progress tracker as it
        // lands (disjoint field borrows: executor and runnables are borrowed
        // immutably, progress mutably).
        let progress = &mut self.progress;
        let mut results = self.executor.execute(
            &runnables,
            config.timeout_ms,
            config.fail_fast,
            &mut |result| {
                progress.test_completed(&run_id, result);
            },
        );

        // Selected definitions with no registered runnable cannot execute.
        // Surface each as an explicit Error result instead of silently
        // shrinking the run (the classic case: definitions restored from
        // storage after a restart, with no runnables re-attached).
        // Keyed on the runnables' REGISTERED ids, not the test_id inside
        // their results — a buggy runnable returning a mismatched test_id
        // must not double-count the run (phantom result + spurious ghost).
        let ran: std::collections::HashSet<&str> = runnables.iter().map(|r| r.id()).collect();
        let missing: Vec<String> = selected_ids
            .iter()
            .filter(|id| !ran.contains(id.as_str()))
            .cloned()
            .collect();
        for id in missing {
            let result = TestResult {
                test_id: id.clone(),
                status: TestStatus::Error,
                duration_ms: 0,
                message: Some(format!(
                    "no runnable registered for test '{}' (definition only)",
                    id
                )),
                stdout: None,
                stderr: None,
            };
            self.progress.test_completed(&run_id, &result);
            results.push(result);
        }

        self.progress.finish_run(&run_id);

        // Build summary
        let mut passed = 0u32;
        let mut failed = 0u32;
        let mut skipped = 0u32;
        let mut errored = 0u32;
        let mut total_duration = 0u64;

        for r in &results {
            // Saturate — a runnable reporting a pathological duration must
            // not panic (debug) or wrap (release) after tests already ran.
            total_duration = total_duration.saturating_add(r.duration_ms);
            match r.status {
                TestStatus::Passed => passed += 1,
                TestStatus::Failed => failed += 1,
                TestStatus::Skipped => skipped += 1,
                TestStatus::Error => errored += 1,
            }
        }

        let summary = RunSummary {
            run_id: run_id.clone(),
            config,
            results,
            total,
            passed,
            failed,
            skipped,
            errored,
            total_duration_ms: total_duration,
            started_at,
            // Real clock when available. Only on WASM (no clock, now_ms()==0)
            // fall back to started_at + duration so the pair still encodes
            // elapsed time — never on native, where self-reported per-test
            // durations could fabricate a completion time in the future.
            completed_at: match storage::now_ms() {
                0 => started_at.saturating_add(total_duration),
                now => now.max(started_at),
            },
        };

        // Surface a failed write instead of letting the user believe the
        // run persisted — the results stay queryable in memory either way.
        let persisted = self.persist_run(&summary);
        self.completed_runs.push(summary);
        if let Err(e) = persisted {
            return Err(ManagerError::PersistFailed(run_id, e));
        }

        Ok(run_id)
    }

    fn check_progress(&self, run_id: &str) -> Result<RunProgress, ManagerError> {
        self.progress
            .get_progress(run_id)
            .ok_or_else(|| ManagerError::UnknownRun(run_id.into()))
    }

    fn active_runs(&self) -> Vec<RunId> {
        self.progress.active_runs()
    }

    fn get_results(&self, run_id: &str) -> Result<RunSummary, ManagerError> {
        // Reject non-run-shaped ids before anything else — the storage
        // layer validates too, but rejecting here gives the typed
        // UnknownRun error without touching disk.
        if !storage::is_valid_run_id(run_id) {
            return Err(ManagerError::UnknownRun(run_id.into()));
        }
        // Check completed runs in memory first
        if let Some(summary) = self.completed_runs.iter().find(|s| s.run_id == run_id) {
            return Ok(summary.clone());
        }
        // Check if it's still running
        if self.progress.active_runs().contains(&run_id.to_string()) {
            return Err(ManagerError::RunInProgress(run_id.into()));
        }
        // Try loading from storage. A missing file is an unknown run; an
        // empty file is another session's reservation (running, or died
        // before persisting) — no results yet, not corruption; a read
        // failure is an access problem, not damage; only content that
        // fails to PARSE is corruption — telling the user the run never
        // happened, or that intact data is damaged, hides the real state.
        if !storage::run_summary_exists(&self.storage, run_id) {
            return Err(ManagerError::UnknownRun(run_id.into()));
        }
        if storage::run_summary_is_reserved_only(&self.storage, run_id) {
            return Err(ManagerError::RunNotPersisted(run_id.into()));
        }
        storage::load_run_summary(&self.storage, run_id).map_err(|e| match e {
            storage::RunLoadError::Io(msg) => ManagerError::ReadFailed(run_id.into(), msg),
            storage::RunLoadError::Parse(msg) => ManagerError::CorruptRun(run_id.into(), msg),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::RunnableTest;

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
                duration_ms: 5,
                message: if self.pass { None } else { Some("failed".into()) },
                stdout: Some("test output".into()),
                stderr: None,
            }
        }
    }

    #[test]
    fn full_lifecycle() {
        let dir = crate::test_util::temp_storage_dir("mgr-lifecycle");
        let mut mgr = PlatformManager::new(&dir);

        // Register tests
        mgr.register_runnable(
            TestDefinition {
                id: "t1".into(),
                name: "auth_basic".into(),
                tags: vec!["smoke".into()],
                group: Some("auth".into()),
                description: None,
                metadata: vec![],
            },
            Box::new(EchoTest { id: "t1".into(), pass: true }),
        ).unwrap();

        mgr.register_runnable(
            TestDefinition {
                id: "t2".into(),
                name: "auth_token".into(),
                tags: vec!["smoke".into()],
                group: Some("auth".into()),
                description: None,
                metadata: vec![],
            },
            Box::new(EchoTest { id: "t2".into(), pass: false }),
        ).unwrap();

        // Discover
        let summary = mgr.summary();
        assert_eq!(summary.total_tests, 2);

        // Run all
        let run_id = mgr.start_run(RunConfig::default()).unwrap();

        // Get results
        let results = mgr.get_results(&run_id).unwrap();
        assert_eq!(results.total, 2);
        assert_eq!(results.passed, 1);
        assert_eq!(results.failed, 1);

        // Results should be persisted to JSON
        let json_path = format!("{}/runs/{}.json", dir, run_id);
        let content = std::fs::read_to_string(&json_path).unwrap();
        assert!(content.contains("\"run_id\""));
        assert!(content.contains("\"test_id\": \"t1\""));

        // Cleanup
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn definition_only_tests_error_instead_of_vanishing() {
        let dir = crate::test_util::temp_storage_dir("mgr-ghost");
        let mut mgr = PlatformManager::new(&dir);
        mgr.register_runnable(
            TestDefinition {
                id: "t1".into(),
                name: "runnable".into(),
                tags: vec![],
                group: None,
                description: None,
                metadata: vec![],
            },
            Box::new(EchoTest { id: "t1".into(), pass: true }),
        ).unwrap();
        // Definition with no runnable attached.
        mgr.register_test(TestDefinition {
            id: "t2".into(),
            name: "definition_only".into(),
            tags: vec![],
            group: None,
            description: None,
            metadata: vec![],
        }).unwrap();

        let run_id = mgr.start_run(RunConfig::default()).unwrap();
        let results = mgr.get_results(&run_id).unwrap();
        // Both tests are accounted for: one executed, one explicit Error.
        assert_eq!(results.total, 2);
        assert_eq!(results.results.len(), 2);
        assert_eq!(results.passed, 1);
        assert_eq!(results.errored, 1);
        let ghost = results.results.iter().find(|r| r.test_id == "t2").unwrap();
        assert_eq!(ghost.status, TestStatus::Error);
        assert!(ghost.message.as_deref().unwrap().contains("no runnable"));
        // Progress reaches 100% and the run is not stuck active.
        let prog = mgr.check_progress(&run_id).unwrap();
        assert_eq!(prog.completed, 2);
        assert_eq!(prog.percent_complete, 100.0);
        assert!(mgr.active_runs().is_empty());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn run_id_path_traversal_is_rejected() {
        let dir = crate::test_util::temp_storage_dir("mgr-traversal");
        let mut mgr = PlatformManager::new(&dir);
        mgr.register_runnable(
            TestDefinition {
                id: "t1".into(),
                name: "t".into(),
                tags: vec![],
                group: None,
                description: None,
                metadata: vec![],
            },
            Box::new(EchoTest { id: "t1".into(), pass: true }),
        ).unwrap();
        mgr.start_run(RunConfig::default()).unwrap();
        // registry.json now exists next to runs/ — a crafted id must not
        // be able to read it (or any other file) through the runs path.
        for evil in ["../registry", "run_0001/../../registry", "..", "run_", "run_1x"] {
            match mgr.get_results(evil) {
                Err(ManagerError::UnknownRun(_)) => {}
                other => panic!("expected UnknownRun for {:?}, got {:?}", evil, other.map(|_| ())),
            }
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn register_then_load_preserves_definition_only_tests() {
        // Session 1 persists a runnable-backed test AND a definition-only
        // test. Session 2 registers its runnable BEFORE loading — the
        // merge-on-persist must not clobber the stored definition-only
        // test in between.
        let dir = crate::test_util::temp_storage_dir("mgr-preserve");
        let def_t1 = TestDefinition {
            id: "t1".into(),
            name: "runnable".into(),
            tags: vec![],
            group: None,
            description: None,
            metadata: vec![],
        };
        let mut mgr1 = PlatformManager::new(&dir);
        mgr1.register_runnable(def_t1.clone(), Box::new(EchoTest { id: "t1".into(), pass: true })).unwrap();
        mgr1.register_test(TestDefinition {
            id: "t2".into(),
            name: "definition_only".into(),
            tags: vec![],
            group: None,
            description: None,
            metadata: vec![],
        }).unwrap();

        // Restart in register-first order.
        let mut mgr2 = PlatformManager::new(&dir);
        mgr2.register_runnable(def_t1, Box::new(EchoTest { id: "t1".into(), pass: true })).unwrap();
        assert!(mgr2.load_from_storage().unwrap().is_empty());
        // t2 survived the register-first persist.
        assert_eq!(mgr2.summary().total_tests, 2);
        let run_id = mgr2.start_run(RunConfig::default()).unwrap();
        assert_eq!(mgr2.get_results(&run_id).unwrap().total, 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn identical_reregistration_still_persists() {
        // The no-op fast path must still write to disk: a retry after a
        // failed persist lands there and success must mean durable.
        let dir = crate::test_util::temp_storage_dir("mgr-fastpath");
        let def = TestDefinition {
            id: "d1".into(),
            name: "t".into(),
            tags: vec![],
            group: None,
            description: None,
            metadata: vec![],
        };
        let mut mgr = PlatformManager::new(&dir);
        mgr.register_test(def.clone()).unwrap();
        // Simulate the definition never having reached disk.
        std::fs::remove_file(format!("{}/registry.json", dir)).unwrap();
        // Identical re-registration takes the fast path — and must persist.
        mgr.register_test(def).unwrap();
        let content = std::fs::read_to_string(format!("{}/registry.json", dir)).unwrap();
        assert!(content.contains("\"d1\""));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn unreadable_run_file_reports_read_failed_not_corrupt() {
        // A directory in place of the run file makes the read fail with
        // an I/O error — an access problem, not evidence of damage.
        let dir = crate::test_util::temp_storage_dir("mgr-readfail");
        std::fs::create_dir_all(format!("{}/runs/run_0042.json", dir)).unwrap();
        let mgr = PlatformManager::new(&dir);
        match mgr.get_results("run_0042") {
            Err(ManagerError::ReadFailed(id, _)) => assert_eq!(id, "run_0042"),
            other => panic!("expected ReadFailed, got {:?}", other.map(|_| ())),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn register_test_tolerates_load_then_register_restart() {
        // Same restart symmetry as register_runnable: re-registering an
        // identical definition-only test after load_from_storage is a
        // no-op, and a conflicting one errors.
        let dir = crate::test_util::temp_storage_dir("mgr-regtest-restart");
        let def = TestDefinition {
            id: "d1".into(),
            name: "definition_only".into(),
            tags: vec![],
            group: None,
            description: None,
            metadata: vec![],
        };
        let mut mgr1 = PlatformManager::new(&dir);
        mgr1.register_test(def.clone()).unwrap();

        let mut mgr2 = PlatformManager::new(&dir);
        assert!(mgr2.load_from_storage().unwrap().is_empty());
        mgr2.register_test(def.clone()).unwrap();
        assert_eq!(mgr2.summary().total_tests, 1);
        let conflicting = TestDefinition { name: "renamed".into(), ..def };
        assert!(mgr2.register_test(conflicting).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn concurrent_session_registrations_survive_merge() {
        // Session A caches nothing: every persist merges against the file
        // as it is NOW, so a concurrent session's registrations survive
        // A's later persists.
        let dir = crate::test_util::temp_storage_dir("mgr-xsession");
        fn def(id: &str) -> TestDefinition {
            TestDefinition {
                id: id.into(),
                name: id.into(),
                tags: vec![],
                group: None,
                description: None,
                metadata: vec![],
            }
        }
        let mut a = PlatformManager::new(&dir);
        let mut b = PlatformManager::new(&dir);
        a.register_runnable(def("a1"), Box::new(EchoTest { id: "a1".into(), pass: true })).unwrap();
        b.register_runnable(def("b1"), Box::new(EchoTest { id: "b1".into(), pass: true })).unwrap();
        // A persists again after B wrote — B's b1 must survive.
        a.register_runnable(def("a2"), Box::new(EchoTest { id: "a2".into(), pass: true })).unwrap();

        let mut fresh = PlatformManager::new(&dir);
        assert!(fresh.load_from_storage().unwrap().is_empty());
        let ids: Vec<String> = {
            let mut v: Vec<String> = fresh
                .discover(&DiscoveryQuery::default())
                .tests
                .iter()
                .map(|t| t.id.clone())
                .collect();
            v.sort();
            v
        };
        assert_eq!(ids, vec!["a1", "a2", "b1"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn corrupt_registry_is_backed_up_not_silently_overwritten() {
        // Register-first restart over a file-level-corrupt registry must
        // preserve the corrupt file as evidence before rewriting.
        let dir = crate::test_util::temp_storage_dir("mgr-corruptreg");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(format!("{}/registry.json", dir), "{ truncated garb").unwrap();

        let mut mgr = PlatformManager::new(&dir);
        mgr.register_runnable(
            TestDefinition {
                id: "t1".into(),
                name: "t".into(),
                tags: vec![],
                group: None,
                description: None,
                metadata: vec![],
            },
            Box::new(EchoTest { id: "t1".into(), pass: true }),
        ).unwrap();
        // Evidence preserved under a unique .corrupt-* name, and the live
        // file is clean again.
        let backups: Vec<_> = std::fs::read_dir(&dir).unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().starts_with("registry.json.corrupt-"))
            .collect();
        assert_eq!(backups.len(), 1);
        let backup = std::fs::read_to_string(backups[0].path()).unwrap();
        assert_eq!(backup, "{ truncated garb");
        assert!(crate::json::parse_json(
            &std::fs::read_to_string(format!("{}/registry.json", dir)).unwrap()
        ).is_ok());
        // No temp files left behind by the atomic writer.
        let stray: Vec<_> = std::fs::read_dir(&dir).unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains(".tmp"))
            .collect();
        assert!(stray.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn reserved_only_run_file_reports_not_persisted() {
        // Another session claimed the id (empty reservation file) but has
        // not persisted a summary — running elsewhere or died mid-run:
        // "no results yet, and here is how to clear it", never corruption.
        let dir = crate::test_util::temp_storage_dir("mgr-reserved");
        std::fs::create_dir_all(format!("{}/runs", dir)).unwrap();
        std::fs::write(format!("{}/runs/run_0005.json", dir), "").unwrap();
        let mgr = PlatformManager::new(&dir);
        match mgr.get_results("run_0005") {
            Err(ManagerError::RunNotPersisted(_)) => {}
            other => panic!("expected RunNotPersisted, got {:?}", other.map(|_| ())),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn hostile_run_filename_cannot_poison_the_counter() {
        // A hand-crafted run_<u64::MAX>.json must not overflow every
        // later mint.
        let dir = crate::test_util::temp_storage_dir("mgr-hostile");
        std::fs::create_dir_all(format!("{}/runs", dir)).unwrap();
        std::fs::write(
            format!("{}/runs/run_18446744073709551615.json", dir),
            "{}",
        ).unwrap();
        let mut mgr = PlatformManager::new(&dir);
        mgr.register_runnable(
            TestDefinition {
                id: "t1".into(),
                name: "t".into(),
                tags: vec![],
                group: None,
                description: None,
                metadata: vec![],
            },
            Box::new(EchoTest { id: "t1".into(), pass: true }),
        ).unwrap();
        let run_id = mgr.start_run(RunConfig::default()).unwrap();
        assert_eq!(run_id, "run_0001");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn reregistration_replaces_the_runnable() {
        // Redeploying a fixed implementation under the same definition
        // must execute the NEW code, never silently keep the stale one.
        let dir = crate::test_util::temp_storage_dir("mgr-replace");
        let def = TestDefinition {
            id: "t1".into(),
            name: "t".into(),
            tags: vec![],
            group: None,
            description: None,
            metadata: vec![],
        };
        let mut mgr = PlatformManager::new(&dir);
        mgr.register_runnable(def.clone(), Box::new(EchoTest { id: "t1".into(), pass: false })).unwrap();
        mgr.register_runnable(def, Box::new(EchoTest { id: "t1".into(), pass: true })).unwrap();
        let run_id = mgr.start_run(RunConfig::default()).unwrap();
        let summary = mgr.get_results(&run_id).unwrap();
        assert_eq!(summary.passed, 1, "replacement runnable must execute");
        assert_eq!(summary.total, 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn failed_upgrade_restores_the_old_definition() {
        // A rejected replacement must leave the previously-valid
        // definition in place, not drop it.
        let dir = crate::test_util::temp_storage_dir("mgr-upgradefail");
        let good = TestDefinition {
            id: "t1".into(),
            name: "good".into(),
            tags: vec![],
            group: None,
            description: None,
            metadata: vec![],
        };
        let mut mgr1 = PlatformManager::new(&dir);
        mgr1.register_test(good.clone()).unwrap();

        let mut mgr2 = PlatformManager::new(&dir);
        assert!(mgr2.load_from_storage().unwrap().is_empty());
        // Upgrade attempt with duplicate metadata keys — rejected.
        let bad = TestDefinition {
            metadata: vec![("k".into(), "a".into()), ("k".into(), "b".into())],
            ..good
        };
        assert!(mgr2.register_test(bad).is_err());
        // The old definition survived the failed upgrade.
        assert_eq!(mgr2.summary().total_tests, 1);
        assert_eq!(mgr2.discover(&DiscoveryQuery::default()).tests[0].name, "good");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn exclude_tag_typo_errors_instead_of_widening() {
        let dir = crate::test_util::temp_storage_dir("mgr-excludetypo");
        let mut mgr = PlatformManager::new(&dir);
        mgr.register_runnable(
            TestDefinition {
                id: "t1".into(),
                name: "t".into(),
                tags: vec!["destructive".into()],
                group: None,
                description: None,
                metadata: vec![],
            },
            Box::new(EchoTest { id: "t1".into(), pass: true }),
        ).unwrap();
        let config = RunConfig {
            exclude_tags: vec!["destrutive".into()], // typo
            ..Default::default()
        };
        match mgr.start_run(config) {
            Err(ManagerError::UnsupportedConfig(msg)) => assert!(msg.contains("destrutive")),
            other => panic!("expected UnsupportedConfig, got {:?}", other.map(|_| ())),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn duplicate_metadata_keys_rejected_at_registration() {
        // A definition must survive its own persistence round-trip; the
        // strict parser rejects duplicate JSON keys, so registration must
        // reject them first.
        let dir = crate::test_util::temp_storage_dir("mgr-dupmeta");
        let mut mgr = PlatformManager::new(&dir);
        let def = TestDefinition {
            id: "t1".into(),
            name: "t".into(),
            tags: vec![],
            group: None,
            description: None,
            metadata: vec![("env".into(), "a".into()), ("env".into(), "b".into())],
        };
        match mgr.register_test(def) {
            Err(ManagerError::RegistrationFailed(msg)) => assert!(msg.contains("duplicate metadata key")),
            other => panic!("expected RegistrationFailed, got {:?}", other),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    struct SlowLiar;
    impl RunnableTest for SlowLiar {
        fn id(&self) -> &str {
            "t1"
        }
        fn run(&self, _timeout: Option<DurationMs>) -> TestResult {
            TestResult {
                test_id: "t1".into(),
                status: TestStatus::Passed,
                duration_ms: u64::MAX, // pathological reported duration
                message: None,
                stdout: None,
                stderr: None,
            }
        }
    }

    #[test]
    fn pathological_duration_round_trips_through_storage() {
        // The platform must be able to reload every file it writes: a
        // u64::MAX duration is clamped on write and loads back cleanly.
        let dir = crate::test_util::temp_storage_dir("mgr-bigdur");
        let mut mgr = PlatformManager::new(&dir);
        mgr.register_runnable(
            TestDefinition {
                id: "t1".into(),
                name: "t".into(),
                tags: vec![],
                group: None,
                description: None,
                metadata: vec![],
            },
            Box::new(SlowLiar),
        ).unwrap();
        let run_id = mgr.start_run(RunConfig::default()).unwrap();
        // A fresh manager reads the run purely from storage.
        let fresh = PlatformManager::new(&dir);
        let summary = fresh.get_results(&run_id).unwrap();
        assert_eq!(summary.passed, 1);
        assert_eq!(summary.results[0].duration_ms, 9_007_199_254_740_992);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn changed_definition_from_storage_is_upgraded() {
        // The normal evolution flow: the embedder's code changed a test's
        // definition between versions. load-then-register must update the
        // stored shape, attach the runnable, and run.
        let dir = crate::test_util::temp_storage_dir("mgr-evolve");
        let old_def = TestDefinition {
            id: "t1".into(),
            name: "t".into(),
            tags: vec!["a".into()],
            group: None,
            description: None,
            metadata: vec![],
        };
        let mut mgr1 = PlatformManager::new(&dir);
        mgr1.register_runnable(old_def.clone(), Box::new(EchoTest { id: "t1".into(), pass: true })).unwrap();

        let new_def = TestDefinition { tags: vec!["a".into(), "b".into()], ..old_def };
        let mut mgr2 = PlatformManager::new(&dir);
        assert!(mgr2.load_from_storage().unwrap().is_empty());
        mgr2.register_runnable(new_def, Box::new(EchoTest { id: "t1".into(), pass: true })).unwrap();
        let found = mgr2.discover(&DiscoveryQuery::default());
        assert_eq!(found.tests[0].tags, vec!["a".to_string(), "b".to_string()]);
        let run_id = mgr2.start_run(RunConfig::default()).unwrap();
        assert_eq!(mgr2.get_results(&run_id).unwrap().passed, 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn in_file_duplicate_ids_converge_on_persist() {
        // A register-first session must collapse hand-edit duplicates
        // (first wins) instead of writing them back forever.
        let dir = crate::test_util::temp_storage_dir("mgr-dupconverge");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            format!("{}/registry.json", dir),
            r#"[
              {"id": "t1", "name": "first", "tags": []},
              {"id": "t1", "name": "second", "tags": []}
            ]"#,
        ).unwrap();
        let mut mgr = PlatformManager::new(&dir);
        mgr.register_test(TestDefinition {
            id: "t2".into(),
            name: "session".into(),
            tags: vec![],
            group: None,
            description: None,
            metadata: vec![],
        }).unwrap();
        let content = std::fs::read_to_string(format!("{}/registry.json", dir)).unwrap();
        assert_eq!(content.matches("\"t1\"").count(), 1, "duplicate must collapse");
        assert!(content.contains("first"));
        assert!(content.contains("\"t2\""));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn zero_match_include_criteria_error() {
        let dir = crate::test_util::temp_storage_dir("mgr-zerocrit");
        let mut mgr = PlatformManager::new(&dir);
        mgr.register_runnable(
            TestDefinition {
                id: "t1".into(),
                name: "fast_test".into(),
                tags: vec!["fast".into()],
                group: None,
                description: None,
                metadata: vec![],
            },
            Box::new(EchoTest { id: "t1".into(), pass: true }),
        ).unwrap();
        // A typo'd tag unioned with a valid id must error, not shrink.
        let config = RunConfig {
            run_all: false,
            include_ids: vec!["t1".into()],
            include_tags: vec!["fastt".into()],
            ..Default::default()
        };
        match mgr.start_run(config) {
            Err(ManagerError::UnsupportedConfig(msg)) => assert!(msg.contains("fastt")),
            other => panic!("expected UnsupportedConfig, got {:?}", other.map(|_| ())),
        }
        // Same for a pattern matching nothing.
        let config = RunConfig {
            run_all: false,
            name_pattern: Some("nonexistent_zzz".into()),
            ..Default::default()
        };
        assert!(matches!(mgr.start_run(config), Err(ManagerError::UnsupportedConfig(_))));
        // Exclude-only programmatic configs get pointed at run_all: true.
        let config = RunConfig {
            run_all: false,
            exclude_tags: vec!["fast".into()],
            ..Default::default()
        };
        match mgr.start_run(config) {
            Err(ManagerError::UnsupportedConfig(msg)) => assert!(msg.contains("run_all")),
            other => panic!("expected UnsupportedConfig, got {:?}", other.map(|_| ())),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn corrupt_run_file_is_reported_as_corrupt_not_unknown() {
        let dir = crate::test_util::temp_storage_dir("mgr-corruptrun");
        std::fs::create_dir_all(format!("{}/runs", dir)).unwrap();
        std::fs::write(format!("{}/runs/run_0042.json", dir), "{ not json").unwrap();
        let mgr = PlatformManager::new(&dir);
        match mgr.get_results("run_0042") {
            Err(ManagerError::CorruptRun(id, _)) => assert_eq!(id, "run_0042"),
            other => panic!("expected CorruptRun, got {:?}", other.map(|_| ())),
        }
        // A genuinely absent run is still UnknownRun.
        match mgr.get_results("run_0099") {
            Err(ManagerError::UnknownRun(_)) => {}
            other => panic!("expected UnknownRun, got {:?}", other.map(|_| ())),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn concurrent_managers_do_not_reuse_run_ids() {
        let dir = crate::test_util::temp_storage_dir("mgr-concurrent");
        let def = TestDefinition {
            id: "t1".into(),
            name: "t".into(),
            tags: vec![],
            group: None,
            description: None,
            metadata: vec![],
        };
        // Both managers open BEFORE either has run — both seed counter 0.
        let mut a = PlatformManager::new(&dir);
        let mut b = PlatformManager::new(&dir);
        a.register_runnable(def.clone(), Box::new(EchoTest { id: "t1".into(), pass: true })).unwrap();
        b.register_runnable(def, Box::new(EchoTest { id: "t1".into(), pass: true })).unwrap();
        let first = a.start_run(RunConfig::default()).unwrap();
        let second = b.start_run(RunConfig::default()).unwrap();
        assert_ne!(first, second, "concurrent sessions minted the same run id");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn bare_run_all_false_is_no_tests_matched() {
        // Programmatic callers constructing RunConfig directly get the
        // plain empty-selection error; the friendly explanation lives at
        // the JSON parse layer where key presence is visible.
        let dir = crate::test_util::temp_storage_dir("mgr-barefalse");
        let mut mgr = PlatformManager::new(&dir);
        mgr.register_runnable(
            TestDefinition {
                id: "t1".into(),
                name: "t".into(),
                tags: vec![],
                group: None,
                description: None,
                metadata: vec![],
            },
            Box::new(EchoTest { id: "t1".into(), pass: true }),
        ).unwrap();
        let config = RunConfig { run_all: false, ..Default::default() };
        match mgr.start_run(config) {
            Err(ManagerError::NoTestsMatched) => {}
            other => panic!("expected NoTestsMatched, got {:?}", other.map(|_| ())),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    struct LyingTest;
    impl RunnableTest for LyingTest {
        fn id(&self) -> &str {
            "t1"
        }
        fn run(&self, _timeout: Option<DurationMs>) -> TestResult {
            TestResult {
                test_id: "other".into(), // deliberately mismatched
                status: TestStatus::Passed,
                duration_ms: 1,
                message: None,
                stdout: None,
                stderr: None,
            }
        }
    }

    #[test]
    fn mismatched_result_test_id_does_not_double_count() {
        // A runnable whose run() returns the wrong test_id must not
        // produce a phantom result + spurious ghost Error, or drive
        // progress completed above total.
        let dir = crate::test_util::temp_storage_dir("mgr-lying");
        let mut mgr = PlatformManager::new(&dir);
        mgr.register_runnable(
            TestDefinition {
                id: "t1".into(),
                name: "t".into(),
                tags: vec![],
                group: None,
                description: None,
                metadata: vec![],
            },
            Box::new(LyingTest),
        ).unwrap();
        let run_id = mgr.start_run(RunConfig::default()).unwrap();
        let summary = mgr.get_results(&run_id).unwrap();
        assert_eq!(summary.total, 1);
        assert_eq!(summary.results.len(), 1);
        let prog = mgr.check_progress(&run_id).unwrap();
        assert_eq!(prog.completed, 1);
        assert_eq!(prog.total, 1);
        // The progress bar renderer must not panic on any counts.
        let _ = mgr.format_progress(&run_id, ReportFormat::Text).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_then_register_order_works() {
        // The reverse restart order: restore definitions from storage
        // FIRST, then attach runnables — must not fail as a duplicate,
        // and the runnable must actually attach (no ghost Errors).
        let dir = crate::test_util::temp_storage_dir("mgr-loadfirst");
        let def = TestDefinition {
            id: "t1".into(),
            name: "t".into(),
            tags: vec![],
            group: None,
            description: None,
            metadata: vec![],
        };
        let mut mgr1 = PlatformManager::new(&dir);
        mgr1.register_runnable(def.clone(), Box::new(EchoTest { id: "t1".into(), pass: true })).unwrap();

        let mut mgr2 = PlatformManager::new(&dir);
        assert!(mgr2.load_from_storage().unwrap().is_empty());
        mgr2.register_runnable(def.clone(), Box::new(EchoTest { id: "t1".into(), pass: true })).unwrap();
        let run_id = mgr2.start_run(RunConfig::default()).unwrap();
        let summary = mgr2.get_results(&run_id).unwrap();
        assert_eq!(summary.passed, 1);
        assert_eq!(summary.errored, 0);

        // Re-registering the identical definition is idempotent (the
        // existing runnable is kept); a conflicting definition errors.
        assert!(mgr2.register_runnable(def.clone(), Box::new(EchoTest { id: "t1".into(), pass: true })).is_ok());
        let conflicting = TestDefinition { name: "renamed".into(), ..def };
        assert!(mgr2.register_runnable(conflicting, Box::new(EchoTest { id: "t1".into(), pass: true })).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn runnable_reregistration_still_persists() {
        // The idempotent path must still write to disk so a retry after a
        // failed persist actually recovers durability.
        let dir = crate::test_util::temp_storage_dir("mgr-runfastpath");
        let def = TestDefinition {
            id: "t1".into(),
            name: "t".into(),
            tags: vec![],
            group: None,
            description: None,
            metadata: vec![],
        };
        let mut mgr = PlatformManager::new(&dir);
        mgr.register_runnable(def.clone(), Box::new(EchoTest { id: "t1".into(), pass: true })).unwrap();
        // Simulate the definition never having reached disk.
        std::fs::remove_file(format!("{}/registry.json", dir)).unwrap();
        mgr.register_runnable(def, Box::new(EchoTest { id: "t1".into(), pass: true })).unwrap();
        let content = std::fs::read_to_string(format!("{}/registry.json", dir)).unwrap();
        assert!(content.contains("\"t1\""));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn run_all_ignores_stale_include_ids() {
        // Documented contract: include filters are ignored under run_all —
        // a stale id must not error the run.
        let dir = crate::test_util::temp_storage_dir("mgr-staleids");
        let mut mgr = PlatformManager::new(&dir);
        mgr.register_runnable(
            TestDefinition {
                id: "t1".into(),
                name: "t".into(),
                tags: vec![],
                group: None,
                description: None,
                metadata: vec![],
            },
            Box::new(EchoTest { id: "t1".into(), pass: true }),
        ).unwrap();
        let config = RunConfig {
            run_all: true,
            include_ids: vec!["removed_test".into()],
            ..Default::default()
        };
        let run_id = mgr.start_run(config).unwrap();
        assert_eq!(mgr.get_results(&run_id).unwrap().total, 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn restart_flow_with_duplicate_definitions_is_ok() {
        // The normal restart sequence: register runnables (which persists
        // their definitions), then load_from_storage. The stored duplicates
        // must be skipped silently, not reported as failures.
        let dir = crate::test_util::temp_storage_dir("mgr-restart");

        let def = TestDefinition {
            id: "t1".into(),
            name: "t".into(),
            tags: vec![],
            group: None,
            description: None,
            metadata: vec![],
        };

        // Session 1 persists the definition.
        let mut mgr1 = PlatformManager::new(&dir);
        mgr1.register_runnable(def.clone(), Box::new(EchoTest { id: "t1".into(), pass: true })).unwrap();

        // Session 2 re-registers, then loads — must succeed.
        let mut mgr2 = PlatformManager::new(&dir);
        mgr2.register_runnable(def, Box::new(EchoTest { id: "t1".into(), pass: true })).unwrap();
        assert!(mgr2.load_from_storage().unwrap().is_empty());
        assert_eq!(mgr2.summary().total_tests, 1);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn duplicate_ids_within_registry_file_are_reported() {
        let dir = crate::test_util::temp_storage_dir("mgr-filedupe");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            format!("{}/registry.json", dir),
            r#"[
              {"id": "t1", "name": "first_definition", "tags": []},
              {"id": "t1", "name": "conflicting_second", "tags": []}
            ]"#,
        ).unwrap();

        let mut mgr = PlatformManager::new(&dir);
        // The load succeeds (usable registry), the in-file duplicate is
        // reported as a warning, and the first definition wins.
        let warnings = mgr.load_from_storage().unwrap();
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("duplicate id within registry.json"));
        assert_eq!(mgr.summary().total_tests, 1);
        let found = mgr.discover(&DiscoveryQuery::default());
        assert_eq!(found.tests[0].name, "first_definition");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn corrupt_registry_entry_does_not_discard_the_rest() {
        let dir = crate::test_util::temp_storage_dir("mgr-corrupt");
        std::fs::create_dir_all(&dir).unwrap();
        // Two valid entries around one with a missing id.
        std::fs::write(
            format!("{}/registry.json", dir),
            r#"[
              {"id": "good1", "name": "first", "tags": []},
              {"name": "no_id_here", "tags": []},
              {"id": "good2", "name": "second", "tags": []}
            ]"#,
        ).unwrap();

        let mut mgr = PlatformManager::new(&dir);
        // The load succeeds; the corrupt entry is reported as a warning...
        let warnings = mgr.load_from_storage().unwrap();
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("entry 1"));
        // ...and both valid definitions loaded.
        assert_eq!(mgr.summary().total_tests, 2);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn run_counter_survives_restart() {
        let dir = crate::test_util::temp_storage_dir("mgr-counter");

        let def = TestDefinition {
            id: "t1".into(),
            name: "t".into(),
            tags: vec![],
            group: None,
            description: None,
            metadata: vec![],
        };

        let mut mgr1 = PlatformManager::new(&dir);
        mgr1.register_runnable(def.clone(), Box::new(EchoTest { id: "t1".into(), pass: true })).unwrap();
        let first = mgr1.start_run(RunConfig::default()).unwrap();
        assert_eq!(first, "run_0001");

        // A fresh manager over the same storage must not reuse run_0001.
        let mut mgr2 = PlatformManager::new(&dir);
        mgr2.register_runnable(def, Box::new(EchoTest { id: "t1".into(), pass: true })).unwrap();
        let second = mgr2.start_run(RunConfig::default()).unwrap();
        assert_eq!(second, "run_0002");
        // The first session's persisted run file is intact.
        assert!(std::fs::metadata(format!("{}/runs/run_0001.json", dir)).is_ok());
        assert!(std::fs::metadata(format!("{}/runs/run_0002.json", dir)).is_ok());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn timestamps_are_real_on_native() {
        let dir = crate::test_util::temp_storage_dir("mgr-clock");
        let mut mgr = PlatformManager::new(&dir);
        mgr.register_runnable(
            TestDefinition {
                id: "t1".into(),
                name: "t".into(),
                tags: vec![],
                group: None,
                description: None,
                metadata: vec![],
            },
            Box::new(EchoTest { id: "t1".into(), pass: true }),
        ).unwrap();
        let run_id = mgr.start_run(RunConfig::default()).unwrap();
        let summary = mgr.get_results(&run_id).unwrap();
        // Sanity floor: well past 2020-01-01 in epoch ms.
        assert!(summary.started_at > 1_577_836_800_000);
        assert!(summary.completed_at >= summary.started_at);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn parallel_execution_model_is_rejected() {
        let dir = crate::test_util::temp_storage_dir("mgr-parallel");
        let mut mgr = PlatformManager::new(&dir);
        mgr.register_runnable(
            TestDefinition {
                id: "t1".into(),
                name: "t".into(),
                tags: vec![],
                group: None,
                description: None,
                metadata: vec![],
            },
            Box::new(EchoTest { id: "t1".into(), pass: true }),
        ).unwrap();
        let config = RunConfig {
            execution_model: ExecutionModel::Parallel { max_concurrency: 8 },
            ..Default::default()
        };
        match mgr.start_run(config) {
            Err(ManagerError::UnsupportedConfig(msg)) => {
                assert!(msg.contains("parallel"));
            }
            other => panic!("expected UnsupportedConfig, got {:?}", other.map(|_| ())),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn run_with_filter() {
        let dir = crate::test_util::temp_storage_dir("mgr-filter");
        let mut mgr = PlatformManager::new(&dir);

        mgr.register_runnable(
            TestDefinition {
                id: "t1".into(),
                name: "fast_test".into(),
                tags: vec!["fast".into()],
                group: None,
                description: None,
                metadata: vec![],
            },
            Box::new(EchoTest { id: "t1".into(), pass: true }),
        ).unwrap();

        mgr.register_runnable(
            TestDefinition {
                id: "t2".into(),
                name: "slow_test".into(),
                tags: vec!["slow".into()],
                group: None,
                description: None,
                metadata: vec![],
            },
            Box::new(EchoTest { id: "t2".into(), pass: true }),
        ).unwrap();

        let config = RunConfig {
            run_all: false,
            include_tags: vec!["fast".into()],
            ..Default::default()
        };
        let run_id = mgr.start_run(config).unwrap();
        let results = mgr.get_results(&run_id).unwrap();
        assert_eq!(results.total, 1);
        assert_eq!(results.results[0].test_id, "t1");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
