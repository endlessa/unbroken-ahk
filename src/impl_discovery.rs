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

        // Filter by name pattern — the same predicate the run filter and
        // its zero-match validation use, applied in one pass (fetching
        // search_by_name's full set and re-matching by id walked the
        // registry twice for the same answer).
        if let Some(ref pattern) = query.name_pattern {
            let pattern_lower = pattern.to_lowercase();
            matches.retain(|t| crate::filter::name_matches_lower(&pattern_lower, &t.name));
        }

        // Filter by tags — the SAME predicate run selection and its
        // zero-match validation use, so 'discover --tag X' can never
        // disagree with 'run --tag X'.
        if !query.tags.is_empty() {
            matches.retain(|t| crate::filter::matches_all_tags(&query.tags, t));
        }

        // Filter by group
        if let Some(ref group) = query.group {
            matches.retain(|t| t.group.as_deref() == Some(group.as_str()));
        }

        let total_matches = matches.len();

        // Collect tags and groups from matches — set-based so an
        // interactive listing stays linear instead of Vec::contains
        // per tag; BTreeSet iteration is already sorted.
        let mut tag_set: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        let mut group_set: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for test in &matches {
            for tag in &test.tags {
                tag_set.insert(tag.clone());
            }
            if let Some(ref g) = test.group {
                group_set.insert(g.clone());
            }
        }
        let available_tags: Vec<String> = tag_set.into_iter().collect();
        let available_groups: Vec<String> = group_set.into_iter().collect();

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

        // Count tags and groups map-based — a linear find per occurrence
        // was quadratic on this interactive path; BTreeMap keeps the
        // sorted output the callers expect.
        let mut tag_counts: std::collections::BTreeMap<String, usize> =
            std::collections::BTreeMap::new();
        let mut group_counts: std::collections::BTreeMap<String, usize> =
            std::collections::BTreeMap::new();
        for test in &all {
            for tag in &test.tags {
                *tag_counts.entry(tag.clone()).or_insert(0) += 1;
            }
            if let Some(ref g) = test.group {
                *group_counts.entry(g.clone()).or_insert(0) += 1;
            }
        }
        let tag_counts: Vec<(String, usize)> = tag_counts.into_iter().collect();
        let group_counts: Vec<(String, usize)> = group_counts.into_iter().collect();

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
        // INVARIANT: summary tag/group counts come back as SORTED ordered
        // sequences (the BTreeMap guarantee callers render verbatim) — a
        // HashMap swap would randomize console/MCP output per process.
        assert_eq!(
            sum.tags,
            vec![
                ("fast".to_string(), 1),
                ("slow".to_string(), 2),
                ("smoke".to_string(), 3),
            ]
        );
        assert_eq!(
            sum.groups,
            vec![("auth".to_string(), 2), ("network".to_string(), 2)]
        );
    }

    #[test]
    fn facets_cover_all_filtered_matches_despite_pagination() {
        let reg = setup();
        let disc = RegistryDiscovery::new(&reg);
        let result = disc.discover(&DiscoveryQuery {
            group: Some("network".into()),
            limit: Some(1),
            ..Default::default()
        });
        // INVARIANT: available_tags/available_groups aggregate over the
        // FILTERED set, not the paginated page and not the whole registry
        // — "smoke" appears only on the second network test (t4), which
        // the 1-item page cuts off, while auth-only facets ("fast",
        // "auth") must not leak in; output is sorted and deduped.
        assert_eq!(result.tests.len(), 1);
        assert_eq!(result.tests[0].id, "t3");
        assert_eq!(result.available_tags, vec!["slow", "smoke"]);
        assert_eq!(result.available_groups, vec!["network"]);
        assert_eq!(result.total_matches, 2);
    }

    #[test]
    fn offset_past_end_returns_empty_page_not_panic() {
        let reg = setup();
        let disc = RegistryDiscovery::new(&reg);
        let result = disc.discover(&DiscoveryQuery {
            offset: Some(10),
            ..Default::default()
        });
        // INVARIANT: an offset at or past the end of the matches yields an
        // empty page (never an out-of-range slice panic), while
        // total_matches and the facets still describe the full filtered
        // set so a paging client can recover.
        assert!(result.tests.is_empty());
        assert_eq!(result.total_matches, 4);
        assert_eq!(result.available_tags, vec!["fast", "slow", "smoke"]);
        assert_eq!(result.available_groups, vec!["auth", "network"]);
    }

    #[test]
    fn discover_lowercases_caller_pattern() {
        let reg = setup();
        let disc = RegistryDiscovery::new(&reg);
        let result = disc.discover(&DiscoveryQuery {
            name_pattern: Some("AUTH_*".into()),
            ..Default::default()
        });
        // INVARIANT: discover lowercases the caller's pattern before
        // handing it to name_matches_lower (whose contract requires an
        // already-lowered pattern) — an uppercase pattern must still find
        // the lowercase-named tests, or case-insensitivity is silently
        // half-implemented at this call site.
        assert_eq!(result.total_matches, 2);
        let ids: Vec<&str> = result.tests.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids, vec!["t1", "t2"]);
    }
}
