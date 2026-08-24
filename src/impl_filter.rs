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
            // run_all: start from every test. Include filters are ignored,
            // but exclusions below STILL apply — "run everything except the
            // destructive tag" must honor the exclusion.
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

            // Step 1: Include by ID
            if !config.include_ids.is_empty() {
                for test in tests {
                    if config.include_ids.contains(&test.id) {
                        candidates.push(test);
                    }
                }
            }

            // Step 2: Include by tags (additive — add tests matching ALL include_tags)
            if !config.include_tags.is_empty() {
                for test in tests {
                    if config.include_tags.iter().all(|tag| test.tags.contains(tag)) {
                        if !candidates.iter().any(|c| c.id == test.id) {
                            candidates.push(test);
                        }
                    }
                }
            }

            // Step 3: Include by name pattern
            if let Some(ref pattern) = config.name_pattern {
                let pattern_lower = pattern.to_lowercase();
                for test in tests {
                    if name_matches_lower(&pattern_lower, &test.name)
                        && !candidates.iter().any(|c| c.id == test.id)
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
