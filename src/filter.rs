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
    /// When `config.run_all` is true, starts from everything — include
    /// filters are ignored, but exclude_tags STILL applies.
    /// Otherwise applies include/exclude filters in order:
    /// 1. Include by ID (if any specified)
    /// 2. Include by tags (if any specified)
    /// 3. Include by name pattern (if specified)
    /// 4. Exclude by tags (always applied, including under run_all)
    ///
    /// run_all=false with no include criteria selects nothing — exclusions
    /// alone never resurrect the full suite.
    fn apply<'a>(
        &self,
        tests: &[&'a TestDefinition],
        config: &RunConfig,
    ) -> Vec<&'a TestDefinition>;
}

/// Case-insensitive name matching shared by discovery search and run
/// filtering, so `discover <pattern>` and `run --pattern <pattern>`
/// always select the same tests. Takes an already-lowercased pattern so
/// callers matching one pattern against many names lowercase it once.
///
/// Supports simple globs: "auth_*" (prefix), "*_ping" (suffix),
/// "*auth*" (contains), otherwise substring match.
pub fn name_matches_lower(pattern_lower: &str, name: &str) -> bool {
    let name = name.to_lowercase();
    if let Some(rest) = pattern_lower.strip_prefix('*') {
        if let Some(middle) = rest.strip_suffix('*') {
            return name.contains(middle);
        }
        return name.ends_with(rest);
    }
    if let Some(prefix) = pattern_lower.strip_suffix('*') {
        return name.starts_with(prefix);
    }
    name.contains(pattern_lower)
}
