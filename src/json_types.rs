//! ToJson / FromJson implementations for all platform types.

use crate::json::*;
use crate::types::*;
use crate::discovery::{DiscoveryQuery, DiscoveryResult, DiscoverySummary};

// ---------------------------------------------------------------------------
// TestDefinition
// ---------------------------------------------------------------------------

impl ToJson for TestDefinition {
    fn to_json(&self) -> JsonValue {
        let mut pairs: Vec<(&str, JsonValue)> = vec![
            ("id", str_val(&self.id)),
            ("name", str_val(&self.name)),
            ("tags", str_array(&self.tags)),
        ];
        if let Some(ref g) = self.group {
            pairs.push(("group", str_val(g)));
        }
        if let Some(ref d) = self.description {
            pairs.push(("description", str_val(d)));
        }
        if !self.metadata.is_empty() {
            let meta = JsonValue::Object(
                self.metadata.iter().map(|(k, v)| (k.clone(), str_val(v))).collect(),
            );
            pairs.push(("metadata", meta));
        }
        obj(pairs)
    }
}

impl FromJson for TestDefinition {
    fn from_json(value: &JsonValue) -> Result<Self, JsonError> {
        let id = value.get_str("id").unwrap_or("").to_string();
        let name = value.get_str("name").unwrap_or("").to_string();
        let tags = value
            .get("tags")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
        let group = value.get_str("group").map(String::from);
        let description = value.get_str("description").map(String::from);
        let metadata = value
            .get("metadata")
            .and_then(|v| v.as_object())
            .map(|pairs| {
                pairs.iter().filter_map(|(k, v)| {
                    v.as_str().map(|s| (k.clone(), s.to_string()))
                }).collect()
            })
            .unwrap_or_default();
        Ok(TestDefinition { id, name, tags, group, description, metadata })
    }
}

// ---------------------------------------------------------------------------
// TestStatus
// ---------------------------------------------------------------------------

impl ToJson for TestStatus {
    fn to_json(&self) -> JsonValue {
        str_val(match self {
            TestStatus::Passed => "passed",
            TestStatus::Failed => "failed",
            TestStatus::Error => "error",
            TestStatus::Skipped => "skipped",
        })
    }
}

impl FromJson for TestStatus {
    fn from_json(value: &JsonValue) -> Result<Self, JsonError> {
        match value.as_str() {
            Some("passed") => Ok(TestStatus::Passed),
            Some("failed") => Ok(TestStatus::Failed),
            Some("error") => Ok(TestStatus::Error),
            Some("skipped") => Ok(TestStatus::Skipped),
            _ => Ok(TestStatus::Error),
        }
    }
}

// ---------------------------------------------------------------------------
// ExecutionModel
// ---------------------------------------------------------------------------

impl ToJson for ExecutionModel {
    fn to_json(&self) -> JsonValue {
        match self {
            ExecutionModel::Sequential => obj(vec![("type", str_val("sequential"))]),
            ExecutionModel::Parallel { max_concurrency } => obj(vec![
                ("type", str_val("parallel")),
                ("max_concurrency", JsonValue::Number(*max_concurrency as f64)),
            ]),
        }
    }
}

impl FromJson for ExecutionModel {
    fn from_json(value: &JsonValue) -> Result<Self, JsonError> {
        match value.get_str("type") {
            Some("parallel") => {
                let mc = value.get_u32("max_concurrency").unwrap_or(4);
                Ok(ExecutionModel::Parallel { max_concurrency: mc })
            }
            _ => Ok(ExecutionModel::Sequential),
        }
    }
}

// ---------------------------------------------------------------------------
// RunConfig
// ---------------------------------------------------------------------------

impl ToJson for RunConfig {
    fn to_json(&self) -> JsonValue {
        let mut pairs: Vec<(&str, JsonValue)> = vec![
            ("run_all", JsonValue::Bool(self.run_all)),
        ];
        if !self.include_ids.is_empty() {
            pairs.push(("include_ids", str_array(&self.include_ids)));
        }
        if !self.include_tags.is_empty() {
            pairs.push(("include_tags", str_array(&self.include_tags)));
        }
        if !self.exclude_tags.is_empty() {
            pairs.push(("exclude_tags", str_array(&self.exclude_tags)));
        }
        if let Some(ref p) = self.name_pattern {
            pairs.push(("name_pattern", str_val(p)));
        }
        pairs.push(("fail_fast", JsonValue::Bool(self.fail_fast)));
        if let Some(t) = self.timeout_ms {
            pairs.push(("timeout_ms", JsonValue::Number(t as f64)));
        }
        pairs.push(("execution_model", self.execution_model.to_json()));
        obj(pairs)
    }
}

impl FromJson for RunConfig {
    fn from_json(value: &JsonValue) -> Result<Self, JsonError> {
        let run_all = value.get_bool("run_all").unwrap_or(true);
        let include_ids = parse_string_array(value.get("include_ids"));
        let include_tags = parse_string_array(value.get("include_tags"));
        let exclude_tags = parse_string_array(value.get("exclude_tags"));
        let name_pattern = value.get_str("name_pattern").map(String::from);
        let fail_fast = value.get_bool("fail_fast").unwrap_or(false);
        let timeout_ms = value.get_u64("timeout_ms");
        let execution_model = value
            .get("execution_model")
            .map(|v| ExecutionModel::from_json(v))
            .transpose()?
            .unwrap_or(ExecutionModel::Sequential);
        Ok(RunConfig {
            run_all,
            include_ids,
            include_tags,
            exclude_tags,
            name_pattern,
            fail_fast,
            timeout_ms,
            execution_model,
        })
    }
}

// ---------------------------------------------------------------------------
// TestResult
// ---------------------------------------------------------------------------

impl ToJson for TestResult {
    fn to_json(&self) -> JsonValue {
        let mut pairs: Vec<(&str, JsonValue)> = vec![
            ("test_id", str_val(&self.test_id)),
            ("status", self.status.to_json()),
            ("duration_ms", JsonValue::Number(self.duration_ms as f64)),
        ];
        if let Some(ref m) = self.message {
            pairs.push(("message", str_val(m)));
        }
        if let Some(ref s) = self.stdout {
            pairs.push(("stdout", str_val(s)));
        }
        if let Some(ref s) = self.stderr {
            pairs.push(("stderr", str_val(s)));
        }
        obj(pairs)
    }
}

impl FromJson for TestResult {
    fn from_json(value: &JsonValue) -> Result<Self, JsonError> {
        Ok(TestResult {
            test_id: value.get_str("test_id").unwrap_or("").to_string(),
            status: value
                .get("status")
                .map(|v| TestStatus::from_json(v))
                .transpose()?
                .unwrap_or(TestStatus::Error),
            duration_ms: value.get_u64("duration_ms").unwrap_or(0),
            message: value.get_str("message").map(String::from),
            stdout: value.get_str("stdout").map(String::from),
            stderr: value.get_str("stderr").map(String::from),
        })
    }
}

// ---------------------------------------------------------------------------
// RunProgress
// ---------------------------------------------------------------------------

impl ToJson for RunProgress {
    fn to_json(&self) -> JsonValue {
        obj(vec![
            ("run_id", str_val(&self.run_id)),
            ("total", JsonValue::Number(self.total as f64)),
            ("completed", JsonValue::Number(self.completed as f64)),
            ("passed", JsonValue::Number(self.passed as f64)),
            ("failed", JsonValue::Number(self.failed as f64)),
            ("skipped", JsonValue::Number(self.skipped as f64)),
            ("running", JsonValue::Number(self.running as f64)),
            ("percent_complete", JsonValue::Number(self.percent_complete)),
            ("elapsed_ms", JsonValue::Number(self.elapsed_ms as f64)),
        ])
    }
}

impl FromJson for RunProgress {
    fn from_json(value: &JsonValue) -> Result<Self, JsonError> {
        Ok(RunProgress {
            run_id: value.get_str("run_id").unwrap_or("").to_string(),
            total: value.get_u32("total").unwrap_or(0),
            completed: value.get_u32("completed").unwrap_or(0),
            passed: value.get_u32("passed").unwrap_or(0),
            failed: value.get_u32("failed").unwrap_or(0),
            skipped: value.get_u32("skipped").unwrap_or(0),
            running: value.get_u32("running").unwrap_or(0),
            percent_complete: value.get("percent_complete").and_then(|v| v.as_f64()).unwrap_or(0.0),
            elapsed_ms: value.get_u64("elapsed_ms").unwrap_or(0),
        })
    }
}

// ---------------------------------------------------------------------------
// RunSummary
// ---------------------------------------------------------------------------

impl ToJson for RunSummary {
    fn to_json(&self) -> JsonValue {
        obj(vec![
            ("run_id", str_val(&self.run_id)),
            ("config", self.config.to_json()),
            ("results", JsonValue::Array(self.results.iter().map(|r| r.to_json()).collect())),
            ("total", JsonValue::Number(self.total as f64)),
            ("passed", JsonValue::Number(self.passed as f64)),
            ("failed", JsonValue::Number(self.failed as f64)),
            ("skipped", JsonValue::Number(self.skipped as f64)),
            ("errored", JsonValue::Number(self.errored as f64)),
            ("total_duration_ms", JsonValue::Number(self.total_duration_ms as f64)),
            ("started_at", JsonValue::Number(self.started_at as f64)),
            ("completed_at", JsonValue::Number(self.completed_at as f64)),
        ])
    }
}

impl FromJson for RunSummary {
    fn from_json(value: &JsonValue) -> Result<Self, JsonError> {
        let results = value
            .get("results")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().map(|v| TestResult::from_json(v)).collect::<Result<Vec<_>, _>>())
            .transpose()?
            .unwrap_or_default();
        let config = value
            .get("config")
            .map(|v| RunConfig::from_json(v))
            .transpose()?
            .unwrap_or_default();
        Ok(RunSummary {
            run_id: value.get_str("run_id").unwrap_or("").to_string(),
            config,
            results,
            total: value.get_u32("total").unwrap_or(0),
            passed: value.get_u32("passed").unwrap_or(0),
            failed: value.get_u32("failed").unwrap_or(0),
            skipped: value.get_u32("skipped").unwrap_or(0),
            errored: value.get_u32("errored").unwrap_or(0),
            total_duration_ms: value.get_u64("total_duration_ms").unwrap_or(0),
            started_at: value.get_u64("started_at").unwrap_or(0),
            completed_at: value.get_u64("completed_at").unwrap_or(0),
        })
    }
}

// ---------------------------------------------------------------------------
// DiscoveryQuery
// ---------------------------------------------------------------------------

impl ToJson for DiscoveryQuery {
    fn to_json(&self) -> JsonValue {
        let mut pairs: Vec<(&str, JsonValue)> = Vec::new();
        if let Some(ref p) = self.name_pattern {
            pairs.push(("name_pattern", str_val(p)));
        }
        if !self.tags.is_empty() {
            pairs.push(("tags", str_array(&self.tags)));
        }
        if let Some(ref g) = self.group {
            pairs.push(("group", str_val(g)));
        }
        if let Some(l) = self.limit {
            pairs.push(("limit", JsonValue::Number(l as f64)));
        }
        if let Some(o) = self.offset {
            pairs.push(("offset", JsonValue::Number(o as f64)));
        }
        obj(pairs)
    }
}

impl FromJson for DiscoveryQuery {
    fn from_json(value: &JsonValue) -> Result<Self, JsonError> {
        Ok(DiscoveryQuery {
            name_pattern: value.get_str("name_pattern").map(String::from),
            tags: parse_string_array(value.get("tags")),
            group: value.get_str("group").map(String::from),
            limit: value.get("limit").and_then(|v| v.as_f64()).map(|n| n as usize),
            offset: value.get("offset").and_then(|v| v.as_f64()).map(|n| n as usize),
        })
    }
}

// ---------------------------------------------------------------------------
// DiscoveryResult
// ---------------------------------------------------------------------------

impl ToJson for DiscoveryResult {
    fn to_json(&self) -> JsonValue {
        obj(vec![
            ("tests", JsonValue::Array(self.tests.iter().map(|t| t.to_json()).collect())),
            ("total_matches", JsonValue::Number(self.total_matches as f64)),
            ("available_tags", str_array(&self.available_tags)),
            ("available_groups", str_array(&self.available_groups)),
        ])
    }
}

// ---------------------------------------------------------------------------
// DiscoverySummary
// ---------------------------------------------------------------------------

impl ToJson for DiscoverySummary {
    fn to_json(&self) -> JsonValue {
        let tags = JsonValue::Array(
            self.tags.iter().map(|(t, c)| {
                obj(vec![("tag", str_val(t)), ("count", JsonValue::Number(*c as f64))])
            }).collect(),
        );
        let groups = JsonValue::Array(
            self.groups.iter().map(|(g, c)| {
                obj(vec![("group", str_val(g)), ("count", JsonValue::Number(*c as f64))])
            }).collect(),
        );
        obj(vec![
            ("total_tests", JsonValue::Number(self.total_tests as f64)),
            ("tags", tags),
            ("groups", groups),
        ])
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_string_array(value: Option<&JsonValue>) -> Vec<String> {
    value
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default()
}
