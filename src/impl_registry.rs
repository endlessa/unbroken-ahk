//! Concrete implementation of TestRegistry backed by a Vec.
//!
//! Simple, predictable, debuggable. All data lives in memory and
//! can be serialized to JSON for inspection.

use crate::json::{JsonValue, ToJson, FromJson, parse_json, to_json_pretty};
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
    pub fn to_json_string(&self) -> String {
        let arr = JsonValue::Array(self.tests.iter().map(|t| t.to_json()).collect());
        to_json_pretty(&arr)
    }

    /// Load a registry from a JSON string.
    pub fn from_json_string(json: &str) -> Result<Self, String> {
        let value = parse_json(json).map_err(|e| format!("{}", e))?;
        let arr = value.as_array().ok_or("expected JSON array")?;
        let mut tests = Vec::new();
        for item in arr {
            tests.push(TestDefinition::from_json(item).map_err(|e| format!("{}", e))?);
        }
        Ok(Self { tests })
    }
}

impl TestRegistry for InMemoryRegistry {
    fn register(&mut self, test: TestDefinition) -> Result<(), RegistryError> {
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
            .filter(|t| {
                let name_lower = t.name.to_lowercase();
                // Support simple glob: "auth_*" matches "auth_basic", "auth_token"
                if let Some(prefix) = pattern_lower.strip_suffix('*') {
                    name_lower.starts_with(prefix)
                } else if let Some(suffix) = pattern_lower.strip_prefix('*') {
                    name_lower.ends_with(suffix)
                } else {
                    // Substring match
                    name_lower.contains(&pattern_lower)
                }
            })
            .collect()
    }

    fn filter_by_tags(&self, tags: &[String]) -> Vec<&TestDefinition> {
        self.tests
            .iter()
            .filter(|t| tags.iter().all(|tag| t.tags.contains(tag)))
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
    fn json_round_trip() {
        let mut reg = InMemoryRegistry::new();
        reg.register(make_test("t1", "auth_basic", &["smoke"], Some("auth"))).unwrap();
        reg.register(make_test("t2", "net_ping", &["slow"], Some("network"))).unwrap();
        let json = reg.to_json_string();
        let reg2 = InMemoryRegistry::from_json_string(&json).unwrap();
        assert_eq!(reg2.count(), 2);
        assert_eq!(reg2.get("t1").unwrap().name, "auth_basic");
    }
}
