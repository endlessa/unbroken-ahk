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

/// Writes a JSON string to a file path.
/// Creates parent directories as needed.
#[cfg(not(target_arch = "wasm32"))]
pub fn write_json_file(path: &str, content: &str) -> Result<(), String> {
    // Ensure parent directory exists
    if let Some(parent) = path.rsplit_once('/') {
        std::fs::create_dir_all(parent.0).map_err(|e| format!("create dir: {}", e))?;
    }
    std::fs::write(path, content).map_err(|e| format!("write file: {}", e))
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

/// Save the test registry to JSON.
pub fn save_registry(paths: &StoragePaths, tests: &[&TestDefinition]) -> Result<(), String> {
    let arr = JsonValue::Array(tests.iter().map(|t| t.to_json()).collect());
    write_json_file(&paths.registry_path(), &to_json_pretty(&arr))
}

/// Load the test registry from JSON.
pub fn load_registry(paths: &StoragePaths) -> Result<Vec<TestDefinition>, String> {
    let content = read_json_file(&paths.registry_path())?;
    let value = parse_json(&content).map_err(|e| format!("{}", e))?;
    let arr = value.as_array().ok_or("expected JSON array")?;
    arr.iter()
        .map(|v| TestDefinition::from_json(v).map_err(|e| format!("{}", e)))
        .collect()
}

/// Save a run summary to JSON.
pub fn save_run_summary(paths: &StoragePaths, summary: &RunSummary) -> Result<(), String> {
    let json = to_json_pretty(&summary.to_json());
    write_json_file(&paths.run_path(&summary.run_id), &json)
}

/// Load a run summary from JSON.
pub fn load_run_summary(paths: &StoragePaths, run_id: &str) -> Result<RunSummary, String> {
    let content = read_json_file(&paths.run_path(run_id))?;
    let value = parse_json(&content).map_err(|e| format!("{}", e))?;
    RunSummary::from_json(&value).map_err(|e| format!("{}", e))
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
