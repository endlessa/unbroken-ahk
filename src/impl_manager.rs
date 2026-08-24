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
        // Round-trip validity (empty id/name, duplicate metadata keys) is
        // enforced by InMemoryRegistry::register itself — the invariant
        // lives at the registry layer so direct registry users get it too.
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
        self.attach_runnable(&id, runnable);
        // Persist unconditionally — "already in memory" does not mean
        // "already durable"; a retry after PersistFailed must reach disk
        // before reporting success.
        self.persist_registry()
            .map_err(|e| ManagerError::PersistFailed(id, e))?;
        Ok(())
    }

    /// REPLACE any already-attached runnable for this id: on a retry or a
    /// redeploy with a fixed implementation, the freshly supplied code
    /// must win — silently keeping stale code behind an Ok would be worse
    /// than either erroring or replacing.
    fn attach_runnable(&mut self, id: &str, runnable: Box<dyn RunnableTest>) {
        if let Some(slot) = self.runnables.iter().position(|r| r.id() == id) {
            self.runnables[slot] = runnable;
        } else {
            self.runnables.push(runnable);
        }
    }

    /// Register many runnable tests with a SINGLE registry persist.
    ///
    /// The per-call re-read/merge/write that keeps persistence lossless
    /// makes one-at-a-time registration cost one file read each; bulk
    /// startup registration should come through here instead — same
    /// per-item semantics as register_runnable, one merge+write total.
    ///
    /// Failure behavior: id mismatches are detected before ANY item is
    /// applied. A definition conflict mid-batch stops the batch, and the
    /// items applied before it are persisted before the error returns —
    /// exactly the state per-item register_runnable calls would have
    /// left, never registered-in-memory-but-absent-from-disk.
    pub fn register_runnables(
        &mut self,
        items: Vec<(TestDefinition, Box<dyn RunnableTest>)>,
    ) -> Result<(), ManagerError> {
        if items.is_empty() {
            return Ok(());
        }
        for (definition, runnable) in &items {
            if runnable.id() != definition.id {
                return Err(ManagerError::RegistrationFailed(format!(
                    "runnable ID '{}' does not match definition ID '{}'",
                    runnable.id(),
                    definition.id
                )));
            }
        }
        let mut applied = 0usize;
        let mut batch_error = None;
        for (definition, runnable) in items {
            match self.ensure_definition(definition) {
                Ok(id) => {
                    self.attach_runnable(&id, runnable);
                    applied += 1;
                }
                Err(e) => {
                    batch_error = Some(e);
                    break;
                }
            }
        }
        if applied > 0 {
            // If both the batch and the persist fail, the persist failure
            // wins — durability of what DID apply is the more urgent news.
            self.persist_registry().map_err(|e| {
                ManagerError::PersistFailed(format!("registration batch ({} tests)", applied), e)
            })?;
        }
        match batch_error {
            Some(e) => Err(e),
            None => Ok(()),
        }
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
        // No file yet (first launch, or WASM) is not an error — but only
        // a CLEAN not-found says that; a stat failure is a read failure.
        match storage::registry_exists(&self.storage) {
            Ok(false) => return Ok(Vec::new()),
            Ok(true) => {}
            Err(msg) => return Err(format!("registry read failed (retryable): {}", msg)),
        }
        let (tests, mut failures) = storage::load_registry(&self.storage).map_err(|e| match e {
            storage::RegistryLoadError::Io(msg) => format!("registry read failed (retryable): {}", msg),
            storage::RegistryLoadError::Parse(msg) => format!("registry file is corrupt: {}", msg),
        })?;
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
        // A stat FAILURE must abort like the read-failure arm below —
        // "could not stat" is not "does not exist", and writing without
        // the merge would erase whatever the file holds.
        let file_present = storage::registry_exists(&self.storage)
            .map_err(|msg| format!("registry stat failed before merge: {}", msg))?;
        let stored = if file_present {
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
                // A transient READ failure says nothing about the data —
                // rewriting from an empty stored set would erase intact
                // definitions. Abort the persist instead; the caller sees
                // PersistFailed and can retry.
                Err(storage::RegistryLoadError::Io(msg)) => {
                    return Err(format!("registry read failed before merge: {}", msg));
                }
                Err(storage::RegistryLoadError::Parse(_)) => {
                    // Genuine file-level corruption: preserve the evidence
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

    fn next_run_id(&mut self) -> Result<RunId, ManagerError> {
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
            match storage::reserve_run_file(&self.storage, &run_id) {
                storage::ReserveOutcome::Claimed => return Ok(run_id),
                // Claimed by another session: resync with disk and retry.
                storage::ReserveOutcome::Taken => {
                    self.run_counter = self
                        .run_counter
                        .max(storage::max_existing_run_number(&self.storage));
                }
                // Storage erroring: an unprotected id could collide with
                // another session's and silently overwrite its summary —
                // refuse to start rather than run without a claim.
                storage::ReserveOutcome::Failed(msg) => {
                    return Err(ManagerError::RunStartFailed(format!(
                        "could not claim a run id ({}); no tests were \
                         executed — retry when storage recovers",
                        msg
                    )));
                }
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

        // run_all combined with include filters is contradictory caller
        // intent: honoring run_all would silently WIDEN the run past the
        // includes (running the destructive tests a tag was scoping out),
        // honoring the includes would silently ignore run_all. Same rule
        // as the sequential+max_concurrency contradiction: reject loudly
        // and name the fix.
        if config.run_all && config.has_include_filters() {
            return Err(ManagerError::UnsupportedConfig(
                "run_all: true conflicts with include filters (include_ids, \
                 include_tags, name_pattern) — drop run_all (include \
                 filters imply it is false) or drop the include filters"
                    .into(),
            ));
        }

        // A timeout above 2^53 cannot round-trip (the run-summary writer
        // clamps u64 fields), so accepting it would persist a different
        // timeout than the caller asked for. The JSON layer rejects it
        // for its callers; programmatic configs must hit the same wall.
        if config.timeout_ms.map_or(false, |t| t > crate::json_types::MAX_SAFE_JSON_INT as u64) {
            return Err(ManagerError::UnsupportedConfig(
                "timeout_ms exceeds the exact JSON integer range (2^53 - 1) \
                 and would not round-trip through the persisted summary"
                    .into(),
            ));
        }

        // An empty name_pattern substring-matches EVERY test. The JSON
        // layer rejects this for its callers; a programmatic RunConfig
        // (say, built from an empty UI field) must hit the same wall
        // instead of silently running the whole suite.
        if config.name_pattern.as_deref() == Some("") {
            return Err(ManagerError::UnsupportedConfig(
                "name_pattern is empty — an empty pattern would match \
                 every test; drop name_pattern or supply a real pattern"
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

        // ONE registry snapshot serves the exclude validation, include
        // validation, and selection below — three traversals of the same
        // data that must never drift apart.
        let all_defs = self.registry.list_all();

        // A misspelled EXCLUDE tag silently WIDENS the run (the exclusion
        // matches nothing and the tests it meant to skip execute) — the
        // mirror image of the include-typo checks below, and validated
        // even under run_all, where exclusions still apply.
        //
        // Deliberate tradeoff: this also fails a standing exclusion whose
        // tag was legitimately drained (last such test removed). For a
        // platform whose callers are AI agents and whose tags gate
        // destructive tests, a loud no-op beats a silent widening; the
        // error names the recovery for the drained-tag case.
        if !config.exclude_tags.is_empty() {
            let unmatched: Vec<&String> = config
                .exclude_tags
                .iter()
                .filter(|tag| !all_defs.iter().any(|t| t.tags.contains(*tag)))
                .collect();
            if !unmatched.is_empty() {
                return Err(ManagerError::UnsupportedConfig(format!(
                    "exclude_tags {:?} match no registered test — a typo here \
                     would silently run the tests it meant to exclude; if the \
                     tag was intentionally retired, remove it from exclude_tags",
                    unmatched
                )));
            }
        }

        // A misspelled include criterion must error, never silently shrink
        // the run while other criteria still match something. (Under
        // run_all include filters cannot appear at all — the contradiction
        // guard above rejected them.)
        if !config.run_all {
            // Validate against the SAME snapshot selection will use — a
            // second live-registry query here is both O(ids × registry)
            // and a chance for the two views to drift.
            let known_ids: std::collections::HashSet<&str> =
                all_defs.iter().map(|t| t.id.as_str()).collect();
            let unknown_ids: Vec<String> = config
                .include_ids
                .iter()
                .filter(|id| !known_ids.contains(id.as_str()))
                .cloned()
                .collect();
            if !unknown_ids.is_empty() {
                return Err(ManagerError::UnknownTestIds(unknown_ids));
            }
            // Same symmetry for the other include criteria: a tag set or
            // pattern matching ZERO tests is a typo, not an empty union
            // contribution.
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
            let selected = self.filter.apply(&all_defs, &config);
            if selected.is_empty() {
                return Err(ManagerError::NoTestsMatched);
            }
            selected.iter().map(|t| t.id.clone()).collect()
        };

        let run_id = self.next_run_id()?;
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

        // Clamp self-reported durations to the exact-JSON range HERE, at
        // ingestion, not only in the serializer: clamping only on write
        // would let this session serve u64::MAX from memory while every
        // other session reads the clamped value from disk — the same
        // run_id answering differently depending on who asks.
        let max_safe = crate::json_types::MAX_SAFE_JSON_INT as u64;
        for r in &mut results {
            r.duration_ms = r.duration_ms.min(max_safe);
        }

        // Build summary
        let mut passed = 0u32;
        let mut failed = 0u32;
        let mut skipped = 0u32;
        let mut errored = 0u32;
        let mut total_duration = 0u64;

        for r in &results {
            // Saturate — a runnable reporting a pathological duration must
            // not panic (debug) or wrap (release) after tests already ran.
            total_duration = total_duration.saturating_add(r.duration_ms).min(max_safe);
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
        if let Some(progress) = self.progress.get_progress(run_id) {
            return Ok(progress);
        }
        // Not tracked in this session. A run persisted by an earlier or
        // sibling session is still a KNOWN run — get_results can serve
        // it, so progress must agree rather than claim the id does not
        // exist. Serve the completed-run snapshot; the error cases
        // (unknown, reserved-only, unreadable, corrupt) pass through
        // with their own truthful messages.
        let summary = self.get_results(run_id)?;
        // completed comes from the RESULTS, not from total: a legacy file
        // (written before ghost-Error reconciliation) can hold fewer
        // results than selected tests, and claiming completed == total
        // would contradict the counters shown beside it. The inverse
        // damage (MORE results than total) is capped at total so the
        // snapshot never reports 150% — get_results is the diagnostic
        // channel for such a file, progress just must not lie.
        let completed = summary.results.len().min(summary.total as usize) as u32;
        let percent_complete = if summary.total > 0 {
            (completed as f64 / summary.total as f64) * 100.0
        } else {
            100.0
        };
        Ok(RunProgress {
            run_id: summary.run_id.clone(),
            total: summary.total,
            completed,
            passed: summary.passed,
            failed: summary.failed,
            errored: summary.errored,
            skipped: summary.skipped,
            running: 0,
            percent_complete,
            elapsed_ms: summary.completed_at.saturating_sub(summary.started_at),
            // A persisted summary IS a finished run — this flag, not
            // percent_complete, is the poll-until signal: a legacy file
            // with fewer results than total truthfully reports <100%
            // and would otherwise look permanently in-progress.
            finished: true,
        })
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
        match storage::run_summary_exists(&self.storage, run_id) {
            Ok(false) => return Err(ManagerError::UnknownRun(run_id.into())),
            Ok(true) => {}
            // A stat failure must not make an existing run "unknown".
            Err(msg) => return Err(ManagerError::ReadFailed(run_id.into(), msg)),
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
            TestDefinition { name: "runnable".into(), ..crate::test_util::def("t1") },
            Box::new(EchoTest { id: "t1".into(), pass: true }),
        ).unwrap();
        // Definition with no runnable attached.
        mgr.register_test(TestDefinition { name: "definition_only".into(), ..crate::test_util::def("t2") }).unwrap();

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
            TestDefinition { name: "t".into(), ..crate::test_util::def("t1") },
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
        let def_t1 = TestDefinition { name: "runnable".into(), ..crate::test_util::def("t1") };
        let mut mgr1 = PlatformManager::new(&dir);
        mgr1.register_runnable(def_t1.clone(), Box::new(EchoTest { id: "t1".into(), pass: true })).unwrap();
        mgr1.register_test(TestDefinition { name: "definition_only".into(), ..crate::test_util::def("t2") }).unwrap();

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
        let def = TestDefinition { name: "t".into(), ..crate::test_util::def("d1") };
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
        let def = TestDefinition { name: "definition_only".into(), ..crate::test_util::def("d1") };
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
        use crate::test_util::def;
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
    fn transient_read_failure_aborts_persist_instead_of_clobbering() {
        let dir = crate::test_util::temp_storage_dir("mgr-io-abort");
        use crate::test_util::def;
        let mut mgr = PlatformManager::new(&dir);
        mgr.register_runnable(def("t1"), Box::new(EchoTest { id: "t1".into(), pass: true }))
            .unwrap();
        // Swap the registry file for a directory: metadata() still
        // succeeds but reading fails — an I/O error, not corruption.
        let reg_path = format!("{}/registry.json", dir);
        std::fs::remove_file(&reg_path).unwrap();
        std::fs::create_dir(&reg_path).unwrap();
        match mgr.register_runnable(def("t2"), Box::new(EchoTest { id: "t2".into(), pass: true })) {
            Err(ManagerError::PersistFailed(id, msg)) => {
                assert_eq!(id, "t2");
                assert!(
                    msg.contains("registry read failed before merge"),
                    "unexpected persist error: {}",
                    msg
                );
            }
            other => panic!("expected PersistFailed, got {:?}", other),
        }
        // The unreadable path was left untouched: not overwritten, and
        // not backed up as "corrupt" (an I/O failure is not evidence).
        assert!(std::fs::metadata(&reg_path).unwrap().is_dir());
        let no_backups = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .all(|e| !e.file_name().to_string_lossy().contains("corrupt"));
        assert!(no_backups);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn register_runnables_batch_persists_all() {
        let dir = crate::test_util::temp_storage_dir("mgr-batch");
        use crate::test_util::def;
        let mut mgr = PlatformManager::new(&dir);
        mgr.register_runnables(vec![
            (def("b1"), Box::new(EchoTest { id: "b1".into(), pass: true }) as Box<dyn RunnableTest>),
            (def("b2"), Box::new(EchoTest { id: "b2".into(), pass: true })),
        ])
        .unwrap();
        // Both runnables are attached: the whole batch executes.
        let run_id = mgr.start_run(RunConfig::default()).unwrap();
        assert_eq!(mgr.get_results(&run_id).unwrap().total, 2);
        // And both definitions reached disk in the single persist.
        let mut fresh = PlatformManager::new(&dir);
        assert!(fresh.load_from_storage().unwrap().is_empty());
        assert_eq!(fresh.discover(&DiscoveryQuery::default()).tests.len(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn register_runnables_batch_id_mismatch_persists_nothing() {
        let dir = crate::test_util::temp_storage_dir("mgr-batch-mismatch");
        use crate::test_util::def;
        let mut mgr = PlatformManager::new(&dir);
        let err = mgr.register_runnables(vec![(
            def("b1"),
            Box::new(EchoTest { id: "WRONG".into(), pass: true }) as Box<dyn RunnableTest>,
        )]);
        assert!(matches!(err, Err(ManagerError::RegistrationFailed(_))));
        // The batch failed before its single persist — nothing on disk.
        assert!(std::fs::metadata(format!("{}/registry.json", dir)).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn mid_batch_conflict_persists_the_applied_prefix() {
        let dir = crate::test_util::temp_storage_dir("mgr-batch-prefix");
        use crate::test_util::def;
        let mut mgr = PlatformManager::new(&dir);
        // "x" is session-defined, so a conflicting redefinition must fail.
        mgr.register_runnable(def("x"), Box::new(EchoTest { id: "x".into(), pass: true }))
            .unwrap();
        let mut conflicting = def("x");
        conflicting.name = "renamed".into();
        let err = mgr.register_runnables(vec![
            (def("y"), Box::new(EchoTest { id: "y".into(), pass: true }) as Box<dyn RunnableTest>),
            (conflicting, Box::new(EchoTest { id: "x".into(), pass: true })),
        ]);
        assert!(matches!(err, Err(ManagerError::RegistrationFailed(_))));
        // "y" was applied before the conflict stopped the batch — it must
        // be durable, never registered-in-memory-but-absent-from-disk.
        let mut fresh = PlatformManager::new(&dir);
        assert!(fresh.load_from_storage().unwrap().is_empty());
        let ids: Vec<String> = fresh
            .discover(&DiscoveryQuery::default())
            .tests
            .iter()
            .map(|t| t.id.clone())
            .collect();
        assert!(ids.contains(&"x".to_string()) && ids.contains(&"y".to_string()));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn stat_failure_aborts_persist_instead_of_treating_as_absent() {
        use crate::test_util::def;
        let dir = crate::test_util::temp_storage_dir("mgr-stat-fail");
        // Make the storage path a FILE: stat of <dir>/registry.json then
        // fails with NotADirectory — an I/O failure, not "not found".
        // Treating it as absence would skip the merge and clobber.
        std::fs::write(&dir, "not a directory").unwrap();
        let mut mgr = PlatformManager::new(&dir);
        match mgr.register_runnable(def("t1"), Box::new(EchoTest { id: "t1".into(), pass: true })) {
            Err(ManagerError::PersistFailed(_, msg)) => {
                assert!(
                    msg.contains("registry stat failed before merge"),
                    "got: {}",
                    msg
                );
            }
            other => panic!("expected PersistFailed, got {:?}", other.map(|_| ())),
        }
        let _ = std::fs::remove_file(&dir);
    }

    #[test]
    fn run_stat_failure_is_read_failed_not_unknown() {
        let dir = crate::test_util::temp_storage_dir("mgr-run-stat");
        std::fs::create_dir_all(&dir).unwrap();
        // runs/ as a FILE: stat of runs/run_0001.json fails with
        // NotADirectory — the run's existence is UNKNOWABLE right now,
        // which must not be reported as "no run named ... exists".
        std::fs::write(format!("{}/runs", dir), "file").unwrap();
        let mgr = PlatformManager::new(&dir);
        match mgr.get_results("run_0001") {
            Err(ManagerError::ReadFailed(id, _)) => assert_eq!(id, "run_0001"),
            other => panic!("expected ReadFailed, got {:?}", other.map(|_| ())),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn progress_of_persisted_run_from_another_session_is_served() {
        use crate::test_util::def;
        let dir = crate::test_util::temp_storage_dir("mgr-xsession-progress");
        let mut a = PlatformManager::new(&dir);
        a.register_runnable(def("t1"), Box::new(EchoTest { id: "t1".into(), pass: true }))
            .unwrap();
        let run_id = a.start_run(RunConfig::default()).unwrap();

        // A fresh session must not claim the id "does not exist" while
        // get_results can serve it — progress answers with the final
        // snapshot instead.
        let mut b = PlatformManager::new(&dir);
        assert!(b.load_from_storage().unwrap().is_empty());
        let progress = b.check_progress(&run_id).unwrap();
        assert_eq!(progress.total, 1);
        assert_eq!(progress.completed, 1);
        assert_eq!(progress.passed, 1);
        assert_eq!(progress.running, 0);
        assert!((progress.percent_complete - 100.0).abs() < f64::EPSILON);
        // A genuinely unknown id still reports unknown.
        assert!(matches!(
            b.check_progress("run_9999"),
            Err(ManagerError::UnknownRun(_))
        ));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn legacy_short_results_progress_stays_consistent() {
        // A pre-ghost-reconciliation run file can hold fewer results
        // than selected tests; the synthetic snapshot must not claim
        // they all completed while the counters beside it say otherwise.
        let dir = crate::test_util::temp_storage_dir("mgr-legacy-progress");
        std::fs::create_dir_all(format!("{}/runs", dir)).unwrap();
        std::fs::write(
            format!("{}/runs/run_0001.json", dir),
            r#"{"run_id": "run_0001", "total": 5, "passed": 3,
                "results": [
                    {"test_id": "a", "status": "passed", "duration_ms": 1},
                    {"test_id": "b", "status": "passed", "duration_ms": 1},
                    {"test_id": "c", "status": "passed", "duration_ms": 1}
                ],
                "started_at": 1000, "completed_at": 4000}"#,
        )
        .unwrap();
        let mgr = PlatformManager::new(&dir);
        let progress = mgr.check_progress("run_0001").unwrap();
        assert_eq!(progress.total, 5);
        assert_eq!(progress.completed, 3);
        assert!((progress.percent_complete - 60.0).abs() < 1e-9);
        assert_eq!(progress.elapsed_ms, 3000);
        // The truthful <100% must not read as "still running": finished
        // is the poll-until signal, and a persisted summary IS finished.
        assert!(progress.finished);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn programmatic_empty_pattern_is_rejected() {
        // The JSON layer rejects an empty name_pattern; a programmatic
        // RunConfig must hit the same wall instead of silently running
        // the whole suite.
        let dir = crate::test_util::temp_storage_dir("mgr-empty-pattern");
        let mut mgr = PlatformManager::new(&dir);
        mgr.register_runnable(
            crate::test_util::def("t1"),
            Box::new(EchoTest { id: "t1".into(), pass: true }),
        )
        .unwrap();
        let config = RunConfig {
            run_all: false,
            name_pattern: Some(String::new()),
            ..Default::default()
        };
        match mgr.start_run(config) {
            Err(ManagerError::UnsupportedConfig(msg)) => {
                assert!(msg.contains("name_pattern is empty"), "got: {}", msg);
            }
            other => panic!("expected UnsupportedConfig, got {:?}", other),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn reservation_failure_refuses_to_start_instead_of_colliding() {
        // While storage errors, an id claimed "best effort" is protected
        // by nothing — two sessions could mint the same id and silently
        // overwrite each other's summaries. Refuse to start instead.
        let dir = crate::test_util::temp_storage_dir("mgr-reserve-fail");
        std::fs::create_dir_all(&dir).unwrap();
        // runs as a FILE: reservation create_new fails NotADirectory and
        // the file provably does not exist — a storage failure.
        std::fs::write(format!("{}/runs", dir), "file").unwrap();
        let mut mgr = PlatformManager::new(&dir);
        mgr.register_runnable(
            crate::test_util::def("t1"),
            Box::new(EchoTest { id: "t1".into(), pass: true }),
        )
        .unwrap();
        match mgr.start_run(RunConfig::default()) {
            Err(ManagerError::RunStartFailed(msg)) => {
                assert!(msg.contains("no tests were executed"), "got: {}", msg);
            }
            other => panic!("expected RunStartFailed, got {:?}", other),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn over_total_results_snapshot_caps_at_total() {
        // The inverse of the legacy short-results case: a damaged file
        // holding MORE results than its total must not yield 150%.
        let dir = crate::test_util::temp_storage_dir("mgr-over-progress");
        std::fs::create_dir_all(format!("{}/runs", dir)).unwrap();
        std::fs::write(
            format!("{}/runs/run_0001.json", dir),
            r#"{"run_id": "run_0001", "total": 2, "passed": 3,
                "results": [
                    {"test_id": "a", "status": "passed", "duration_ms": 1},
                    {"test_id": "b", "status": "passed", "duration_ms": 1},
                    {"test_id": "c", "status": "passed", "duration_ms": 1}
                ],
                "started_at": 1000, "completed_at": 2000}"#,
        )
        .unwrap();
        let mgr = PlatformManager::new(&dir);
        let progress = mgr.check_progress("run_0001").unwrap();
        assert_eq!(progress.completed, 2);
        assert!((progress.percent_complete - 100.0).abs() < 1e-9);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn parse_defect_in_one_registry_entry_spares_the_rest() {
        // A duplicate object key INSIDE one entry is that entry's damage:
        // the healthy definitions around it must load, and the next
        // persist must keep them in the live file (the damaged entry
        // survives as a .corrupt backup, not in silence).
        let dir = crate::test_util::temp_storage_dir("mgr-entry-defect");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            format!("{}/registry.json", dir),
            r#"[
                {"id": "good1", "name": "good1"},
                {"id": "bad", "name": "bad", "metadata": {"k": "a", "k": "b"}},
                {"id": "good2", "name": "good2"}
            ]"#,
        )
        .unwrap();
        let mut mgr = PlatformManager::new(&dir);
        let warnings = mgr.load_from_storage().unwrap();
        assert_eq!(warnings.len(), 1, "got: {:?}", warnings);
        assert!(warnings[0].contains("duplicate object key"), "got: {:?}", warnings);
        let loaded: Vec<String> = mgr
            .discover(&DiscoveryQuery::default())
            .tests
            .iter()
            .map(|t| t.id.clone())
            .collect();
        assert_eq!(loaded, vec!["good1", "good2"]);

        mgr.register_test(crate::test_util::def("new1"))
        .unwrap();
        let rewritten = std::fs::read_to_string(format!("{}/registry.json", dir)).unwrap();
        for id in ["good1", "good2", "new1"] {
            assert!(rewritten.contains(id), "{} missing after persist", id);
        }
        let has_backup = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .any(|e| e.file_name().to_string_lossy().contains("corrupt"));
        assert!(has_backup, "damaged entry must be preserved as evidence");
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
            TestDefinition { name: "t".into(), ..crate::test_util::def("t1") },
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
            Err(e @ ManagerError::RunNotPersisted(_)) => {
                // The user-facing rendering must carry the recovery
                // guidance, not just the variant name.
                let text = format!("{}", e);
                assert!(
                    text.contains("delete runs/run_0005.json"),
                    "guidance missing: {}",
                    text
                );
            }
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
            TestDefinition { name: "t".into(), ..crate::test_util::def("t1") },
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
        let def = TestDefinition { name: "t".into(), ..crate::test_util::def("t1") };
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
        let good = TestDefinition { name: "good".into(), ..crate::test_util::def("t1") };
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
            TestDefinition { name: "t".into(), ..crate::test_util::def("t1") },
            Box::new(SlowLiar),
        ).unwrap();
        let run_id = mgr.start_run(RunConfig::default()).unwrap();
        // The clamp happens at INGESTION, so the session that ran the
        // test serves the same values as everyone reading from disk —
        // never u64::MAX from memory and 2^53 from the file.
        let in_mem = mgr.get_results(&run_id).unwrap();
        assert_eq!(in_mem.results[0].duration_ms, 9_007_199_254_740_991);
        assert_eq!(in_mem.total_duration_ms, 9_007_199_254_740_991);
        // A fresh manager reads the run purely from storage.
        let fresh = PlatformManager::new(&dir);
        let summary = fresh.get_results(&run_id).unwrap();
        assert_eq!(summary.passed, 1);
        assert_eq!(summary.results[0].duration_ms, in_mem.results[0].duration_ms);
        assert_eq!(summary.total_duration_ms, in_mem.total_duration_ms);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn oversized_timeout_is_rejected_before_running() {
        // A timeout above 2^53 would persist clamped — a value the
        // caller never asked for. Reject up front, like the JSON layer.
        let dir = crate::test_util::temp_storage_dir("mgr-big-timeout");
        let mut mgr = PlatformManager::new(&dir);
        mgr.register_runnable(
            crate::test_util::def("t1"),
            Box::new(EchoTest { id: "t1".into(), pass: true }),
        )
        .unwrap();
        let config = RunConfig {
            timeout_ms: Some(u64::MAX),
            ..Default::default()
        };
        match mgr.start_run(config) {
            Err(ManagerError::UnsupportedConfig(msg)) => {
                assert!(msg.contains("timeout_ms exceeds"), "got: {}", msg);
            }
            other => panic!("expected UnsupportedConfig, got {:?}", other),
        }
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
        mgr.register_test(TestDefinition { name: "session".into(), ..crate::test_util::def("t2") }).unwrap();
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
        let def = TestDefinition { name: "t".into(), ..crate::test_util::def("t1") };
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
            TestDefinition { name: "t".into(), ..crate::test_util::def("t1") },
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
            TestDefinition { name: "t".into(), ..crate::test_util::def("t1") },
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
        let def = TestDefinition { name: "t".into(), ..crate::test_util::def("t1") };
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
        let def = TestDefinition { name: "t".into(), ..crate::test_util::def("t1") };
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
    fn run_all_with_include_filters_is_rejected() {
        // Contradictory intent: run_all would silently widen the run past
        // the includes, dropping the includes would silently ignore
        // run_all. Same rule as the sequential+max_concurrency
        // contradiction — reject, never half-honor.
        let dir = crate::test_util::temp_storage_dir("mgr-allplusinc");
        let mut mgr = PlatformManager::new(&dir);
        mgr.register_runnable(
            crate::test_util::def("t1"),
            Box::new(EchoTest { id: "t1".into(), pass: true }),
        ).unwrap();
        let config = RunConfig {
            run_all: true,
            include_ids: vec!["t1".into()],
            ..Default::default()
        };
        match mgr.start_run(config) {
            Err(ManagerError::UnsupportedConfig(msg)) => {
                assert!(msg.contains("conflicts with include filters"), "got: {}", msg);
            }
            other => panic!("expected UnsupportedConfig, got {:?}", other),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn restart_flow_with_duplicate_definitions_is_ok() {
        // The normal restart sequence: register runnables (which persists
        // their definitions), then load_from_storage. The stored duplicates
        // must be skipped silently, not reported as failures.
        let dir = crate::test_util::temp_storage_dir("mgr-restart");

        let def = TestDefinition { name: "t".into(), ..crate::test_util::def("t1") };

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

        let def = TestDefinition { name: "t".into(), ..crate::test_util::def("t1") };

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
            TestDefinition { name: "t".into(), ..crate::test_util::def("t1") },
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
            TestDefinition { name: "t".into(), ..crate::test_util::def("t1") },
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
