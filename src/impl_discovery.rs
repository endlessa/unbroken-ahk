//! Concrete implementation of TestDiscovery backed by a TestRegistry.

use crate::discovery::{DiscoveryQuery, DiscoveryResult, DiscoverySummary, TestDiscovery};
use crate::registry::TestRegistry;
use crate::types::TestDefinition;

/// Discovery implementation that delegates to a registry.
pub struct RegistryDiscovery<'a, R: TestRegistry> {
    registry: &'a R,
}

impl<'a, R: TestRegistry> RegistryDiscovery<'a, R> {
    pub fn new(registry: &'a R) -> Self {
        Self { registry }
    }
}

impl<'a, R: TestRegistry> TestDiscovery for RegistryDiscovery<'a, R> {
    fn discover(&self, query: &DiscoveryQuery) -> DiscoveryResult {
        // Start with all tests, then narrow down
        let mut matches: Vec<&TestDefinition> = self.registry.list_all();

        // Filter by name pattern
        if let Some(ref pattern) = query.name_pattern {
            let found = self.registry.search_by_name(pattern);
            matches.retain(|t| found.iter().any(|f| f.id == t.id));
        }

        // Filter by tags (ALL must match)
        if !query.tags.is_empty() {
            matches.retain(|t| query.tags.iter().all(|tag| t.tags.contains(tag)));
        }

        // Filter by group
        if let Some(ref group) = query.group {
            matches.retain(|t| t.group.as_deref() == Some(group.as_str()));
        }

        let total_matches = matches.len();

        // Collect tags and groups from matches
        let mut available_tags: Vec<String> = Vec::new();
        let mut available_groups: Vec<String> = Vec::new();
        for test in &matches {
            for tag in &test.tags {
                if !available_tags.contains(tag) {
                    available_tags.push(tag.clone());
                }
            }
            if let Some(ref g) = test.group {
                if !available_groups.contains(g) {
                    available_groups.push(g.clone());
                }
            }
        }
        available_tags.sort();
        available_groups.sort();

        // Apply pagination
        let offset = query.offset.unwrap_or(0);
        let matches: Vec<&TestDefinition> = if offset < matches.len() {
            let sliced = &matches[offset..];
            match query.limit {
                Some(limit) => sliced.iter().take(limit).copied().collect(),
                None => sliced.to_vec(),
            }
        } else {
            Vec::new()
        };

        DiscoveryResult {
            tests: matches.into_iter().cloned().collect(),
            total_matches,
            available_tags,
            available_groups,
        }
    }

    fn summary(&self) -> DiscoverySummary {
        let all = self.registry.list_all();

        // Count tags
        let mut tag_counts: Vec<(String, usize)> = Vec::new();
        for test in &all {
            for tag in &test.tags {
                if let Some(entry) = tag_counts.iter_mut().find(|(t, _)| t == tag) {
                    entry.1 += 1;
                } else {
                    tag_counts.push((tag.clone(), 1));
                }
            }
        }
        tag_counts.sort_by(|a, b| a.0.cmp(&b.0));

        // Count groups
        let mut group_counts: Vec<(String, usize)> = Vec::new();
        for test in &all {
            if let Some(ref g) = test.group {
                if let Some(entry) = group_counts.iter_mut().find(|(grp, _)| grp == g) {
                    entry.1 += 1;
                } else {
                    group_counts.push((g.clone(), 1));
                }
            }
        }
        group_counts.sort_by(|a, b| a.0.cmp(&b.0));

        DiscoverySummary {
            total_tests: all.len(),
            tags: tag_counts,
            groups: group_counts,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::impl_registry::InMemoryRegistry;

    fn setup() -> InMemoryRegistry {
        let mut reg = InMemoryRegistry::new();
        for (id, name, tags, group) in [
            ("t1", "auth_basic", vec!["smoke", "fast"], Some("auth")),
            ("t2", "auth_token", vec!["smoke"], Some("auth")),
            ("t3", "net_ping", vec!["slow"], Some("network")),
            ("t4", "net_dns", vec!["smoke", "slow"], Some("network")),
        ] {
            reg.register(TestDefinition {
                id: id.into(),
                name: name.into(),
                tags: tags.into_iter().map(String::from).collect(),
                group: group.map(String::from),
                description: None,
                metadata: vec![],
            })
            .unwrap();
        }
        reg
    }

    #[test]
    fn discover_all() {
        let reg = setup();
        let disc = RegistryDiscovery::new(&reg);
        let result = disc.discover(&DiscoveryQuery::default());
        assert_eq!(result.total_matches, 4);
    }

    #[test]
    fn discover_by_group() {
        let reg = setup();
        let disc = RegistryDiscovery::new(&reg);
        let result = disc.discover(&DiscoveryQuery {
            group: Some("auth".into()),
            ..Default::default()
        });
        assert_eq!(result.total_matches, 2);
    }

    #[test]
    fn discover_with_pagination() {
        let reg = setup();
        let disc = RegistryDiscovery::new(&reg);
        let result = disc.discover(&DiscoveryQuery {
            limit: Some(2),
            offset: Some(1),
            ..Default::default()
        });
        assert_eq!(result.total_matches, 4);
        assert_eq!(result.tests.len(), 2);
    }

    #[test]
    fn summary_counts() {
        let reg = setup();
        let disc = RegistryDiscovery::new(&reg);
        let sum = disc.summary();
        assert_eq!(sum.total_tests, 4);
        assert!(sum.tags.iter().any(|(t, c)| t == "smoke" && *c == 3));
        assert!(sum.groups.iter().any(|(g, c)| g == "auth" && *c == 2));
    }
}
