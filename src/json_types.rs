//! ToJson / FromJson implementations for all platform types.

use crate::json::*;
use crate::types::*;
use crate::discovery::{DiscoveryQuery, DiscoveryResult, DiscoverySummary};


/// Serialize a u64 for JSON, clamped to the exact-integer range (2^53):
/// the strict read side rejects larger values, so the write side must
/// never produce them — a pathological duration/timestamp is stored as
/// the clamp rather than a value the platform itself cannot reload.
fn u64_json(n: u64) -> JsonValue {
    JsonValue::Number(n.min(9_007_199_254_740_992) as f64)
}

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
        // id and name are the test's identity — a missing, mistyped, or empty
        // value must be a load error, not a silent ghost entry. The remaining
        // fields are equally strict when present, and unknown keys are
        // rejected (registry.json is hand-editable; a misspelled field
        // silently dropped is invisible corruption).
        reject_unknown_keys(
            value,
            "test definition",
            &["id", "name", "tags", "group", "description", "metadata"],
        )?;
        let id = match value.get_str("id") {
            Some(s) if !s.is_empty() => s.to_string(),
            _ => return Err(JsonError::MissingField("id".into())),
        };
        let name = match value.get_str("name") {
            Some(s) if !s.is_empty() => s.to_string(),
            _ => return Err(JsonError::MissingField("name".into())),
        };
        let tags = strict_string_array(value, "tags")?;
        let group = strict_opt_string(value, "group")?;
        let description = strict_opt_string(value, "description")?;
        let metadata = match value.get("metadata") {
            None | Some(JsonValue::Null) => Vec::new(),
            Some(JsonValue::Object(pairs)) => pairs
                .iter()
                .map(|(k, v)| {
                    v.as_str().map(|s| (k.clone(), s.to_string())).ok_or_else(|| {
                        JsonError::InvalidField(format!("metadata.{}", k), "a string".into())
                    })
                })
                .collect::<Result<Vec<_>, _>>()?,
            Some(_) => {
                return Err(JsonError::InvalidField(
                    "metadata".into(),
                    "an object with string values".into(),
                ))
            }
        };
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
            // A corrupted status must be a load error, not a silent Error
            // status that skews the counts.
            _ => Err(JsonError::InvalidField(
                "status".into(),
                "one of \"passed\", \"failed\", \"error\", \"skipped\"".into(),
            )),
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
        reject_unknown_keys(value, "execution_model", &["type", "max_concurrency"])?;
        match value.get("type") {
            None | Some(JsonValue::Null) => {
                // max_concurrency without a type is a clear parallel intent
                // that would otherwise silently run sequentially.
                if matches!(value.get("max_concurrency"), Some(v) if !matches!(v, JsonValue::Null))
                {
                    return Err(JsonError::InvalidField(
                        "execution_model.type".into(),
                        "required when max_concurrency is given (use \"parallel\")".into(),
                    ));
                }
                Ok(ExecutionModel::Sequential)
            }
            Some(JsonValue::Str(s)) if s == "sequential" => Ok(ExecutionModel::Sequential),
            Some(JsonValue::Str(s)) if s == "parallel" => {
                let mc = match value.get("max_concurrency") {
                    None | Some(JsonValue::Null) => 4,
                    Some(JsonValue::Number(n))
                        if *n >= 1.0 && n.fract() == 0.0 && *n <= u32::MAX as f64 =>
                    {
                        *n as u32
                    }
                    Some(_) => {
                        return Err(JsonError::InvalidField(
                            "execution_model.max_concurrency".into(),
                            "a positive integer that fits in 32 bits".into(),
                        ))
                    }
                };
                Ok(ExecutionModel::Parallel { max_concurrency: mc })
            }
            // A mis-cased or unrecognized type must not silently become
            // Sequential — that hides the caller's intent.
            Some(JsonValue::Str(s)) => Err(JsonError::InvalidField(
                "execution_model.type".into(),
                format!("\"sequential\" or \"parallel\" (got \"{}\")", s),
            )),
            Some(_) => Err(JsonError::InvalidField(
                "execution_model.type".into(),
                "a string".into(),
            )),
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
            pairs.push(("timeout_ms", u64_json(t)));
        }
        pairs.push(("execution_model", self.execution_model.to_json()));
        obj(pairs)
    }
}

impl FromJson for RunConfig {
    fn from_json(value: &JsonValue) -> Result<Self, JsonError> {
        parse_run_config(value, true)
    }
}

impl RunConfig {
    /// Parse a config embedded in a PERSISTED run summary. Same strict
    /// typing as caller input, but without the bare-run_all:false guard:
    /// files written before the exclude-under-run_all rework legitimately
    /// contain {"run_all": false, "exclude_tags": [...]} configs, and
    /// history must stay readable.
    pub fn from_json_stored(value: &JsonValue) -> Result<Self, JsonError> {
        parse_run_config(value, false)
    }
}

fn parse_run_config(value: &JsonValue, reject_bare_run_all_false: bool) -> Result<RunConfig, JsonError> {
    // Every field is strictly typed when present, and unknown keys are
    // rejected: a mistyped filter — wrong type OR wrong name ("tags"
    // instead of "include_tags") — must be an error, never a
    // silently-empty filter that runs the whole suite.
    reject_unknown_keys(
        value,
        "run config",
        &[
            "run_all",
            "include_ids",
            "include_tags",
            "exclude_tags",
            "name_pattern",
            "fail_fast",
            "timeout_ms",
            "execution_model",
        ],
    )?;
    let include_ids = strict_string_array(value, "include_ids")?;
    let include_tags = strict_string_array(value, "include_tags")?;
    let exclude_tags = strict_string_array(value, "exclude_tags")?;
    let name_pattern = strict_opt_string(value, "name_pattern")?;
    // An empty pattern substring-matches EVERY test — CALLER input meant
    // to narrow the run must never silently widen to the whole suite.
    // Stored configs are exempt (same gate as the run_all guard below):
    // history persisted by older, lenient versions must stay readable.
    if reject_bare_run_all_false && name_pattern.as_deref() == Some("") {
        return Err(JsonError::InvalidField(
            "name_pattern".into(),
            "a non-empty string (an empty pattern would match every test)".into(),
        ));
    }
    // run_all defaults to true unless an INCLUDE-side key was supplied.
    // Key PRESENCE decides, not emptiness: {"include_ids": []} selected
    // zero tests — that must become NoTestsMatched, never run-everything.
    // exclude_tags does not flip the default: exclusions apply under
    // run_all anyway, so {"exclude_tags": [...]} means "everything
    // except these" and {"exclude_tags": []} means "everything".
    let includes_supplied = ["include_ids", "include_tags", "name_pattern"]
        .iter()
        .any(|k| matches!(value.get(k), Some(v) if !matches!(v, JsonValue::Null)));
    let run_all_explicit = strict_opt_bool(value, "run_all")?;
    let run_all = run_all_explicit.unwrap_or(!includes_supplied);
    // An explicit run_all=false without include keys selects nothing by
    // definition — exclusions never resurrect an empty selection — so
    // name the cause and the fix here, where key presence is still
    // visible, instead of a bare no-tests-matched downstream.
    // ({"run_all": false, "include_ids": []} keeps its NoTestsMatched
    // contract: an include key WAS supplied.) Skipped for persisted
    // configs (from_json_stored), which may predate this rule.
    if reject_bare_run_all_false && run_all_explicit == Some(false) && !includes_supplied {
        return Err(JsonError::InvalidField(
            "run_all".into(),
            "false requires at least one include filter (include_ids, \
             include_tags, or name_pattern) — exclusions cannot resurrect \
             an empty selection, so with run_all false nothing is selected; \
             drop run_all or add include filters"
                .into(),
        ));
    }
    let fail_fast = strict_opt_bool(value, "fail_fast")?.unwrap_or(false);
    let timeout_ms = strict_opt_u64(value, "timeout_ms")?;
    let execution_model = match value.get("execution_model") {
        None | Some(JsonValue::Null) => ExecutionModel::Sequential,
        Some(obj @ JsonValue::Object(_)) => ExecutionModel::from_json(obj)?,
        // A bare string like "parallel" must not silently become
        // Sequential — the caller's intent would be ignored.
        Some(_) => {
            return Err(JsonError::InvalidField(
                "execution_model".into(),
                "an object like {\"type\": \"sequential\"}".into(),
            ))
        }
    };
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

// ---------------------------------------------------------------------------
// TestResult
// ---------------------------------------------------------------------------

impl ToJson for TestResult {
    fn to_json(&self) -> JsonValue {
        let mut pairs: Vec<(&str, JsonValue)> = vec![
            ("test_id", str_val(&self.test_id)),
            ("status", self.status.to_json()),
            ("duration_ms", u64_json(self.duration_ms)),
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
        // Strict like every other loader: a result whose identity or
        // status is missing/mistyped is a damaged record (CorruptRun
        // upstream), never a silent Error-status placeholder.
        let test_id = match value.get_str("test_id") {
            Some(s) if !s.is_empty() => s.to_string(),
            _ => return Err(JsonError::MissingField("test_id".into())),
        };
        let status = match value.get("status") {
            Some(v) => TestStatus::from_json(v)?,
            None => return Err(JsonError::MissingField("status".into())),
        };
        Ok(TestResult {
            test_id,
            status,
            duration_ms: strict_opt_u64(value, "duration_ms")?.unwrap_or(0),
            message: strict_opt_string(value, "message")?,
            stdout: strict_opt_string(value, "stdout")?,
            stderr: strict_opt_string(value, "stderr")?,
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
            ("errored", JsonValue::Number(self.errored as f64)),
            ("skipped", JsonValue::Number(self.skipped as f64)),
            ("running", JsonValue::Number(self.running as f64)),
            ("percent_complete", JsonValue::Number(self.percent_complete)),
            ("elapsed_ms", u64_json(self.elapsed_ms)),
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
            errored: value.get_u32("errored").unwrap_or(0),
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
            ("total_duration_ms", u64_json(self.total_duration_ms)),
            ("started_at", u64_json(self.started_at)),
            ("completed_at", u64_json(self.completed_at)),
        ])
    }
}

impl FromJson for RunSummary {
    fn from_json(value: &JsonValue) -> Result<Self, JsonError> {
        // Strictly typed when present: a summary whose results field or
        // counters are mistyped is a damaged record and must load as an
        // error (surfaced upstream as CorruptRun), never as a summary
        // silently claiming zero results.
        let run_id = match value.get_str("run_id") {
            Some(s) if !s.is_empty() => s.to_string(),
            _ => return Err(JsonError::MissingField("run_id".into())),
        };
        let results = match value.get("results") {
            None | Some(JsonValue::Null) => Vec::new(),
            Some(JsonValue::Array(arr)) => arr
                .iter()
                .map(TestResult::from_json)
                .collect::<Result<Vec<_>, _>>()?,
            Some(_) => {
                return Err(JsonError::InvalidField(
                    "results".into(),
                    "an array of test results".into(),
                ))
            }
        };
        let config = value
            .get("config")
            .map(RunConfig::from_json_stored)
            .transpose()?
            .unwrap_or_default();
        let count = |field: &str| -> Result<u32, JsonError> {
            match strict_opt_u64(value, field)?.unwrap_or(0) {
                n if n <= u32::MAX as u64 => Ok(n as u32),
                // Reject rather than wrap modulo 2^32 — a fabricated count
                // is exactly what strict loading exists to prevent.
                _ => Err(JsonError::InvalidField(
                    field.into(),
                    "a count that fits in 32 bits".into(),
                )),
            }
        };
        Ok(RunSummary {
            run_id,
            config,
            results,
            total: count("total")?,
            passed: count("passed")?,
            failed: count("failed")?,
            skipped: count("skipped")?,
            errored: count("errored")?,
            total_duration_ms: strict_opt_u64(value, "total_duration_ms")?.unwrap_or(0),
            started_at: strict_opt_u64(value, "started_at")?.unwrap_or(0),
            completed_at: strict_opt_u64(value, "completed_at")?.unwrap_or(0),
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
        reject_unknown_keys(
            value,
            "discovery query",
            &["name_pattern", "tags", "group", "limit", "offset"],
        )?;
        Ok(DiscoveryQuery {
            name_pattern: strict_opt_string(value, "name_pattern")?,
            tags: strict_string_array(value, "tags")?,
            group: strict_opt_string(value, "group")?,
            // Saturate rather than truncate: on 32-bit targets (wasm32) an
            // oversized limit must clamp to "everything", not wrap to 0.
            limit: strict_opt_u64(value, "limit")?
                .map(|n| usize::try_from(n).unwrap_or(usize::MAX)),
            offset: strict_opt_u64(value, "offset")?
                .map(|n| usize::try_from(n).unwrap_or(usize::MAX)),
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
        obj(vec![
            ("total_tests", JsonValue::Number(self.total_tests as f64)),
            ("tags", counts_json("tag", &self.tags)),
            ("groups", counts_json("group", &self.groups)),
        ])
    }
}

/// Serialize a (name, count) list as [{<key>: name, "count": n}, ...].
/// Single source of truth for the tag/group count shape used by the
/// summary, the MCP list tools, and the console commands.
pub fn counts_json(key: &str, items: &[(String, usize)]) -> JsonValue {
    JsonValue::Array(
        items.iter().map(|(name, count)| {
            obj(vec![(key, str_val(name)), ("count", JsonValue::Number(*count as f64))])
        }).collect(),
    )
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Reject caller-supplied input that is not an object, or that carries
/// keys outside the known set — a misspelled key ("tags" for
/// "include_tags") must error, never act as "no filter".
fn reject_unknown_keys(
    value: &JsonValue,
    what: &str,
    known: &[&str],
) -> Result<(), JsonError> {
    match value {
        JsonValue::Object(pairs) => {
            for (key, _) in pairs {
                if !known.contains(&key.as_str()) {
                    return Err(JsonError::UnknownField(
                        what.into(),
                        key.clone(),
                        known.join(", "),
                    ));
                }
            }
            Ok(())
        }
        _ => Err(JsonError::InvalidField(
            what.into(),
            "a JSON object".into(),
        )),
    }
}

/// Strictly-typed optional field accessors. An absent field (or explicit
/// null, the JSON convention for "not set") yields the default; a present
/// field of the wrong type is an error, never a silent fallback.

fn strict_string_array(value: &JsonValue, field: &str) -> Result<Vec<String>, JsonError> {
    match value.get(field) {
        None | Some(JsonValue::Null) => Ok(Vec::new()),
        Some(JsonValue::Array(items)) => items
            .iter()
            .map(|v| {
                v.as_str().map(String::from).ok_or_else(|| {
                    JsonError::InvalidField(field.into(), "an array of strings".into())
                })
            })
            .collect(),
        Some(_) => Err(JsonError::InvalidField(
            field.into(),
            "an array of strings".into(),
        )),
    }
}

fn strict_opt_string(value: &JsonValue, field: &str) -> Result<Option<String>, JsonError> {
    match value.get(field) {
        None | Some(JsonValue::Null) => Ok(None),
        Some(JsonValue::Str(s)) => Ok(Some(s.clone())),
        Some(_) => Err(JsonError::InvalidField(field.into(), "a string".into())),
    }
}

fn strict_opt_bool(value: &JsonValue, field: &str) -> Result<Option<bool>, JsonError> {
    match value.get(field) {
        None | Some(JsonValue::Null) => Ok(None),
        Some(JsonValue::Bool(b)) => Ok(Some(*b)),
        Some(_) => Err(JsonError::InvalidField(field.into(), "a boolean".into())),
    }
}

/// Largest integer a JSON double represents exactly (2^53). Values above
/// it have already lost integer precision in transit, so the strict
/// loaders reject them rather than fabricating a number.
const MAX_SAFE_JSON_INT: f64 = 9_007_199_254_740_992.0;

fn strict_opt_u64(value: &JsonValue, field: &str) -> Result<Option<u64>, JsonError> {
    match value.get(field) {
        None | Some(JsonValue::Null) => Ok(None),
        Some(JsonValue::Number(n))
            if *n >= 0.0 && n.is_finite() && n.fract() == 0.0 && *n <= MAX_SAFE_JSON_INT =>
        {
            Ok(Some(*n as u64))
        }
        Some(_) => Err(JsonError::InvalidField(
            field.into(),
            "a non-negative integer within the exact JSON range (<= 2^53)".into(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_config_filters_imply_run_all_false() {
        // Include-side filters without an explicit run_all must NOT default
        // to run-everything.
        let val = parse_json(r#"{"include_tags": ["fast"]}"#).unwrap();
        let config = RunConfig::from_json(&val).unwrap();
        assert!(!config.run_all);
        assert_eq!(config.include_tags, vec!["fast".to_string()]);

        let val = parse_json(r#"{"name_pattern": "auth_*"}"#).unwrap();
        let config = RunConfig::from_json(&val).unwrap();
        assert!(!config.run_all);

        // exclude_tags alone keeps run_all true — exclusions apply under
        // run_all, so this means "everything except slow".
        let val = parse_json(r#"{"exclude_tags": ["slow"]}"#).unwrap();
        let config = RunConfig::from_json(&val).unwrap();
        assert!(config.run_all);
        assert_eq!(config.exclude_tags, vec!["slow".to_string()]);

        // "Exclude nothing" runs everything, not NoTestsMatched.
        let val = parse_json(r#"{"exclude_tags": []}"#).unwrap();
        assert!(RunConfig::from_json(&val).unwrap().run_all);
    }

    #[test]
    fn run_config_no_filters_defaults_run_all() {
        let val = parse_json("{}").unwrap();
        let config = RunConfig::from_json(&val).unwrap();
        assert!(config.run_all);
    }

    #[test]
    fn run_config_explicit_run_all_wins() {
        let val = parse_json(r#"{"run_all": true, "include_tags": ["fast"]}"#).unwrap();
        let config = RunConfig::from_json(&val).unwrap();
        assert!(config.run_all);
    }

    #[test]
    fn run_config_mistyped_filters_error() {
        // A mistyped filter must be an error, never a silently-empty
        // filter that lets run_all default to true.
        for bad in [
            r#"{"include_ids": "t1"}"#,
            r#"{"include_ids": [1, 2]}"#,
            r#"{"include_tags": ["fast", 1]}"#,
            r#"{"exclude_tags": 3}"#,
            r#"{"name_pattern": 42}"#,
            r#"{"run_all": "yes"}"#,
            r#"{"fail_fast": "no"}"#,
            r#"{"timeout_ms": "5s"}"#,
            // Beyond exact-JSON-integer range: reject, never saturate.
            r#"{"timeout_ms": 1e300}"#,
            // An empty pattern would match every test — reject.
            r#"{"name_pattern": ""}"#,
        ] {
            let val = parse_json(bad).unwrap();
            assert!(RunConfig::from_json(&val).is_err(), "should reject: {}", bad);
        }
    }

    #[test]
    fn empty_filter_array_means_zero_tests_not_everything() {
        // {"include_ids": []} selected zero tests — run_all must NOT
        // default to true just because the array is empty.
        let val = parse_json(r#"{"include_ids": []}"#).unwrap();
        let config = RunConfig::from_json(&val).unwrap();
        assert!(!config.run_all);
        let val = parse_json(r#"{"include_tags": []}"#).unwrap();
        assert!(!RunConfig::from_json(&val).unwrap().run_all);
    }

    #[test]
    fn max_concurrency_without_type_errors() {
        let val = parse_json(r#"{"max_concurrency": 8}"#).unwrap();
        assert!(ExecutionModel::from_json(&val).is_err());
    }

    #[test]
    fn run_config_unknown_keys_error() {
        // A misspelled key must error, never act as "no filter".
        for bad in [
            r#"{"tags": ["fast"]}"#,
            r#"{"include_tag": ["fast"]}"#,
            r#"{"ids": ["t1"]}"#,
        ] {
            let val = parse_json(bad).unwrap();
            let err = RunConfig::from_json(&val).unwrap_err();
            assert!(format!("{}", err).contains("valid keys"), "should reject: {}", bad);
        }
        // Non-object params are rejected too.
        let val = parse_json(r#"["fast"]"#).unwrap();
        assert!(RunConfig::from_json(&val).is_err());
    }

    #[test]
    fn run_config_non_object_execution_model_errors() {
        let val = parse_json(r#"{"execution_model": "parallel"}"#).unwrap();
        assert!(RunConfig::from_json(&val).is_err());
    }

    #[test]
    fn run_config_fractional_timeout_errors() {
        let val = parse_json(r#"{"timeout_ms": 1.9}"#).unwrap();
        assert!(RunConfig::from_json(&val).is_err());
    }

    #[test]
    fn test_definition_mistyped_tags_and_metadata_error() {
        assert!(TestDefinition::from_json(
            &parse_json(r#"{"id": "t", "name": "x", "tags": ["fast", 5]}"#).unwrap()
        ).is_err());
        assert!(TestDefinition::from_json(
            &parse_json(r#"{"id": "t", "name": "x", "metadata": {"k": 5}}"#).unwrap()
        ).is_err());
        assert!(TestDefinition::from_json(
            &parse_json(r#"{"id": "t", "name": "x", "group": 3}"#).unwrap()
        ).is_err());
    }

    #[test]
    fn stored_exclude_only_config_stays_readable() {
        // Files written before the exclude-under-run_all rework contain
        // {"run_all": false, "exclude_tags": [...]} configs. History must
        // stay loadable even though caller input now rejects that shape.
        let old_file = r#"{
            "run_id": "run_0007",
            "config": {"run_all": false, "exclude_tags": ["slow"], "fail_fast": false,
                       "execution_model": {"type": "sequential"}},
            "results": [], "total": 2, "passed": 2, "failed": 0, "skipped": 0,
            "errored": 0, "total_duration_ms": 10, "started_at": 5, "completed_at": 15
        }"#;
        let val = parse_json(old_file).unwrap();
        let summary = RunSummary::from_json(&val).unwrap();
        assert_eq!(summary.run_id, "run_0007");
        assert!(!summary.config.run_all);
        // The same config shape as direct caller input still errors.
        let caller = parse_json(r#"{"run_all": false, "exclude_tags": ["slow"]}"#).unwrap();
        assert!(RunConfig::from_json(&caller).is_err());

        // Old lenient versions also accepted an empty name_pattern from
        // callers — stored history with it must stay readable, while
        // fresh caller input rejects it.
        let old_pattern = parse_json(r#"{
            "run_id": "run_0008",
            "config": {"run_all": false, "name_pattern": "", "fail_fast": false},
            "results": [], "total": 3, "passed": 3, "failed": 0, "skipped": 0,
            "errored": 0, "total_duration_ms": 9, "started_at": 1, "completed_at": 10
        }"#).unwrap();
        assert!(RunSummary::from_json(&old_pattern).is_ok());
    }

    #[test]
    fn test_definition_unknown_keys_error() {
        // registry.json is hand-editable — a misspelled field must error,
        // never silently drop.
        let val = parse_json(r#"{"id": "t1", "name": "x", "tag": ["smoke"]}"#).unwrap();
        let err = TestDefinition::from_json(&val).unwrap_err();
        assert!(format!("{}", err).contains("valid keys"));
    }

    #[test]
    fn run_summary_mistyped_fields_error() {
        // A damaged summary must load as an error (CorruptRun upstream),
        // never as a summary silently claiming zero results.
        for bad in [
            r#"{"run_id": "run_0001", "results": "oops", "total": 5}"#,
            r#"{"run_id": "run_0001", "results": [], "total": "five"}"#,
            r#"{"results": [], "total": 1}"#,
            // Out-of-range counters must reject, not wrap modulo 2^32.
            r#"{"run_id": "run_0001", "results": [], "total": 4294967296}"#,
            // A result entry missing its status or test_id is damage too.
            r#"{"run_id": "run_0001", "results": [{"test_id": "t1", "duration_ms": 5}]}"#,
            r#"{"run_id": "run_0001", "results": [{"status": "passed", "duration_ms": 5}]}"#,
        ] {
            let val = parse_json(bad).unwrap();
            assert!(RunSummary::from_json(&val).is_err(), "should reject: {}", bad);
        }
    }

    #[test]
    fn run_config_null_fields_are_absent() {
        // Explicit null is the JSON convention for "not set".
        let val = parse_json(r#"{"run_all": null, "include_tags": null, "name_pattern": null}"#).unwrap();
        let config = RunConfig::from_json(&val).unwrap();
        assert!(config.run_all);
        assert!(config.include_tags.is_empty());
    }

    #[test]
    fn execution_model_rejects_unknown_type() {
        // Mis-cased or misspelled types must not silently become Sequential.
        for bad in [
            r#"{"type": "Parallel"}"#,
            r#"{"type": "concurrent"}"#,
            r#"{"type": 7}"#,
            r#"{"type": "parallel", "max_concurrency": "eight"}"#,
            r#"{"type": "parallel", "max_concurrency": 0}"#,
            r#"{"type": "parallel", "max_concurrency": 5000000000}"#,
        ] {
            let val = parse_json(bad).unwrap();
            assert!(ExecutionModel::from_json(&val).is_err(), "should reject: {}", bad);
        }
        let val = parse_json(r#"{"type": "parallel", "max_concurrency": 8}"#).unwrap();
        assert_eq!(
            ExecutionModel::from_json(&val).unwrap(),
            ExecutionModel::Parallel { max_concurrency: 8 }
        );
        let val = parse_json(r#"{"type": "sequential"}"#).unwrap();
        assert_eq!(ExecutionModel::from_json(&val).unwrap(), ExecutionModel::Sequential);
    }

    #[test]
    fn discovery_query_mistyped_fields_error() {
        for bad in [
            r#"{"tags": "fast"}"#,
            r#"{"name_pattern": []}"#,
            r#"{"limit": "ten"}"#,
        ] {
            let val = parse_json(bad).unwrap();
            assert!(DiscoveryQuery::from_json(&val).is_err(), "should reject: {}", bad);
        }
    }

    #[test]
    fn test_definition_requires_id_and_name() {
        assert!(TestDefinition::from_json(&parse_json(r#"{"name": "x"}"#).unwrap()).is_err());
        assert!(TestDefinition::from_json(&parse_json(r#"{"id": "x"}"#).unwrap()).is_err());
        assert!(TestDefinition::from_json(&parse_json(r#"{"id": "", "name": "x"}"#).unwrap()).is_err());
        assert!(TestDefinition::from_json(&parse_json(r#"{"id": 7, "name": "x"}"#).unwrap()).is_err());
        assert!(TestDefinition::from_json(&parse_json(r#"{"id": "t", "name": "x"}"#).unwrap()).is_ok());
    }
}
