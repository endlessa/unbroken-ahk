//! Concrete implementation of TestFilter.

use crate::filter::TestFilter;
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
        if config.run_all {
            return tests.to_vec();
        }

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
                let name_lower = test.name.to_lowercase();
                let matches = if let Some(prefix) = pattern_lower.strip_suffix('*') {
                    name_lower.starts_with(prefix)
                } else if let Some(suffix) = pattern_lower.strip_prefix('*') {
                    name_lower.ends_with(suffix)
                } else {
                    name_lower.contains(&pattern_lower)
                };
                if matches && !candidates.iter().any(|c| c.id == test.id) {
                    candidates.push(test);
                }
            }
        }

        // If no include criteria were specified, start with everything
        if config.include_ids.is_empty()
            && config.include_tags.is_empty()
            && config.name_pattern.is_none()
        {
            candidates = tests.to_vec();
        }

        // Step 4: Exclude by tags (remove tests matching ANY exclude tag)
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
        let config = RunConfig {
            run_all: false,
            exclude_tags: vec!["slow".into()],
            ..Default::default()
        };
        let result = StandardFilter::new().apply(&refs, &config);
        assert_eq!(result.len(), 2);
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
