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
}

impl PlatformManager {
    pub fn new(storage_dir: &str) -> Self {
        let storage = StoragePaths::new(storage_dir);
        // Resume the run counter from persisted runs so a new session never
        // reuses — and overwrites — a previous session's run IDs.
        let run_counter = storage::max_existing_run_number(&storage);
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
        self.registry
            .register(definition)
            .map_err(|e| ManagerError::RegistrationFailed(format!("{:?}", e)))?;
        self.runnables.push(runnable);
        self.persist_registry();
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
    /// malformed; an Err reports what was skipped or corrupt, after
    /// loading everything that could be loaded.
    pub fn load_from_storage(&mut self) -> Result<(), String> {
        // No file yet (first launch, or WASM) is not an error.
        if !storage::registry_exists(&self.storage) {
            return Ok(());
        }
        let (tests, mut failures) = storage::load_registry(&self.storage)?;
        for test in tests {
            // Already present in memory (typically re-registered by
            // the embedder before loading) — not an error.
            if self.registry.get(&test.id).is_some() {
                continue;
            }
            let id = test.id.clone();
            if let Err(e) = self.registry.register(test) {
                failures.push(format!("{}: {:?}", id, e));
            }
        }
        if failures.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "loaded registry with {} problem entr{}: {}",
                failures.len(),
                if failures.len() == 1 { "y" } else { "ies" },
                failures.join("; ")
            ))
        }
    }

    /// Get the storage paths for external use.
    pub fn storage_paths(&self) -> &StoragePaths {
        &self.storage
    }

    fn persist_registry(&self) {
        let all = self.registry.list_all();
        let _ = storage::save_registry(&self.storage, &all);
    }

    fn persist_run(&self, summary: &RunSummary) {
        let _ = storage::save_run_summary(&self.storage, summary);
    }

    fn next_run_id(&mut self) -> RunId {
        self.run_counter += 1;
        format!("run_{:04}", self.run_counter)
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
        self.registry
            .register(definition)
            .map_err(|e| ManagerError::RegistrationFailed(format!("{:?}", e)))?;
        self.persist_registry();
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

        let runnables: Vec<&dyn RunnableTest> = self
            .runnables
            .iter()
            .filter(|r| selected_ids.contains(&r.id().to_string()))
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
        let executed: Vec<&str> = results.iter().map(|r| r.test_id.as_str()).collect();
        let missing: Vec<String> = selected_ids
            .iter()
            .filter(|id| !executed.contains(&id.as_str()))
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
            // Real clock when available; on WASM (now_ms() == 0) fall back
            // to started_at + duration so the pair still encodes elapsed time.
            completed_at: storage::now_ms().max(started_at.saturating_add(total_duration)),
        };

        self.persist_run(&summary);
        self.completed_runs.push(summary);

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
        // Check completed runs in memory first
        if let Some(summary) = self.completed_runs.iter().find(|s| s.run_id == run_id) {
            return Ok(summary.clone());
        }
        // Check if it's still running
        if self.progress.active_runs().contains(&run_id.to_string()) {
            return Err(ManagerError::RunInProgress(run_id.into()));
        }
        // Try loading from storage
        storage::load_run_summary(&self.storage, run_id)
            .map_err(|_| ManagerError::UnknownRun(run_id.into()))
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
        assert!(mgr2.load_from_storage().is_ok());
        assert_eq!(mgr2.summary().total_tests, 1);

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
        let result = mgr.load_from_storage();
        // The corrupt entry is reported...
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("entry 1"));
        // ...but both valid definitions loaded.
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
