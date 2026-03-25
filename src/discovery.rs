//! Discovery — the public-facing interface for callers to find out
//! what tests are available before requesting a run.

use crate::types::TestDefinition;

/// Describes a query for discovering tests.
#[derive(Debug, Clone, Default)]
pub struct DiscoveryQuery {
    /// Search test names by substring or pattern.
    pub name_pattern: Option<String>,
    /// Only return tests with ALL of these tags.
    pub tags: Vec<String>,
    /// Only return tests in this group.
    pub group: Option<String>,
    /// Maximum number of results to return. None means no limit.
    pub limit: Option<usize>,
    /// Offset for pagination.
    pub offset: Option<usize>,
}

/// Result of a discovery query.
#[derive(Debug, Clone)]
pub struct DiscoveryResult {
    /// The matching tests.
    pub tests: Vec<TestDefinition>,
    /// Total number of matches (before limit/offset).
    pub total_matches: usize,
    /// All tag values available across matching tests.
    pub available_tags: Vec<String>,
    /// All group names available across matching tests.
    pub available_groups: Vec<String>,
}

/// The discovery interface that callers use to explore what's available.
pub trait TestDiscovery {
    /// Query for available tests with optional filtering and pagination.
    fn discover(&self, query: &DiscoveryQuery) -> DiscoveryResult;

    /// Quick summary: how many tests exist, what tags and groups are available.
    fn summary(&self) -> DiscoverySummary;
}

/// High-level overview of the test landscape.
#[derive(Debug, Clone)]
pub struct DiscoverySummary {
    /// Total number of registered tests.
    pub total_tests: usize,
    /// All available tags with counts.
    pub tags: Vec<(String, usize)>,
    /// All available groups with counts.
    pub groups: Vec<(String, usize)>,
}
