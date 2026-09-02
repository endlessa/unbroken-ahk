//! JSON file storage for all platform state.
//!
//! Everything goes to disk as readable JSON so you can crack it open
//! and see exactly what's happening. Each concern gets its own file.

use crate::json::{parse_json, to_json_pretty, JsonValue, ToJson, FromJson};
use crate::types::{RunSummary, TestDefinition};

/// Storage paths for platform data.
pub struct StoragePaths {
    /// Base directory for all JSON files.
    pub base_dir: String,
}

impl StoragePaths {
    pub fn new(base_dir: &str) -> Self {
        Self {
            base_dir: base_dir.to_string(),
        }
    }

    pub fn registry_path(&self) -> String {
        format!("{}/registry.json", self.base_dir)
    }

    pub fn run_path(&self, run_id: &str) -> String {
        format!("{}/runs/{}.json", self.base_dir, run_id)
    }

    pub fn progress_path(&self) -> String {
        format!("{}/progress.json", self.base_dir)
    }

    pub fn runs_dir(&self) -> String {
        format!("{}/runs", self.base_dir)
    }
}

/// Monotonic per-process sequence for unique temp/backup file names —
/// keyed by (pid, seq) so neither concurrent processes nor concurrent
/// threads within one process can collide on the same scratch path.
#[cfg(not(target_arch = "wasm32"))]
fn unique_suffix() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    format!("{}-{}", std::process::id(), SEQ.fetch_add(1, Ordering::Relaxed))
}

/// Create the parent directory of a file path.
#[cfg(not(target_arch = "wasm32"))]
fn ensure_parent_dir(path: &str) -> Result<(), String> {
    if let Some((parent, _)) = path.rsplit_once('/') {
        std::fs::create_dir_all(parent).map_err(|e| format!("create dir: {}", e))?;
    }
    Ok(())
}

/// Writes a JSON string to a file path, atomically AND durably: the
/// content lands in a uniquely-named temp file, is fsynced, and is then
/// renamed into place — so neither a process crash nor a power loss can
/// leave a truncated JSON file behind, and concurrent writers never
/// share a temp file. Creates parent directories as needed.
#[cfg(not(target_arch = "wasm32"))]
pub fn write_json_file(path: &str, content: &str) -> Result<(), String> {
    use std::io::Write;
    ensure_parent_dir(path)?;
    let tmp = format!("{}.tmp{}", path, unique_suffix());
    let write_and_sync = || -> std::io::Result<()> {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(content.as_bytes())?;
        // rename() orders the DIRECTORY entry, not the data blocks: on
        // some journaling filesystems a power loss after the rename can
        // leave the new name pointing at zero-length data unless the
        // data was fsynced first.
        f.sync_all()
    };
    if let Err(e) = write_and_sync() {
        // Clean up the partial temp file (disk-full, quota) — unique
        // names mean a leaked file would never be reused or overwritten.
        let _ = std::fs::remove_file(&tmp);
        return Err(format!("write file: {}", e));
    }
    std::fs::rename(&tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        format!("rename into place: {}", e)
    })?;
    // Make the RENAME durable too: sync the parent directory entry.
    // Best effort — directory handles cannot be synced on every platform
    // (Windows), and the data blocks themselves are already safe.
    if let Some((parent, _)) = path.rsplit_once('/') {
        if let Ok(dir) = std::fs::File::open(parent) {
            let _ = dir.sync_all();
        }
    }
    Ok(())
}

/// Preserve a corrupt registry file as evidence before it would be
/// overwritten. Each incident gets a unique backup name, so a later
/// corruption never clobbers earlier evidence. Best effort — returns the
/// backup path when the copy succeeded.
#[cfg(not(target_arch = "wasm32"))]
pub fn backup_corrupt_registry(paths: &StoragePaths) -> Option<String> {
    let src = paths.registry_path();
    let dst = format!("{}.corrupt-{}", src, unique_suffix());
    std::fs::copy(&src, &dst).ok().map(|_| dst)
}

/// WASM stub: nothing to back up.
#[cfg(target_arch = "wasm32")]
pub fn backup_corrupt_registry(_paths: &StoragePaths) -> Option<String> {
    None
}

/// Reads a JSON string from a file path.
#[cfg(not(target_arch = "wasm32"))]
pub fn read_json_file(path: &str) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("read file: {}", e))
}

/// WASM stub: write to in-memory store (to be replaced with actual WASM storage).
#[cfg(target_arch = "wasm32")]
pub fn write_json_file(_path: &str, _content: &str) -> Result<(), String> {
    // In WASM, storage will go through the host via imports.
    // For now this is a no-op stub.
    Ok(())
}

/// WASM stub: read from in-memory store.
#[cfg(target_arch = "wasm32")]
pub fn read_json_file(_path: &str) -> Result<String, String> {
    Err("WASM storage not yet implemented".into())
}

/// Current wall-clock time in milliseconds since the Unix epoch.
#[cfg(not(target_arch = "wasm32"))]
pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// WASM stub: no wall clock available; timestamps are 0 until the host
/// provides a clock import.
#[cfg(target_arch = "wasm32")]
pub fn now_ms() -> u64 {
    0
}

/// Highest run number among already-persisted run summaries
/// (files named `run_NNNN.json` in the runs directory).
///
/// Used to seed the run counter so a new session never reuses —
/// and overwrites — a previous session's run IDs.
#[cfg(not(target_arch = "wasm32"))]
pub fn max_existing_run_number(paths: &StoragePaths) -> u64 {
    let mut max = 0u64;
    if let Ok(entries) = std::fs::read_dir(paths.runs_dir()) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if let Some(num) = name
                .strip_prefix("run_")
                .and_then(|rest| rest.strip_suffix(".json"))
                .and_then(|digits| digits.parse::<u64>().ok())
                // Ignore absurd numbers a malformed/hostile file could
                // carry (run_18446744073709551615.json) — seeding the
                // counter near u64::MAX would overflow every later mint.
                .filter(|n| *n < 1_000_000_000)
            {
                max = max.max(num);
            }
        }
    }
    max
}

/// WASM stub: no persisted runs to scan.
#[cfg(target_arch = "wasm32")]
pub fn max_existing_run_number(_paths: &StoragePaths) -> u64 {
    0
}

/// Whether a persisted registry file exists. Ok(false) ONLY on a clean
/// "not found": any other stat failure (EIO, EACCES, stale handle) says
/// nothing about existence and must surface as Err — treating it as
/// absence is how a later merge-less write clobbers intact data.
#[cfg(not(target_arch = "wasm32"))]
pub fn registry_exists(paths: &StoragePaths) -> Result<bool, String> {
    match std::fs::metadata(paths.registry_path()) {
        Ok(_) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(format!("{}", e)),
    }
}

/// WASM stub: no persisted registry.
#[cfg(target_arch = "wasm32")]
pub fn registry_exists(_paths: &StoragePaths) -> Result<bool, String> {
    Ok(false)
}

/// Save the test registry to JSON.
pub fn save_registry(paths: &StoragePaths, tests: &[&TestDefinition]) -> Result<(), String> {
    let arr = JsonValue::Array(tests.iter().map(|t| t.to_json()).collect());
    write_json_file(&paths.registry_path(), &to_json_pretty(&arr))
}

/// Why a registry file failed to load — I/O trouble (typed Io) says
/// nothing about the data and must never be treated as corruption.
#[derive(Debug)]
pub enum RegistryLoadError {
    /// The file could not be read (permissions, transient I/O).
    Io(String),
    /// The file was read but is not a valid registry document.
    Parse(String),
}

/// Load the test registry from JSON.
///
/// Returns every definition that parsed cleanly plus a description of each
/// entry that did not — one malformed entry must not discard the rest of
/// the registry. The outer error is reserved for file-level problems,
/// split into Io (retryable, data intact) and Parse (corruption).
pub fn load_registry(
    paths: &StoragePaths,
) -> Result<(Vec<TestDefinition>, Vec<String>), RegistryLoadError> {
    let content = read_json_file(&paths.registry_path()).map_err(RegistryLoadError::Io)?;
    // Split the array FIRST, then parse each element on its own: a
    // parse-level defect inside one entry (a duplicate object key, a
    // malformed number) is that entry's problem. Parsing the whole
    // document in one strict call would let one such entry discard every
    // healthy definition around it — exactly what per-entry tolerance
    // exists to prevent. Only structural damage to the array skeleton
    // itself is file-level corruption.
    let slices = crate::json::split_top_level_array(&content)
        .map_err(|e| RegistryLoadError::Parse(format!("{}", e)))?;
    let mut tests = Vec::new();
    let mut entry_errors = Vec::new();
    for (i, slice) in slices.iter().enumerate() {
        match parse_json(slice).and_then(|v| TestDefinition::from_json(&v)) {
            Ok(t) => tests.push(t),
            Err(e) => entry_errors.push(format!("entry {}: {}", i, e)),
        }
    }
    Ok((tests, entry_errors))
}

/// Run IDs are always minted as run_<digits>. Anything else cannot name a
/// real run — and must never reach the filesystem, where a crafted id
/// like "../registry" would escape the runs directory. Enforced HERE, at
/// the layer that builds the path, so every caller is covered.
pub fn is_valid_run_id(id: &str) -> bool {
    id.strip_prefix("run_")
        .map(|digits| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
        .unwrap_or(false)
}

/// Whether a persisted summary exists for this run id. Same contract as
/// registry_exists: Ok(false) only on a clean "not found" — a stat
/// failure must not make an existing run report as unknown.
#[cfg(not(target_arch = "wasm32"))]
pub fn run_summary_exists(paths: &StoragePaths, run_id: &str) -> Result<bool, String> {
    if !is_valid_run_id(run_id) {
        return Ok(false);
    }
    match std::fs::metadata(paths.run_path(run_id)) {
        Ok(_) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(format!("{}", e)),
    }
}

/// WASM stub: no persisted runs.
#[cfg(target_arch = "wasm32")]
pub fn run_summary_exists(_paths: &StoragePaths, _run_id: &str) -> Result<bool, String> {
    Ok(false)
}

/// Outcome of attempting to claim a run id.
#[derive(Debug)]
pub enum ReserveOutcome {
    /// The reservation file was atomically created — the id is ours.
    Claimed,
    /// Another session already holds this id — try the next number.
    Taken,
    /// Storage itself is erroring: the id is neither claimed nor
    /// known-taken. Proceeding anyway would let two sessions mint the
    /// SAME id during a transient error and silently overwrite each
    /// other's summaries later — the caller must surface this, not run.
    Failed(String),
}

/// Atomically claim a run id by creating its file with create_new —
/// Taken when another session already claimed it, Failed when storage
/// errors without proving either way.
#[cfg(not(target_arch = "wasm32"))]
pub fn reserve_run_file(paths: &StoragePaths, run_id: &str) -> ReserveOutcome {
    if !is_valid_run_id(run_id) {
        return ReserveOutcome::Failed(format!("invalid run id '{}'", run_id));
    }
    let path = paths.run_path(run_id);
    // Keep a create_dir failure for the diagnostic below: the open that
    // follows it fails with a misleading NotFound (parent missing), and
    // pointing the user at a phantom missing file instead of the real
    // permission/IO problem sends recovery the wrong way.
    let parent_err = ensure_parent_dir(&path).err();
    match std::fs::OpenOptions::new().write(true).create_new(true).open(&path) {
        Ok(_) => ReserveOutcome::Claimed,
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => ReserveOutcome::Taken,
        // Some other IO error: if the file demonstrably exists, someone
        // else claimed it between the attempts — advance to the next id.
        // Otherwise NOTHING is known; report the failure rather than
        // "claim" an id no reservation actually protects.
        Err(e) => {
            if std::fs::metadata(&path).is_ok() {
                ReserveOutcome::Taken
            } else {
                ReserveOutcome::Failed(match parent_err {
                    Some(pe) => {
                        format!("{} (runs directory could not be created: {})", e, pe)
                    }
                    None => format!("{}", e),
                })
            }
        }
    }
}

/// WASM stub: no filesystem, no contention.
#[cfg(target_arch = "wasm32")]
pub fn reserve_run_file(_paths: &StoragePaths, _run_id: &str) -> ReserveOutcome {
    ReserveOutcome::Claimed
}

/// Save a run summary to JSON.
pub fn save_run_summary(paths: &StoragePaths, summary: &RunSummary) -> Result<(), String> {
    if !is_valid_run_id(&summary.run_id) {
        return Err(format!("invalid run id '{}'", summary.run_id));
    }
    let json = to_json_pretty(&summary.to_json());
    write_json_file(&paths.run_path(&summary.run_id), &json)
}

/// Why a run summary failed to load — I/O trouble is NOT evidence of a
/// damaged record and callers must not report it as corruption.
#[derive(Debug)]
pub enum RunLoadError {
    /// No file exists for this run id — the run is unknown.
    NotFound,
    /// The file exists but is an empty reservation placeholder: the run
    /// was claimed by some session but its summary has not been written —
    /// still executing, or that session died mid-run. "No results yet",
    /// never corruption.
    ReservedOnly,
    /// The file could not be read (permissions, transient I/O). Says
    /// nothing about the data.
    Io(String),
    /// The file was read but its content does not parse as a valid
    /// summary — the record is damaged or version-incompatible.
    Parse(String),
}

/// Load a run summary from JSON. ONE read classifies every case —
/// separate exists/reserved stats before the read left TOCTOU windows
/// where a file removed between checks was reported as a retryable read
/// failure instead of the truthful "unknown run".
#[cfg(not(target_arch = "wasm32"))]
pub fn load_run_summary(paths: &StoragePaths, run_id: &str) -> Result<RunSummary, RunLoadError> {
    if !is_valid_run_id(run_id) {
        return Err(RunLoadError::NotFound);
    }
    let content = match std::fs::read_to_string(paths.run_path(run_id)) {
        Ok(content) => content,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(RunLoadError::NotFound)
        }
        Err(e) => return Err(RunLoadError::Io(format!("read file: {}", e))),
    };
    if content.trim().is_empty() {
        return Err(RunLoadError::ReservedOnly);
    }
    let value = parse_json(&content).map_err(|e| RunLoadError::Parse(format!("{}", e)))?;
    RunSummary::from_json(&value).map_err(|e| RunLoadError::Parse(format!("{}", e)))
}

/// WASM stub: no filesystem — nothing was ever persisted.
#[cfg(target_arch = "wasm32")]
pub fn load_run_summary(_paths: &StoragePaths, _run_id: &str) -> Result<RunSummary, RunLoadError> {
    Err(RunLoadError::NotFound)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storage_paths_format() {
        let paths = StoragePaths::new("/tmp/test-platform");
        assert_eq!(paths.registry_path(), "/tmp/test-platform/registry.json");
        assert_eq!(paths.run_path("run123"), "/tmp/test-platform/runs/run123.json");
    }
}
