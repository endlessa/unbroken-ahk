//! Test Registry — the single source of truth for what tests exist.
//!
//! All discovery feeds into the registry. All filtering reads from it.

use crate::types::TestDefinition;
use crate::types::TestId;

/// The registry holds every known test definition and supports
/// registration and lookup operations.
pub trait TestRegistry {
    /// Register a new test. Returns an error if the ID is already taken.
    fn register(&mut self, test: TestDefinition) -> Result<(), RegistryError>;

    /// Remove a test by ID. Returns the definition if it existed.
    fn deregister(&mut self, id: &str) -> Option<TestDefinition>;

    /// Look up a single test by its exact ID.
    fn get(&self, id: &str) -> Option<&TestDefinition>;

    /// Return all registered test definitions.
    fn list_all(&self) -> Vec<&TestDefinition>;

    /// Return the total number of registered tests.
    fn count(&self) -> usize;

    /// Search tests by a name substring or pattern.
    fn search_by_name(&self, pattern: &str) -> Vec<&TestDefinition>;

    /// Return all tests that have ALL of the given tags.
    fn filter_by_tags(&self, tags: &[String]) -> Vec<&TestDefinition>;

    /// Return all tests belonging to a given group.
    fn filter_by_group(&self, group: &str) -> Vec<&TestDefinition>;

    /// Return all known tag values across all tests.
    fn all_tags(&self) -> Vec<String>;

    /// Return all known group names across all tests.
    fn all_groups(&self) -> Vec<String>;
}

/// Errors that can occur during registry operations.
#[derive(Debug, Clone)]
pub enum RegistryError {
    /// A test with this ID is already registered.
    DuplicateId(TestId),
}
