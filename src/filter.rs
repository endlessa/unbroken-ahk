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

/// The include_tags predicate: a test matches when it carries ALL of the
/// requested tags. Shared by the filter engine and the manager's
/// zero-match validation so what is validated is exactly what selects.
pub fn matches_all_tags(tags: &[String], test: &TestDefinition) -> bool {
    tags.iter().all(|tag| test.tags.contains(tag))
}

/// Case-insensitive name matching shared by discovery search and run
/// filtering, so `discover <pattern>` and `run --pattern <pattern>`
/// always select the same tests. Takes an already-lowercased pattern so
/// callers matching one pattern against many names lowercase it once.
///
/// A pattern without '*' is a substring match. A pattern with '*'s gets
/// full glob semantics — "auth_*" (prefix), "*_ping" (suffix), "*auth*"
/// (contains), and interior stars like "net_*_v4" all follow one rule.
pub fn name_matches_lower(pattern_lower: &str, name: &str) -> bool {
    let name = name.to_lowercase();
    if !pattern_lower.contains('*') {
        return name.contains(pattern_lower);
    }
    glob_match_lower(pattern_lower, &name)
}

/// Glob over any number of '*'s: the text must start with the segment
/// before the first star, end with the segment after the last star, and
/// contain the interior segments in order between those anchors. An
/// interior star silently matching NOTHING (the old literal-substring
/// fallback) made "net_*_v4" return zero matches with no hint the shape
/// was unsupported.
fn glob_match_lower(pattern: &str, text: &str) -> bool {
    let segments: Vec<&str> = pattern.split('*').collect(); // >= 2 entries
    let first = segments[0];
    let last = segments[segments.len() - 1];
    if !text.starts_with(first)
        || text.len() < first.len() + last.len()
        || !text.ends_with(last)
    {
        return false;
    }
    let mut window = &text[first.len()..text.len() - last.len()];
    for segment in &segments[1..segments.len() - 1] {
        if segment.is_empty() {
            continue; // "**" constrains nothing extra
        }
        match window.find(segment) {
            Some(i) => window = &window[i + segment.len()..],
            None => return false,
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glob_shapes_all_follow_one_rule() {
        // The classic three...
        assert!(name_matches_lower("auth_*", "auth_basic"));
        assert!(!name_matches_lower("auth_*", "net_auth"));
        assert!(name_matches_lower("*_ping", "net_ping"));
        assert!(!name_matches_lower("*_ping", "ping_net"));
        assert!(name_matches_lower("*auth*", "basic_auth_v2"));
        // ...and interior stars, which used to silently match nothing.
        assert!(name_matches_lower("net_*_v4", "net_ping_v4"));
        assert!(!name_matches_lower("net_*_v4", "net_ping_v6"));
        assert!(name_matches_lower("a*b*c", "a_x_b_y_c"));
        assert!(!name_matches_lower("a*b*c", "a_x_c_y_b"));
        // Anchors must not overlap: "ab" cannot satisfy "ab*ab".
        assert!(!name_matches_lower("ab*ab", "ab"));
        assert!(name_matches_lower("ab*ab", "abab"));
        // No star: plain substring, case-insensitive via lowered name.
        assert!(name_matches_lower("ping", "Net_Ping"));
    }
}
