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
    /// Stored-registry definitions not registered in memory, read at most
    /// once per session (None = file not read yet) and re-preserved on
    /// every persist so a register-before-load restart cannot clobber
    /// definition-only tests a previous session persisted.
    preserved_from_file: Option<Vec<TestDefinition>>,
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
            preserved_from_file: None,
        }
    }

    /// Register a runnable test implementation alongside its definition.
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
        // Load-then-register restart flow: if an identical definition was
        // already restored from storage, just attach the runnable to it.
        // A conflicting definition, or a second runnable for the same id,
        // is still an error.
        if let Some(existing) = self.registry.get(&definition.id) {
            if *existing != definition {
                return Err(ManagerError::RegistrationFailed(format!(
                    "definition for '{}' conflicts with the already-registered one",
                    definition.id
                )));
            }
            if self.runnables.iter().any(|r| r.id() == definition.id) {
                return Err(ManagerError::RegistrationFailed(format!(
                    "a runnable is already registered for '{}'",
                    definition.id
                )));
            }
            self.runnables.push(runnable);
            return Ok(());
        }
        let id = definition.id.clone();
        self.registry
            .register(definition)
            .map_err(|e| ManagerError::RegistrationFailed(format!("{:?}", e)))?;
        self.runnables.push(runnable);
        // Registered in memory either way; a failed write must be loud.
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

    /// Persist the registry, MERGED with what the file already holds:
    /// stored definitions not present in memory are preserved, so a
    /// register-before-load restart cannot clobber definition-only tests
    /// that a previous session persisted. In-memory definitions win on id
    /// conflict. The file is read at most once per session (cached in
    /// preserved_from_file) so bulk registration stays O(N). A corrupt
    /// file — whole-file or individual entries — is backed up to
    /// registry.json.corrupt before the rewrite would destroy the
    /// evidence. Write errors are returned, never swallowed. (If a
    /// manager-level deregister is ever added, it must delete from the
    /// file explicitly — this merge would resurrect otherwise.)
    fn persist_registry(&mut self) -> Result<(), String> {
        if self.preserved_from_file.is_none() {
            let stored = if storage::registry_exists(&self.storage) {
                match storage::load_registry(&self.storage) {
                    Ok((stored, entry_errors)) => {
                        if !entry_errors.is_empty() {
                            // Malformed entries would be dropped by the
                            // rewrite — keep the original as evidence.
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
            self.preserved_from_file = Some(stored);
        }
        let in_memory: std::collections::HashSet<String> = self
            .registry
            .list_all()
            .iter()
            .map(|t| t.id.clone())
            .collect();
        let mut all: Vec<TestDefinition> =
            self.registry.list_all().into_iter().cloned().collect();
        all.extend(
            self.preserved_from_file
                .as_ref()
                .expect("populated above")
                .iter()
                .filter(|t| !in_memory.contains(t.id.as_str()))
                .cloned(),
        );
        let refs: Vec<&TestDefinition> = all.iter().collect();
        storage::save_registry(&self.storage, &refs)
    }

    fn persist_run(&self, summary: &RunSummary) -> Result<(), String> {
        storage::save_run_summary(&self.storage, summary)
    }

    fn next_run_id(&mut self) -> RunId {
        // Re-scan persisted runs at mint time and then CLAIM the id by
        // atomically creating its file — two sessions racing on the same
        // storage dir can no longer mint the same id and clobber each
        // other. On a claimed id, advance and retry.
        loop {
            self.run_counter = self
                .run_counter
                .max(storage::max_existing_run_number(&self.storage));
            self.run_counter += 1;
            let run_id = format!("run_{:04}", self.run_counter);
            if storage::reserve_run_file(&self.storage, &run_id) {
                return run_id;
            }
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
        let id = definition.id.clone();
        self.registry
            .register(definition)
            .map_err(|e| ManagerError::RegistrationFailed(format!("{:?}", e)))?;
        // Registered in memory either way; a failed write must be loud.
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

        // A misspelled include id must error, never silently shrink the
        // run while other criteria still match something. Skipped under
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
            total_duration += r.duration_ms;
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
        // before persisting) — no results yet, not corruption; a file with
        // content that fails to parse is CORRUPTION and must say so —
        // telling the user the run never happened hides real damage.
        if !storage::run_summary_exists(&self.storage, run_id) {
            return Err(ManagerError::UnknownRun(run_id.into()));
        }
        if storage::run_summary_is_reserved_only(&self.storage, run_id) {
            return Err(ManagerError::RunInProgress(run_id.into()));
        }
        storage::load_run_summary(&self.storage, run_id)
            .map_err(|e| ManagerError::CorruptRun(run_id.into(), e))
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
        // Evidence preserved, and the live file is clean again.
        let backup = std::fs::read_to_string(format!("{}/registry.json.corrupt", dir)).unwrap();
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
    fn reserved_only_run_file_reports_in_progress_not_corrupt() {
        // Another session claimed the id (empty reservation file) but has
        // not persisted a summary — that is "no results yet", never
        // corruption.
        let dir = crate::test_util::temp_storage_dir("mgr-reserved");
        std::fs::create_dir_all(format!("{}/runs", dir)).unwrap();
        std::fs::write(format!("{}/runs/run_0005.json", dir), "").unwrap();
        let mgr = PlatformManager::new(&dir);
        match mgr.get_results("run_0005") {
            Err(ManagerError::RunInProgress(_)) => {}
            other => panic!("expected RunInProgress, got {:?}", other.map(|_| ())),
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

        // A second runnable for the same id is still rejected, as is a
        // conflicting definition.
        assert!(mgr2.register_runnable(def, Box::new(EchoTest { id: "t1".into(), pass: true })).is_err());
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
