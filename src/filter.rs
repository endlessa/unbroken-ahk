//! Filter Engine — selects which tests to run based on a RunConfig.
//!
//! Takes the full registry and a run configuration, produces the
//! subset of tests that should execute.

use crate::types::RunConfig;
use crate::types::TestDefinition;

/// Applies run configuration criteria against a set of test definitions
/// to produce the execution subset.
pub trait TestFilter {
    /// Given all available tests and a run config, return only the tests
    /// that should be executed.
    ///
    /// When `config.run_all` is true, returns everything.
    /// Otherwise applies include/exclude filters in order:
    /// 1. Include by ID (if any specified)
    /// 2. Include by tags (if any specified)
    /// 3. Include by name pattern (if specified)
    /// 4. Exclude by tags (always applied)
    fn apply<'a>(
        &self,
        tests: &[&'a TestDefinition],
        config: &RunConfig,
    ) -> Vec<&'a TestDefinition>;
}

/// Case-insensitive name matching shared by discovery search and run
/// filtering, so `discover <pattern>` and `run --pattern <pattern>`
/// always select the same tests.
///
/// Supports simple globs: "auth_*" (prefix), "*_ping" (suffix),
/// otherwise substring match.
pub fn name_matches(pattern: &str, name: &str) -> bool {
    let pattern = pattern.to_lowercase();
    let name = name.to_lowercase();
    if let Some(prefix) = pattern.strip_suffix('*') {
        name.starts_with(prefix)
    } else if let Some(suffix) = pattern.strip_prefix('*') {
        name.ends_with(suffix)
    } else {
        name.contains(&pattern)
    }
}
