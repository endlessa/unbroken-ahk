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

/// Writes a JSON string to a file path, atomically: the content lands in
/// a uniquely-named temp file first and is renamed into place, so a crash
/// mid-write can never leave a truncated JSON file behind and concurrent
/// writers never share a temp file. Creates parent directories as needed.
#[cfg(not(target_arch = "wasm32"))]
pub fn write_json_file(path: &str, content: &str) -> Result<(), String> {
    ensure_parent_dir(path)?;
    let tmp = format!("{}.tmp{}", path, unique_suffix());
    std::fs::write(&tmp, content).map_err(|e| {
        // Clean up the partial temp file (disk-full, quota) — unique
        // names mean a leaked file would never be reused or overwritten.
        let _ = std::fs::remove_file(&tmp);
        format!("write file: {}", e)
    })?;
    std::fs::rename(&tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        format!("rename into place: {}", e)
    })
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

/// Whether a persisted registry file exists. Typed check instead of
/// string-matching error text (which breaks on Windows locale messages
/// and the WASM stub).
#[cfg(not(target_arch = "wasm32"))]
pub fn registry_exists(paths: &StoragePaths) -> bool {
    std::fs::metadata(paths.registry_path()).is_ok()
}

/// WASM stub: no persisted registry.
#[cfg(target_arch = "wasm32")]
pub fn registry_exists(_paths: &StoragePaths) -> bool {
    false
}

/// Save the test registry to JSON.
pub fn save_registry(paths: &StoragePaths, tests: &[&TestDefinition]) -> Result<(), String> {
    let arr = JsonValue::Array(tests.iter().map(|t| t.to_json()).collect());
    write_json_file(&paths.registry_path(), &to_json_pretty(&arr))
}

/// Load the test registry from JSON.
///
/// Returns every definition that parsed cleanly plus a description of each
/// entry that did not — one malformed entry must not discard the rest of
/// the registry. The outer error is reserved for file-level problems
/// (missing file, unparseable JSON, not an array).
pub fn load_registry(
    paths: &StoragePaths,
) -> Result<(Vec<TestDefinition>, Vec<String>), String> {
    let content = read_json_file(&paths.registry_path())?;
    let value = parse_json(&content).map_err(|e| format!("{}", e))?;
    let arr = value.as_array().ok_or("expected JSON array")?;
    let mut tests = Vec::new();
    let mut entry_errors = Vec::new();
    for (i, v) in arr.iter().enumerate() {
        match TestDefinition::from_json(v) {
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

/// Whether a persisted summary exists for this run id.
#[cfg(not(target_arch = "wasm32"))]
pub fn run_summary_exists(paths: &StoragePaths, run_id: &str) -> bool {
    is_valid_run_id(run_id) && std::fs::metadata(paths.run_path(run_id)).is_ok()
}

/// WASM stub: no persisted runs.
#[cfg(target_arch = "wasm32")]
pub fn run_summary_exists(_paths: &StoragePaths, _run_id: &str) -> bool {
    false
}

/// Whether a run file is a reservation placeholder only (exists, empty):
/// the run was claimed by some session but its summary has not been
/// written — still executing, or that session died mid-run. Either way
/// it is "no results yet", never corruption.
#[cfg(not(target_arch = "wasm32"))]
pub fn run_summary_is_reserved_only(paths: &StoragePaths, run_id: &str) -> bool {
    is_valid_run_id(run_id)
        && std::fs::metadata(paths.run_path(run_id))
            .map(|m| m.len() == 0)
            .unwrap_or(false)
}

/// WASM stub: no filesystem, no reservations.
#[cfg(target_arch = "wasm32")]
pub fn run_summary_is_reserved_only(_paths: &StoragePaths, _run_id: &str) -> bool {
    false
}

/// Atomically claim a run id by creating its file with create_new —
/// fails when another session already claimed it. Returns false ONLY on
/// already-claimed; other IO errors report true (best effort — the later
/// save will surface them) so callers can never loop forever.
#[cfg(not(target_arch = "wasm32"))]
pub fn reserve_run_file(paths: &StoragePaths, run_id: &str) -> bool {
    if !is_valid_run_id(run_id) {
        return false;
    }
    let path = paths.run_path(run_id);
    // Best effort — a create_dir failure surfaces via the open below.
    let _ = ensure_parent_dir(&path);
    match std::fs::OpenOptions::new().write(true).create_new(true).open(&path) {
        Ok(_) => true,
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => false,
        // Some other IO error: if the file turns out to exist, someone
        // else claimed it — treat as taken so the caller retries the next
        // id; only when it genuinely does not exist do we proceed
        // best-effort (uniqueness degrades while storage itself errors,
        // and the later save will surface the underlying problem).
        Err(_) => std::fs::metadata(&path).is_err(),
    }
}

/// WASM stub: no filesystem, no contention.
#[cfg(target_arch = "wasm32")]
pub fn reserve_run_file(_paths: &StoragePaths, _run_id: &str) -> bool {
    true
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
    /// The file could not be read (permissions, transient I/O, deleted
    /// between checks). Says nothing about the data.
    Io(String),
    /// The file was read but its content does not parse as a valid
    /// summary — the record is damaged or version-incompatible.
    Parse(String),
}

/// Load a run summary from JSON.
pub fn load_run_summary(paths: &StoragePaths, run_id: &str) -> Result<RunSummary, RunLoadError> {
    if !is_valid_run_id(run_id) {
        return Err(RunLoadError::Io(format!("invalid run id '{}'", run_id)));
    }
    let content = read_json_file(&paths.run_path(run_id)).map_err(RunLoadError::Io)?;
    let value = parse_json(&content).map_err(|e| RunLoadError::Parse(format!("{}", e)))?;
    RunSummary::from_json(&value).map_err(|e| RunLoadError::Parse(format!("{}", e)))
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
