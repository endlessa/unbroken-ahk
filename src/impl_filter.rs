//! Concrete implementation of TestFilter.

use crate::filter::{name_matches_lower, TestFilter};
use crate::types::{RunConfig, TestDefinition};

/// Standard filter that applies RunConfig criteria in precedence order.
pub struct StandardFilter;

impl StandardFilter {
    pub fn new() -> Self {
        Self
    }
}

impl TestFilter for StandardFilter {
    fn apply<'a>(
        &self,
        tests: &[&'a TestDefinition],
        config: &RunConfig,
    ) -> Vec<&'a TestDefinition> {
        let no_includes = !config.has_include_filters();

        let mut candidates: Vec<&'a TestDefinition> = if config.run_all {
            // run_all: start from every test; exclusions below STILL apply —
            // "run everything except the destructive tag" must honor the
            // exclusion. Include filters are ignored here as defense in
            // depth, but start_run rejects that contradictory combination
            // before any config reaches this point.
            tests.to_vec()
        } else if no_includes {
            // run_all=false with no include criteria: the include side
            // selected nothing, so nothing runs (NoTestsMatched upstream) —
            // exclusions alone never resurrect the whole suite. Exclude-only
            // configs run everything-minus via run_all (its default stays
            // true when only exclude_tags is supplied).
            return Vec::new();
        } else {
            let mut candidates: Vec<&'a TestDefinition> = Vec::new();

            // Set-based dedup so a large registry stays O(n) per step.
            let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();

            // Step 1: Include by ID (set-based membership, O(n + m))
            if !config.include_ids.is_empty() {
                let wanted: std::collections::HashSet<&str> =
                    config.include_ids.iter().map(|s| s.as_str()).collect();
                for test in tests {
                    if wanted.contains(test.id.as_str()) && seen.insert(test.id.as_str()) {
                        candidates.push(test);
                    }
                }
            }

            // Step 2: Include by tags (additive — add tests matching ALL include_tags)
            if !config.include_tags.is_empty() {
                for test in tests {
                    if crate::filter::matches_all_tags(&config.include_tags, test)
                        && seen.insert(test.id.as_str())
                    {
                        candidates.push(test);
                    }
                }
            }

            // Step 3: Include by name pattern
            if let Some(ref pattern) = config.name_pattern {
                let pattern_lower = pattern.to_lowercase();
                for test in tests {
                    if name_matches_lower(&pattern_lower, &test.name)
                        && seen.insert(test.id.as_str())
                    {
                        candidates.push(test);
                    }
                }
            }

            candidates
        };

        // Exclude by tags — always applied, including under run_all.
        if !config.exclude_tags.is_empty() {
            candidates.retain(|test| {
                !config.exclude_tags.iter().any(|tag| test.tags.contains(tag))
            });
        }

        candidates
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn td(id: &str, name: &str, tags: &[&str]) -> TestDefinition {
        TestDefinition {
            id: id.into(),
            name: name.into(),
            tags: tags.iter().map(|s| s.to_string()).collect(),
            group: None,
            description: None,
            metadata: vec![],
        }
    }

    #[test]
    fn run_all_returns_everything() {
        let tests = vec![td("a", "a", &[]), td("b", "b", &[])];
        let refs: Vec<&TestDefinition> = tests.iter().collect();
        let config = RunConfig { run_all: true, ..Default::default() };
        let result = StandardFilter::new().apply(&refs, &config);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn include_by_id() {
        let tests = vec![td("a", "a", &[]), td("b", "b", &[]), td("c", "c", &[])];
        let refs: Vec<&TestDefinition> = tests.iter().collect();
        let config = RunConfig {
            run_all: false,
            include_ids: vec!["a".into(), "c".into()],
            ..Default::default()
        };
        let result = StandardFilter::new().apply(&refs, &config);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].id, "a");
        assert_eq!(result[1].id, "c");
    }

    #[test]
    fn exclude_by_tag() {
        let tests = vec![
            td("a", "a", &["fast"]),
            td("b", "b", &["slow"]),
            td("c", "c", &["fast"]),
        ];
        let refs: Vec<&TestDefinition> = tests.iter().collect();
        // Exclude-only selection rides on run_all (the parse-side default
        // when only exclude_tags is supplied).
        let config = RunConfig {
            run_all: true,
            exclude_tags: vec!["slow".into()],
            ..Default::default()
        };
        let result = StandardFilter::new().apply(&refs, &config);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn run_all_still_honors_exclusions() {
        let tests = vec![
            td("a", "a", &["fast"]),
            td("b", "b", &["slow"]),
        ];
        let refs: Vec<&TestDefinition> = tests.iter().collect();
        let config = RunConfig {
            run_all: true,
            exclude_tags: vec!["slow".into()],
            ..Default::default()
        };
        let result = StandardFilter::new().apply(&refs, &config);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "a");
    }

    #[test]
    fn explicit_run_all_false_with_no_filters_selects_nothing() {
        let tests = vec![td("a", "a", &[]), td("b", "b", &[])];
        let refs: Vec<&TestDefinition> = tests.iter().collect();
        let config = RunConfig { run_all: false, ..Default::default() };
        let result = StandardFilter::new().apply(&refs, &config);
        assert!(result.is_empty());
        // Exclusions alone never resurrect the suite when run_all is false.
        let config = RunConfig {
            run_all: false,
            exclude_tags: vec!["x".into()],
            ..Default::default()
        };
        let result = StandardFilter::new().apply(&refs, &config);
        assert!(result.is_empty());
    }

    #[test]
    fn contains_glob_matches() {
        use crate::filter::name_matches_lower;
        assert!(name_matches_lower("*auth*", "basic_auth_test"));
        assert!(name_matches_lower("*auth*", "auth_basic"));
        assert!(!name_matches_lower("*auth*", "network_ping"));
        // Bare "*" matches everything.
        assert!(name_matches_lower("*", "anything"));
    }

    #[test]
    fn include_steps_union_and_dedup() {
        // INVARIANT: include criteria are ADDITIVE — each step appends to
        // the candidates of the previous steps (never replaces them) — and
        // a test matching more than one criterion appears exactly once.
        let tests = vec![
            td("a", "a", &["fast"]),
            td("b", "b", &["fast"]),
            td("c", "c", &[]),
        ];
        let refs: Vec<&TestDefinition> = tests.iter().collect();
        // "a" matches BOTH include_ids and include_tags; "c" matches only
        // include_ids (no tags), so a replace-instead-of-append regression
        // in Step 2 would drop it.
        let config = RunConfig {
            run_all: false,
            include_ids: vec!["c".into(), "a".into()],
            include_tags: vec!["fast".into()],
            ..Default::default()
        };
        let result = StandardFilter::new().apply(&refs, &config);
        let ids: Vec<&str> = result.iter().map(|t| t.id.as_str()).collect();
        // Step 1 walks registry order (a then c), Step 2 appends b; "a"
        // is not pushed a second time by the tag step.
        assert_eq!(ids, vec!["a", "c", "b"]);
    }

    #[test]
    fn excludes_apply_to_include_selected_candidates() {
        // INVARIANT: exclude_tags prunes the include-selected candidates
        // too, not only the run_all population — "run the fast tests but
        // never the flaky ones" must drop a fast+flaky test.
        let tests = vec![
            td("a", "a", &["fast"]),
            td("b", "b", &["fast", "flaky"]),
        ];
        let refs: Vec<&TestDefinition> = tests.iter().collect();
        let config = RunConfig {
            run_all: false,
            include_tags: vec!["fast".into()],
            exclude_tags: vec!["flaky".into()],
            ..Default::default()
        };
        let result = StandardFilter::new().apply(&refs, &config);
        let ids: Vec<&str> = result.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids, vec!["a"]);
    }

    #[test]
    fn exclude_tags_are_any_of() {
        // INVARIANT: a test is excluded when it carries ANY one of the
        // exclude_tags — it need not carry all of them. An ALL-of
        // regression would keep running tests the config meant to exclude.
        let tests = vec![
            td("a", "a", &["slow"]),
            td("b", "b", &["flaky"]),
            td("c", "c", &["fast"]),
        ];
        let refs: Vec<&TestDefinition> = tests.iter().collect();
        let config = RunConfig {
            run_all: true,
            exclude_tags: vec!["slow".into(), "flaky".into()],
            ..Default::default()
        };
        let result = StandardFilter::new().apply(&refs, &config);
        let ids: Vec<&str> = result.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids, vec!["c"]);
    }

    #[test]
    fn include_tags_are_all_of() {
        // INVARIANT: include_tags is a conjunction — only tests carrying
        // ALL of the requested tags are selected, so multi-tag includes
        // never over-select tests carrying just one of the tags.
        let tests = vec![
            td("a", "a", &["smoke", "fast"]),
            td("b", "b", &["smoke"]),
        ];
        let refs: Vec<&TestDefinition> = tests.iter().collect();
        let config = RunConfig {
            run_all: false,
            include_tags: vec!["smoke".into(), "fast".into()],
            ..Default::default()
        };
        let result = StandardFilter::new().apply(&refs, &config);
        let ids: Vec<&str> = result.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids, vec!["a"]);
    }

    #[test]
    fn name_pattern_is_lowercased_before_matching() {
        // INVARIANT: the filter lowercases the caller's pattern before
        // handing it to name_matches_lower (whose contract requires a
        // pre-lowered pattern), so an uppercase pattern still selects.
        let tests = vec![
            td("a", "auth_basic", &[]),
            td("b", "network_ping", &[]),
        ];
        let refs: Vec<&TestDefinition> = tests.iter().collect();
        let config = RunConfig {
            run_all: false,
            name_pattern: Some("AUTH_*".into()),
            ..Default::default()
        };
        let result = StandardFilter::new().apply(&refs, &config);
        let ids: Vec<&str> = result.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids, vec!["a"]);
    }

    #[test]
    fn run_all_ignores_include_filters() {
        // INVARIANT: under run_all, include filters are ignored (defense
        // in depth for direct filter users — the manager rejects the
        // combination earlier): the full population is returned, never an
        // intersection with the includes.
        let tests = vec![td("a", "a", &[]), td("b", "b", &[])];
        let refs: Vec<&TestDefinition> = tests.iter().collect();
        let config = RunConfig {
            run_all: true,
            include_ids: vec!["a".into()],
            ..Default::default()
        };
        let result = StandardFilter::new().apply(&refs, &config);
        let ids: Vec<&str> = result.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids, vec!["a", "b"]);
    }

    #[test]
    fn name_pattern_glob() {
        let tests = vec![
            td("a", "auth_basic", &[]),
            td("b", "auth_token", &[]),
            td("c", "network_ping", &[]),
        ];
        let refs: Vec<&TestDefinition> = tests.iter().collect();
        let config = RunConfig {
            run_all: false,
            name_pattern: Some("auth_*".into()),
            ..Default::default()
        };
        let result = StandardFilter::new().apply(&refs, &config);
        assert_eq!(result.len(), 2);
    }
}
