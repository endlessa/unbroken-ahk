//! Concrete implementation of TestRegistry backed by a Vec.
//!
//! Simple, predictable, debuggable. All data lives in memory and
//! can be serialized to JSON for inspection.

use crate::json::{JsonValue, ToJson, to_json_pretty};
use crate::registry::{RegistryError, TestRegistry};
use crate::types::TestDefinition;

/// In-memory test registry. Stores tests in insertion order.
pub struct InMemoryRegistry {
    tests: Vec<TestDefinition>,
}

impl InMemoryRegistry {
    pub fn new() -> Self {
        Self { tests: Vec::new() }
    }

    /// Serialize the entire registry to a JSON string for storage/debugging.
    /// Loading persisted registries goes through storage::load_registry,
    /// which tolerates individual corrupt entries — there is deliberately
    /// no second, stricter loader here to diverge from it.
    pub fn to_json_string(&self) -> String {
        let arr = JsonValue::Array(self.tests.iter().map(|t| t.to_json()).collect());
        to_json_pretty(&arr)
    }

    /// Round-trip validity shared by register and replace: what the
    /// registry accepts must survive its own persistence round-trip, and
    /// the strict loader rejects empty id/name and duplicate metadata
    /// keys.
    fn validate(test: &TestDefinition) -> Result<(), RegistryError> {
        if test.id.is_empty() {
            return Err(RegistryError::InvalidDefinition("empty id".into()));
        }
        if test.name.is_empty() {
            return Err(RegistryError::InvalidDefinition("empty name".into()));
        }
        let mut meta_keys = std::collections::HashSet::new();
        for (k, _) in &test.metadata {
            if !meta_keys.insert(k.as_str()) {
                return Err(RegistryError::InvalidDefinition(format!(
                    "duplicate metadata key '{}'",
                    k
                )));
            }
        }
        Ok(())
    }

    /// Reorder to match `order` (by id). Ids not listed keep their
    /// existing relative order AFTER the listed ones. Used by
    /// load_from_storage so the FILE order — the discovery/fail_fast
    /// order every other session sees — is authoritative in memory too,
    /// whatever order this session happened to register in.
    pub fn reorder_to(&mut self, order: &[String]) {
        let rank: std::collections::HashMap<&str, usize> = order
            .iter()
            .enumerate()
            .map(|(i, id)| (id.as_str(), i))
            .collect();
        // sort_by_key is stable: unlisted ids keep relative order at MAX.
        self.tests
            .sort_by_key(|t| rank.get(t.id.as_str()).copied().unwrap_or(usize::MAX));
    }

    /// Replace an EXISTING definition IN PLACE, preserving registry
    /// order — a definition upgrade must not move the test to the end,
    /// silently shifting discovery pages and which tests execute first
    /// under fail_fast. Validation runs before any mutation, so a failed
    /// replacement leaves the old definition untouched.
    pub fn replace(&mut self, test: TestDefinition) -> Result<(), RegistryError> {
        Self::validate(&test)?;
        match self.tests.iter().position(|t| t.id == test.id) {
            Some(pos) => {
                self.tests[pos] = test;
                Ok(())
            }
            None => Err(RegistryError::InvalidDefinition(format!(
                "no existing definition '{}' to replace",
                test.id
            ))),
        }
    }
}

impl TestRegistry for InMemoryRegistry {
    fn register(&mut self, test: TestDefinition) -> Result<(), RegistryError> {
        Self::validate(&test)?;
        if self.tests.iter().any(|t| t.id == test.id) {
            return Err(RegistryError::DuplicateId(test.id));
        }
        self.tests.push(test);
        Ok(())
    }

    fn deregister(&mut self, id: &str) -> Option<TestDefinition> {
        let pos = self.tests.iter().position(|t| t.id == id)?;
        Some(self.tests.remove(pos))
    }

    fn get(&self, id: &str) -> Option<&TestDefinition> {
        self.tests.iter().find(|t| t.id == id)
    }

    fn list_all(&self) -> Vec<&TestDefinition> {
        self.tests.iter().collect()
    }

    fn count(&self) -> usize {
        self.tests.len()
    }

    fn search_by_name(&self, pattern: &str) -> Vec<&TestDefinition> {
        let pattern_lower = pattern.to_lowercase();
        self.tests
            .iter()
            .filter(|t| crate::filter::name_matches_lower(&pattern_lower, &t.name))
            .collect()
    }

    fn filter_by_tags(&self, tags: &[String]) -> Vec<&TestDefinition> {
        // The ONE shared all-tags predicate — a private copy here could
        // silently drift from what run selection and its zero-match
        // validation use.
        self.tests
            .iter()
            .filter(|t| crate::filter::matches_all_tags(tags, t))
            .collect()
    }

    fn filter_by_group(&self, group: &str) -> Vec<&TestDefinition> {
        self.tests
            .iter()
            .filter(|t| t.group.as_deref() == Some(group))
            .collect()
    }

    fn all_tags(&self) -> Vec<String> {
        let mut tags: Vec<String> = Vec::new();
        for test in &self.tests {
            for tag in &test.tags {
                if !tags.contains(tag) {
                    tags.push(tag.clone());
                }
            }
        }
        tags.sort();
        tags
    }

    fn all_groups(&self) -> Vec<String> {
        let mut groups: Vec<String> = Vec::new();
        for test in &self.tests {
            if let Some(ref g) = test.group {
                if !groups.contains(g) {
                    groups.push(g.clone());
                }
            }
        }
        groups.sort();
        groups
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test(id: &str, name: &str, tags: &[&str], group: Option<&str>) -> TestDefinition {
        TestDefinition {
            id: id.into(),
            name: name.into(),
            tags: tags.iter().map(|s| s.to_string()).collect(),
            group: group.map(String::from),
            description: None,
            metadata: vec![],
        }
    }

    #[test]
    fn register_and_lookup() {
        let mut reg = InMemoryRegistry::new();
        reg.register(make_test("t1", "auth_basic", &["smoke"], Some("auth"))).unwrap();
        assert_eq!(reg.count(), 1);
        assert!(reg.get("t1").is_some());
        assert!(reg.get("t2").is_none());
    }

    #[test]
    fn duplicate_rejected() {
        let mut reg = InMemoryRegistry::new();
        reg.register(make_test("t1", "test", &[], None)).unwrap();
        assert!(reg.register(make_test("t1", "test2", &[], None)).is_err());
    }

    #[test]
    fn empty_id_or_name_rejected_at_registration() {
        // What the registry accepts must survive its own persistence
        // round-trip — the load side rejects empty id/name, so the
        // write side must too.
        let mut reg = InMemoryRegistry::new();
        assert!(matches!(
            reg.register(make_test("", "named", &[], None)),
            Err(RegistryError::InvalidDefinition(_))
        ));
        assert!(matches!(
            reg.register(make_test("t1", "", &[], None)),
            Err(RegistryError::InvalidDefinition(_))
        ));
        assert_eq!(reg.count(), 0);
    }

    #[test]
    fn search_by_name_glob() {
        let mut reg = InMemoryRegistry::new();
        reg.register(make_test("t1", "auth_basic", &[], None)).unwrap();
        reg.register(make_test("t2", "auth_token", &[], None)).unwrap();
        reg.register(make_test("t3", "network_ping", &[], None)).unwrap();
        assert_eq!(reg.search_by_name("auth_*").len(), 2);
        assert_eq!(reg.search_by_name("ping").len(), 1);
    }

    #[test]
    fn filter_by_tags_all_match() {
        let mut reg = InMemoryRegistry::new();
        reg.register(make_test("t1", "a", &["smoke", "fast"], None)).unwrap();
        reg.register(make_test("t2", "b", &["smoke"], None)).unwrap();
        let tags = vec!["smoke".into(), "fast".into()];
        assert_eq!(reg.filter_by_tags(&tags).len(), 1);
    }

    #[test]
    fn deregister_returns_the_removed_test_and_preserves_order() {
        // Deregister must hand back exactly the named definition and
        // leave the survivors in insertion order — the order discovery
        // pagination and fail_fast execution depend on. Four tests so a
        // swap_remove regression ([t1, t4, t3]) cannot masquerade as
        // ordered removal; a missing id must remove nothing.
        let mut reg = InMemoryRegistry::new();
        reg.register(make_test("t1", "a", &[], None)).unwrap();
        reg.register(make_test("t2", "b", &[], None)).unwrap();
        reg.register(make_test("t3", "c", &[], None)).unwrap();
        reg.register(make_test("t4", "d", &[], None)).unwrap();
        let removed = reg.deregister("t2").unwrap();
        assert_eq!(removed.id, "t2");
        assert_eq!(reg.count(), 3);
        assert!(reg.get("t2").is_none());
        let order: Vec<&str> = reg.list_all().iter().map(|t| t.id.as_str()).collect();
        assert_eq!(order, vec!["t1", "t3", "t4"]);
        assert!(reg.deregister("missing").is_none());
        assert_eq!(reg.count(), 3);
    }

    #[test]
    fn reorder_to_puts_unlisted_ids_last_keeping_their_relative_order() {
        // Ids absent from the given order must land AFTER the listed
        // ones, keeping their existing relative order — never jump to
        // the front or shuffle — so file order stays authoritative for
        // discovery pages and fail_fast even when memory holds ids the
        // file does not.
        let mut reg = InMemoryRegistry::new();
        for id in ["a", "b", "c", "d"] {
            reg.register(make_test(id, id, &[], None)).unwrap();
        }
        reg.reorder_to(&["c".to_string(), "a".to_string()]);
        let order: Vec<&str> = reg.list_all().iter().map(|t| t.id.as_str()).collect();
        assert_eq!(order, vec!["c", "a", "b", "d"]);
    }

    #[test]
    fn replace_of_unknown_id_errors_instead_of_inserting() {
        // Replace upgrades an EXISTING definition; an unknown id must
        // error and mutate nothing — it must never degrade into an
        // insert that creates a definition no one registered.
        let mut reg = InMemoryRegistry::new();
        reg.register(make_test("t1", "a", &[], None)).unwrap();
        let err = reg.replace(make_test("t2", "b", &[], None)).unwrap_err();
        match err {
            RegistryError::InvalidDefinition(msg) => {
                assert_eq!(msg, "no existing definition 't2' to replace");
            }
            other => panic!("expected InvalidDefinition, got {:?}", other),
        }
        assert_eq!(reg.count(), 1);
        assert!(reg.get("t2").is_none());
    }

    #[test]
    fn search_by_name_lowercases_the_caller_pattern() {
        // name_matches_lower demands an already-lowered pattern; THIS
        // call site owns that lowering, so a mixed-case search — glob
        // or substring — must still find its test.
        let mut reg = InMemoryRegistry::new();
        reg.register(make_test("t1", "auth_basic", &[], None)).unwrap();
        assert_eq!(reg.search_by_name("AUTH_*").len(), 1);
        assert_eq!(reg.search_by_name("Basic").len(), 1);
    }

    #[test]
    fn all_tags_and_groups_are_sorted_deduped_and_skip_ungrouped() {
        // The facet accessors must return each value exactly once, in
        // sorted order regardless of registration order, and all_groups
        // must omit ungrouped tests — duplicates or insertion-order
        // output would corrupt the drill-down surfaces built on them.
        let mut reg = InMemoryRegistry::new();
        reg.register(make_test("t1", "a", &["zeta", "smoke"], Some("net"))).unwrap();
        reg.register(make_test("t2", "b", &["smoke", "alpha"], Some("auth"))).unwrap();
        reg.register(make_test("t3", "c", &["zeta"], None)).unwrap();
        let expected_tags: Vec<String> =
            vec!["alpha".into(), "smoke".into(), "zeta".into()];
        assert_eq!(reg.all_tags(), expected_tags);
        let expected_groups: Vec<String> = vec!["auth".into(), "net".into()];
        assert_eq!(reg.all_groups(), expected_groups);
    }

    #[test]
    fn json_round_trip() {
        use crate::json::{parse_json, FromJson};
        use crate::types::TestDefinition;
        let mut reg = InMemoryRegistry::new();
        reg.register(make_test("t1", "auth_basic", &["smoke"], Some("auth"))).unwrap();
        reg.register(make_test("t2", "net_ping", &["slow"], Some("network"))).unwrap();
        let json = reg.to_json_string();
        let parsed = parse_json(&json).unwrap();
        let defs: Vec<TestDefinition> = parsed
            .as_array()
            .unwrap()
            .iter()
            .map(|v| TestDefinition::from_json(v).unwrap())
            .collect();
        assert_eq!(defs.len(), 2);
        assert_eq!(defs[0].name, "auth_basic");
        assert_eq!(defs[1].tags, vec!["slow".to_string()]);
    }
}
