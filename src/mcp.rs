//! MCP Tool interface for AI agents.
//!
//! Exposes the test platform as a set of MCP-style tools. Each tool
//! takes JSON input and returns JSON output. The AI sends a tool name
//! and parameters, the platform dispatches and responds.
//!
//! Tools:
//!   test_summary      — Overview of registered tests
//!   test_discover     — Search and filter available tests
//!   test_run          — Execute tests with a configuration
//!   test_progress     — Check progress of a running suite
//!   test_results      — Get results of a completed run
//!   test_list_tags    — List all available tags
//!   test_list_groups  — List all available groups
//!   tool_list         — List all available MCP tools

use crate::discovery::DiscoveryQuery;
use crate::impl_manager::PlatformManager;
use crate::json::*;
use crate::manager::TestManager;
use crate::types::RunConfig;

// ---------------------------------------------------------------------------
// MCP Tool Descriptor
// ---------------------------------------------------------------------------

/// Describes an available MCP tool for the AI to discover.
#[derive(Debug, Clone)]
pub struct ToolDescriptor {
    pub name: String,
    pub description: String,
    pub parameters: JsonValue,
}

impl ToJson for ToolDescriptor {
    fn to_json(&self) -> JsonValue {
        obj(vec![
            ("name", str_val(&self.name)),
            ("description", str_val(&self.description)),
            ("parameters", self.parameters.clone()),
        ])
    }
}

// ---------------------------------------------------------------------------
// MCP Request / Response
// ---------------------------------------------------------------------------

/// An incoming MCP tool call.
#[derive(Debug)]
pub struct McpRequest {
    pub tool: String,
    pub params: JsonValue,
}

/// The response from an MCP tool call.
#[derive(Debug)]
pub struct McpResponse {
    pub success: bool,
    pub data: JsonValue,
    pub error: Option<String>,
}

impl ToJson for McpResponse {
    fn to_json(&self) -> JsonValue {
        let mut pairs: Vec<(&str, JsonValue)> = vec![
            ("success", JsonValue::Bool(self.success)),
            ("data", self.data.clone()),
        ];
        if let Some(ref e) = self.error {
            pairs.push(("error", str_val(e)));
        }
        obj(pairs)
    }
}

impl McpResponse {
    fn ok(data: JsonValue) -> Self {
        Self { success: true, data, error: None }
    }

    fn err(message: &str) -> Self {
        Self {
            success: false,
            data: JsonValue::Null,
            error: Some(message.to_string()),
        }
    }
}

// ---------------------------------------------------------------------------
// Tool Definitions
// ---------------------------------------------------------------------------

/// Returns descriptors for all available MCP tools.
pub fn list_tools() -> Vec<ToolDescriptor> {
    vec![
        ToolDescriptor {
            name: "tool_list".into(),
            description: "List all available MCP tools with their descriptions and parameters.".into(),
            parameters: obj(vec![]),
        },
        ToolDescriptor {
            name: "test_summary".into(),
            description: "Get a high-level overview of all registered tests, including total count, available tags, and groups.".into(),
            parameters: obj(vec![]),
        },
        ToolDescriptor {
            name: "test_discover".into(),
            description: "Search and filter available tests. Returns matching tests with metadata.".into(),
            parameters: obj(vec![
                ("name_pattern", obj(vec![
                    ("type", str_val("string")),
                    ("description", str_val("Search test names by substring or glob pattern (e.g. 'auth_*')")),
                    ("required", JsonValue::Bool(false)),
                ])),
                ("tags", obj(vec![
                    ("type", str_val("array")),
                    ("description", str_val("Filter to tests with ALL of these tags")),
                    ("required", JsonValue::Bool(false)),
                ])),
                ("group", obj(vec![
                    ("type", str_val("string")),
                    ("description", str_val("Filter to tests in this group")),
                    ("required", JsonValue::Bool(false)),
                ])),
                ("limit", obj(vec![
                    ("type", str_val("number")),
                    ("description", str_val("Maximum results to return")),
                    ("required", JsonValue::Bool(false)),
                ])),
                ("offset", obj(vec![
                    ("type", str_val("number")),
                    ("description", str_val("Skip this many results (pagination)")),
                    ("required", JsonValue::Bool(false)),
                ])),
            ]),
        },
        ToolDescriptor {
            name: "test_run".into(),
            description: "Execute tests. Run all tests, or filter by IDs, tags, name pattern. Returns a run_id and full results.".into(),
            parameters: obj(vec![
                ("run_all", obj(vec![
                    ("type", str_val("boolean")),
                    ("description", str_val("Start from every registered test (exclude_tags still applies; combining with include filters is rejected as contradictory). Defaults to true unless include_ids, include_tags, or name_pattern is supplied — exclude_tags alone keeps run_all true, running everything except the excluded tags.")),
                    ("required", JsonValue::Bool(false)),
                ])),
                ("include_ids", obj(vec![
                    ("type", str_val("array")),
                    ("description", str_val("Run only these specific test IDs")),
                    ("required", JsonValue::Bool(false)),
                ])),
                ("include_tags", obj(vec![
                    ("type", str_val("array")),
                    ("description", str_val("Run tests matching ALL of these tags")),
                    ("required", JsonValue::Bool(false)),
                ])),
                ("exclude_tags", obj(vec![
                    ("type", str_val("array")),
                    ("description", str_val("Exclude tests matching ANY of these tags")),
                    ("required", JsonValue::Bool(false)),
                ])),
                ("name_pattern", obj(vec![
                    ("type", str_val("string")),
                    ("description", str_val("Run tests matching this name pattern")),
                    ("required", JsonValue::Bool(false)),
                ])),
                ("fail_fast", obj(vec![
                    ("type", str_val("boolean")),
                    ("description", str_val("Stop on first failure (default false)")),
                    ("required", JsonValue::Bool(false)),
                ])),
                ("timeout_ms", obj(vec![
                    ("type", str_val("number")),
                    ("description", str_val("Per-test timeout in milliseconds")),
                    ("required", JsonValue::Bool(false)),
                ])),
                ("execution_model", obj(vec![
                    ("type", str_val("object")),
                    ("description", str_val("Execution strategy, e.g. {\"type\": \"sequential\"}. Only \"sequential\" is currently supported; \"parallel\" is accepted syntax but rejected at run time.")),
                    ("required", JsonValue::Bool(false)),
                ])),
            ]),
        },
        ToolDescriptor {
            name: "test_progress".into(),
            description: "Check the progress of a running test suite. Returns completion percentage, pass/fail counts, and a 'finished' flag. Poll until finished is true — not until percent_complete reaches 100, which a finished legacy run may truthfully never report.".into(),
            parameters: obj(vec![
                ("run_id", obj(vec![
                    ("type", str_val("string")),
                    ("description", str_val("The run ID to check. Omit to list all active runs.")),
                    ("required", JsonValue::Bool(false)),
                ])),
            ]),
        },
        ToolDescriptor {
            name: "test_results".into(),
            description: "Get the full results of a completed test run including per-test status, timing, and output.".into(),
            parameters: obj(vec![
                ("run_id", obj(vec![
                    ("type", str_val("string")),
                    ("description", str_val("The run ID to get results for")),
                    ("required", JsonValue::Bool(true)),
                ])),
            ]),
        },
        ToolDescriptor {
            name: "test_list_tags".into(),
            description: "List all available tags across registered tests with counts.".into(),
            parameters: obj(vec![]),
        },
        ToolDescriptor {
            name: "test_list_groups".into(),
            description: "List all available groups across registered tests with counts.".into(),
            parameters: obj(vec![]),
        },
    ]
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

/// Parse a JSON string into an MCP request.
pub fn parse_request(json_input: &str) -> Result<McpRequest, String> {
    let value = parse_json(json_input).map_err(|e| format!("Invalid JSON: {}", e))?;
    // The envelope gets the same strictness as every inner parser: a
    // client sending {"arguments": {...}} (the MCP-spec spelling) must
    // hear "unknown field", never have its filters silently dropped and
    // the whole suite run instead.
    crate::json_types::reject_unknown_keys(&value, "request", &["tool", "params"])
        .map_err(|e| format!("{}", e))?;
    // Present-but-mistyped must not be reported as "missing" — the same
    // precision run_id_param gives one layer down.
    let tool = match value.get("tool") {
        Some(JsonValue::Str(s)) => s.clone(),
        Some(other) => {
            return Err(format!(
                "Field 'tool' must be a string, got: {}",
                crate::json::to_json_compact(other)
            ))
        }
        None => return Err("Missing 'tool' field".into()),
    };
    // Absent params — or explicit null, the JSON convention for "not set" —
    // means an empty parameter object.
    let params = match value.get("params") {
        None | Some(JsonValue::Null) => JsonValue::Object(vec![]),
        Some(other) => other.clone(),
    };
    Ok(McpRequest { tool, params })
}

/// Dispatch an MCP request to the appropriate handler.
pub fn handle_request(manager: &mut PlatformManager, request: &McpRequest) -> McpResponse {
    // Parameter-less tools reject supplied params like every other
    // handler rejects unknown keys: {"tool": "test_list_tags",
    // "params": {"group": "auth"}} answered with the FULL unfiltered
    // list reads as a filtered result.
    if matches!(
        request.tool.as_str(),
        "tool_list" | "test_summary" | "test_list_tags" | "test_list_groups"
    ) && !matches!(&request.params, JsonValue::Object(pairs) if pairs.is_empty())
    {
        return McpResponse::err(&format!(
            "'{}' takes no parameters (use test_discover to filter tests)",
            request.tool
        ));
    }
    match request.tool.as_str() {
        "tool_list" => handle_tool_list(),
        "test_summary" => handle_summary(manager),
        "test_discover" => handle_discover(manager, &request.params),
        "test_run" => handle_run(manager, &request.params),
        "test_progress" => handle_progress(manager, &request.params),
        "test_results" => handle_results(manager, &request.params),
        "test_list_tags" => handle_list_tags(manager),
        "test_list_groups" => handle_list_groups(manager),
        _ => McpResponse::err(&format!("Unknown tool: '{}'", request.tool)),
    }
}

/// Convenience: parse JSON input and dispatch in one call.
pub fn execute_mcp(manager: &mut PlatformManager, json_input: &str) -> String {
    match parse_request(json_input) {
        Ok(request) => {
            let response = handle_request(manager, &request);
            to_json_pretty(&response.to_json())
        }
        Err(e) => {
            let response = McpResponse::err(&e);
            to_json_pretty(&response.to_json())
        }
    }
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

fn handle_tool_list() -> McpResponse {
    let tools = list_tools();
    let arr = JsonValue::Array(tools.iter().map(|t| t.to_json()).collect());
    McpResponse::ok(arr)
}

fn handle_summary(manager: &PlatformManager) -> McpResponse {
    let summary = manager.summary();
    McpResponse::ok(summary.to_json())
}

fn handle_discover(manager: &PlatformManager, params: &JsonValue) -> McpResponse {
    let query = match DiscoveryQuery::from_json(params) {
        Ok(q) => q,
        Err(e) => return McpResponse::err(&format!("Invalid params: {}", e)),
    };
    let result = manager.discover(&query);
    McpResponse::ok(result.to_json())
}

fn handle_run(manager: &mut PlatformManager, params: &JsonValue) -> McpResponse {
    let config = match RunConfig::from_json(params) {
        Ok(c) => c,
        Err(e) => return McpResponse::err(&format!("Invalid config: {}", e)),
    };

    // The summary comes straight from the run — no disk read-back that
    // could transiently fail and misreport a finished run as
    // "in_progress".
    match manager.run_to_completion(config) {
        Ok(summary) => McpResponse::ok(summary.to_json()),
        // The tests EXECUTED — a retry would run everything again. Say so
        // machine-readably so an agent fetches results instead of re-running.
        Err(crate::manager::ManagerError::PersistFailed(run_id, msg)) => McpResponse {
            success: false,
            data: obj(vec![
                ("run_id", str_val(&run_id)),
                ("executed", JsonValue::Bool(true)),
            ]),
            error: Some(format!(
                "Run {} EXECUTED but its summary could not be persisted ({}). \
                 Results are queryable via test_results with this run_id — do NOT re-run.",
                run_id, msg
            )),
        },
        Err(e) => McpResponse::err(&format!("{}", e)),
    }
}

/// Shared extraction of the optional run_id parameter.
///
/// Ok(Some(id)): a string run_id was supplied. Ok(None): absent, or
/// explicit null (the JSON convention for "not set"). Err: non-object
/// params or a non-string run_id — caller mistakes that must error rather
/// than silently act like "not set".
fn run_id_param(params: &JsonValue) -> Result<Option<&str>, String> {
    if !matches!(params, JsonValue::Object(_)) {
        return Err(format!(
            "Parameters must be an object like {{\"run_id\": \"...\"}}, got: {}",
            crate::json::to_json_compact(params)
        ));
    }
    // A misspelled key ("runId") must error — silently treating it as
    // "no run_id" switches test_progress to the list-active-runs
    // semantics and the caller reads [] as "the run vanished".
    crate::json_types::reject_unknown_keys(params, "params", &["run_id"])
        .map_err(|e| format!("{}", e))?;
    match params.get("run_id") {
        Some(JsonValue::Str(run_id)) => Ok(Some(run_id)),
        None | Some(JsonValue::Null) => Ok(None),
        Some(other) => Err(format!(
            "Parameter 'run_id' must be a string, got: {}",
            crate::json::to_json_compact(other)
        )),
    }
}

fn handle_progress(manager: &PlatformManager, params: &JsonValue) -> McpResponse {
    match run_id_param(params) {
        Ok(Some(run_id)) => match manager.check_progress(run_id) {
            Ok(progress) => McpResponse::ok(progress.to_json()),
            Err(e) => McpResponse::err(&format!("{}", e)),
        },
        // No run_id means "list the active runs".
        Ok(None) => McpResponse::ok(str_array(&manager.active_runs())),
        Err(e) => McpResponse::err(&e),
    }
}

fn handle_results(manager: &PlatformManager, params: &JsonValue) -> McpResponse {
    match run_id_param(params) {
        Ok(Some(run_id)) => match manager.get_results(run_id) {
            Ok(summary) => McpResponse::ok(summary.to_json()),
            Err(e) => McpResponse::err(&format!("{}", e)),
        },
        Ok(None) => McpResponse::err("Missing required parameter: 'run_id'"),
        Err(e) => McpResponse::err(&e),
    }
}

fn handle_list_tags(manager: &PlatformManager) -> McpResponse {
    McpResponse::ok(crate::json_types::counts_json("tag", &manager.summary().tags))
}

fn handle_list_groups(manager: &PlatformManager) -> McpResponse {
    McpResponse::ok(crate::json_types::counts_json("group", &manager.summary().groups))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::RunnableTest;
    use crate::types::{DurationMs, TestDefinition, TestResult, TestStatus};

    struct StubTest {
        id: String,
        pass: bool,
    }

    impl RunnableTest for StubTest {
        fn id(&self) -> &str {
            &self.id
        }
        fn run(&self, _timeout: Option<DurationMs>) -> TestResult {
            TestResult {
                test_id: self.id.clone(),
                status: if self.pass { TestStatus::Passed } else { TestStatus::Failed },
                duration_ms: 5,
                message: if self.pass { None } else { Some("assertion failed".into()) },
                stdout: Some("output".into()),
                stderr: None,
            }
        }
    }

    fn setup_manager() -> PlatformManager {
        let mut mgr = PlatformManager::new(&crate::test_util::temp_storage_dir("mcp-test"));
        mgr.register_runnable(
            TestDefinition {
                id: "t1".into(),
                name: "auth_basic".into(),
                tags: vec!["smoke".into(), "fast".into()],
                group: Some("auth".into()),
                description: Some("Basic authentication test".into()),
                metadata: vec![],
            },
            Box::new(StubTest { id: "t1".into(), pass: true }),
        ).unwrap();
        mgr.register_runnable(
            TestDefinition {
                id: "t2".into(),
                name: "auth_token".into(),
                tags: vec!["smoke".into()],
                group: Some("auth".into()),
                description: None,
                metadata: vec![],
            },
            Box::new(StubTest { id: "t2".into(), pass: true }),
        ).unwrap();
        mgr.register_runnable(
            TestDefinition {
                id: "t3".into(),
                name: "net_ping".into(),
                tags: vec!["slow".into()],
                group: Some("network".into()),
                description: None,
                metadata: vec![],
            },
            Box::new(StubTest { id: "t3".into(), pass: false }),
        ).unwrap();
        mgr
    }

    // -- tool_list --

    #[test]
    fn tool_list_returns_all_tools() {
        let mut mgr = setup_manager();
        let resp = execute_mcp(&mut mgr, r#"{"tool": "tool_list"}"#);
        let val = parse_json(&resp).unwrap();
        assert_eq!(val.get_bool("success"), Some(true));
        let data = val.get("data").unwrap().as_array().unwrap();
        assert!(data.len() >= 8);
        // Should include our core tools
        let names: Vec<&str> = data.iter().filter_map(|t| t.get_str("name")).collect();
        assert!(names.contains(&"test_summary"));
        assert!(names.contains(&"test_discover"));
        assert!(names.contains(&"test_run"));
        assert!(names.contains(&"test_progress"));
        assert!(names.contains(&"test_results"));
    }

    // -- test_summary --

    #[test]
    fn summary_returns_counts() {
        let mut mgr = setup_manager();
        let resp = execute_mcp(&mut mgr, r#"{"tool": "test_summary"}"#);
        let val = parse_json(&resp).unwrap();
        assert_eq!(val.get_bool("success"), Some(true));
        let data = val.get("data").unwrap();
        assert_eq!(data.get("total_tests").and_then(|v| v.as_f64()), Some(3.0));
    }

    // -- test_discover --

    #[test]
    fn discover_all_tests() {
        let mut mgr = setup_manager();
        let resp = execute_mcp(&mut mgr, r#"{"tool": "test_discover"}"#);
        let val = parse_json(&resp).unwrap();
        assert_eq!(val.get_bool("success"), Some(true));
        let data = val.get("data").unwrap();
        assert_eq!(data.get("total_matches").and_then(|v| v.as_f64()), Some(3.0));
    }

    #[test]
    fn discover_by_name_pattern() {
        let mut mgr = setup_manager();
        let resp = execute_mcp(&mut mgr, r#"{"tool": "test_discover", "params": {"name_pattern": "auth_*"}}"#);
        let val = parse_json(&resp).unwrap();
        let data = val.get("data").unwrap();
        assert_eq!(data.get("total_matches").and_then(|v| v.as_f64()), Some(2.0));
    }

    #[test]
    fn discover_by_tag() {
        let mut mgr = setup_manager();
        let resp = execute_mcp(&mut mgr, r#"{"tool": "test_discover", "params": {"tags": ["slow"]}}"#);
        let val = parse_json(&resp).unwrap();
        let data = val.get("data").unwrap();
        assert_eq!(data.get("total_matches").and_then(|v| v.as_f64()), Some(1.0));
    }

    #[test]
    fn discover_by_group() {
        let mut mgr = setup_manager();
        let resp = execute_mcp(&mut mgr, r#"{"tool": "test_discover", "params": {"group": "auth"}}"#);
        let val = parse_json(&resp).unwrap();
        let data = val.get("data").unwrap();
        assert_eq!(data.get("total_matches").and_then(|v| v.as_f64()), Some(2.0));
    }

    #[test]
    fn discover_with_pagination() {
        let mut mgr = setup_manager();
        let resp = execute_mcp(&mut mgr, r#"{"tool": "test_discover", "params": {"limit": 1, "offset": 0}}"#);
        let val = parse_json(&resp).unwrap();
        let data = val.get("data").unwrap();
        assert_eq!(data.get("total_matches").and_then(|v| v.as_f64()), Some(3.0));
        let tests = data.get("tests").unwrap().as_array().unwrap();
        assert_eq!(tests.len(), 1);
    }

    // -- test_run --

    #[test]
    fn run_all_tests() {
        let mut mgr = setup_manager();
        let resp = execute_mcp(&mut mgr, r#"{"tool": "test_run"}"#);
        let val = parse_json(&resp).unwrap();
        assert_eq!(val.get_bool("success"), Some(true));
        let data = val.get("data").unwrap();
        assert_eq!(data.get("total").and_then(|v| v.as_f64()), Some(3.0));
        assert_eq!(data.get("passed").and_then(|v| v.as_f64()), Some(2.0));
        assert_eq!(data.get("failed").and_then(|v| v.as_f64()), Some(1.0));
    }

    #[test]
    fn run_by_tag() {
        let mut mgr = setup_manager();
        let resp = execute_mcp(&mut mgr, r#"{"tool": "test_run", "params": {"run_all": false, "include_tags": ["smoke"]}}"#);
        let val = parse_json(&resp).unwrap();
        let data = val.get("data").unwrap();
        assert_eq!(data.get("total").and_then(|v| v.as_f64()), Some(2.0));
        assert_eq!(data.get("passed").and_then(|v| v.as_f64()), Some(2.0));
    }

    #[test]
    fn run_by_ids() {
        let mut mgr = setup_manager();
        let resp = execute_mcp(&mut mgr, r#"{"tool": "test_run", "params": {"run_all": false, "include_ids": ["t1", "t3"]}}"#);
        let val = parse_json(&resp).unwrap();
        let data = val.get("data").unwrap();
        assert_eq!(data.get("total").and_then(|v| v.as_f64()), Some(2.0));
        assert_eq!(data.get("passed").and_then(|v| v.as_f64()), Some(1.0));
        assert_eq!(data.get("failed").and_then(|v| v.as_f64()), Some(1.0));
    }

    #[test]
    fn run_with_exclude() {
        // Exclude-only means "everything except these" — run_all defaults
        // true and the exclusion still applies.
        let mut mgr = setup_manager();
        let resp = execute_mcp(&mut mgr, r#"{"tool": "test_run", "params": {"exclude_tags": ["slow"]}}"#);
        let val = parse_json(&resp).unwrap();
        let data = val.get("data").unwrap();
        assert_eq!(data.get("total").and_then(|v| v.as_f64()), Some(2.0));
        assert_eq!(data.get("passed").and_then(|v| v.as_f64()), Some(2.0));

        // An explicit run_all=false with only exclusions selects nothing —
        // and says so with the explanatory parse error (exclusions cannot
        // resurrect an empty selection).
        let resp = execute_mcp(&mut mgr, r#"{"tool": "test_run", "params": {"run_all": false, "exclude_tags": ["slow"]}}"#);
        let val = parse_json(&resp).unwrap();
        assert_eq!(val.get_bool("success"), Some(false));
        assert!(val.get_str("error").unwrap().contains("nothing is selected"));
    }

    #[test]
    fn run_with_fail_fast() {
        let mut mgr = setup_manager();
        // Run all 3 tests with fail_fast — t3 fails, so at least one should be skipped
        let resp = execute_mcp(&mut mgr, r#"{"tool": "test_run", "params": {"fail_fast": true}}"#);
        let val = parse_json(&resp).unwrap();
        let data = val.get("data").unwrap();
        let results = data.get("results").unwrap().as_array().unwrap();
        let statuses: Vec<&str> = results.iter().filter_map(|r| r.get_str("status")).collect();
        assert!(statuses.contains(&"failed"));
        // With fail_fast and 3 tests where one fails, we expect at least one skip
        let has_skip_or_fewer_runs = statuses.contains(&"skipped") || results.len() < 3;
        assert!(has_skip_or_fewer_runs || statuses.iter().filter(|s| **s == "failed").count() >= 1);
    }

    #[test]
    fn run_no_match_returns_error() {
        // A misspelled/unknown id errors and NAMES the id — it must never
        // silently shrink the run.
        let mut mgr = setup_manager();
        let resp = execute_mcp(&mut mgr, r#"{"tool": "test_run", "params": {"run_all": false, "include_ids": ["nonexistent"]}}"#);
        let val = parse_json(&resp).unwrap();
        assert_eq!(val.get_bool("success"), Some(false));
        let err = val.get_str("error").unwrap();
        assert!(err.contains("not registered") && err.contains("nonexistent"));

        // Even when another include criterion matches, the typo still errors.
        let resp = execute_mcp(&mut mgr, r#"{"tool": "test_run", "params": {"include_ids": ["t1", "tpyo3"]}}"#);
        let val = parse_json(&resp).unwrap();
        assert_eq!(val.get_bool("success"), Some(false));
        assert!(val.get_str("error").unwrap().contains("tpyo3"));
    }

    // -- test_progress --

    #[test]
    fn progress_no_active_runs() {
        let mut mgr = setup_manager();
        let resp = execute_mcp(&mut mgr, r#"{"tool": "test_progress"}"#);
        let val = parse_json(&resp).unwrap();
        assert_eq!(val.get_bool("success"), Some(true));
        let data = val.get("data").unwrap().as_array().unwrap();
        assert!(data.is_empty());
    }

    #[test]
    fn progress_after_run_completed() {
        let mut mgr = setup_manager();
        let run_resp = execute_mcp(&mut mgr, r#"{"tool": "test_run"}"#);
        let run_id = parse_json(&run_resp).unwrap()
            .get("data").unwrap().get_str("run_id").unwrap().to_string();
        let resp = execute_mcp(&mut mgr, &format!(
            r#"{{"tool": "test_progress", "params": {{"run_id": "{}"}}}}"#, run_id));
        let val = parse_json(&resp).unwrap();
        assert_eq!(val.get_bool("success"), Some(true));
        let data = val.get("data").unwrap();
        assert_eq!(data.get("completed").and_then(|v| v.as_f64()), Some(3.0));
        assert_eq!(data.get("percent_complete").and_then(|v| v.as_f64()), Some(100.0));
        // The poll-until signal an agent should watch.
        assert_eq!(data.get_bool("finished"), Some(true));
    }

    #[test]
    fn progress_unknown_run() {
        let mut mgr = setup_manager();
        let resp = execute_mcp(&mut mgr, r#"{"tool": "test_progress", "params": {"run_id": "bogus"}}"#);
        let val = parse_json(&resp).unwrap();
        assert_eq!(val.get_bool("success"), Some(false));
    }

    // -- test_results --

    #[test]
    fn results_after_run() {
        let mut mgr = setup_manager();
        let run_resp = execute_mcp(&mut mgr, r#"{"tool": "test_run"}"#);
        let run_id = parse_json(&run_resp).unwrap()
            .get("data").unwrap().get_str("run_id").unwrap().to_string();
        let resp = execute_mcp(&mut mgr, &format!(
            r#"{{"tool": "test_results", "params": {{"run_id": "{}"}}}}"#, run_id));
        let val = parse_json(&resp).unwrap();
        assert_eq!(val.get_bool("success"), Some(true));
        let data = val.get("data").unwrap();
        assert_eq!(data.get("total").and_then(|v| v.as_f64()), Some(3.0));
    }

    #[test]
    fn results_missing_run_id() {
        let mut mgr = setup_manager();
        let resp = execute_mcp(&mut mgr, r#"{"tool": "test_results"}"#);
        let val = parse_json(&resp).unwrap();
        assert_eq!(val.get_bool("success"), Some(false));
        assert!(val.get_str("error").unwrap().contains("run_id"));
    }

    #[test]
    fn results_unknown_run() {
        let mut mgr = setup_manager();
        let resp = execute_mcp(&mut mgr, r#"{"tool": "test_results", "params": {"run_id": "bogus"}}"#);
        let val = parse_json(&resp).unwrap();
        assert_eq!(val.get_bool("success"), Some(false));
    }

    // -- test_list_tags / test_list_groups --

    #[test]
    fn list_tags() {
        let mut mgr = setup_manager();
        let resp = execute_mcp(&mut mgr, r#"{"tool": "test_list_tags"}"#);
        let val = parse_json(&resp).unwrap();
        assert_eq!(val.get_bool("success"), Some(true));
        let data = val.get("data").unwrap().as_array().unwrap();
        let tag_names: Vec<&str> = data.iter().filter_map(|t| t.get_str("tag")).collect();
        assert!(tag_names.contains(&"smoke"));
        assert!(tag_names.contains(&"fast"));
        assert!(tag_names.contains(&"slow"));
    }

    #[test]
    fn list_groups() {
        let mut mgr = setup_manager();
        let resp = execute_mcp(&mut mgr, r#"{"tool": "test_list_groups"}"#);
        let val = parse_json(&resp).unwrap();
        assert_eq!(val.get_bool("success"), Some(true));
        let data = val.get("data").unwrap().as_array().unwrap();
        let group_names: Vec<&str> = data.iter().filter_map(|g| g.get_str("group")).collect();
        assert!(group_names.contains(&"auth"));
        assert!(group_names.contains(&"network"));
    }

    // -- Error handling --

    #[test]
    fn unknown_tool() {
        let mut mgr = setup_manager();
        let resp = execute_mcp(&mut mgr, r#"{"tool": "nonexistent"}"#);
        let val = parse_json(&resp).unwrap();
        assert_eq!(val.get_bool("success"), Some(false));
        assert!(val.get_str("error").unwrap().contains("Unknown tool"));
    }

    #[test]
    fn invalid_json_input() {
        let mut mgr = setup_manager();
        let resp = execute_mcp(&mut mgr, "not json at all");
        let val = parse_json(&resp).unwrap();
        assert_eq!(val.get_bool("success"), Some(false));
        assert!(val.get_str("error").unwrap().contains("Invalid JSON"));
    }

    #[test]
    fn missing_tool_field() {
        let mut mgr = setup_manager();
        let resp = execute_mcp(&mut mgr, r#"{"params": {}}"#);
        let val = parse_json(&resp).unwrap();
        assert_eq!(val.get_bool("success"), Some(false));
        assert!(val.get_str("error").unwrap().contains("tool"));
    }

    #[test]
    fn empty_params_defaults() {
        let mut mgr = setup_manager();
        // test_run with empty params should default to run_all
        let resp = execute_mcp(&mut mgr, r#"{"tool": "test_run", "params": {}}"#);
        let val = parse_json(&resp).unwrap();
        assert_eq!(val.get_bool("success"), Some(true));
        let data = val.get("data").unwrap();
        assert_eq!(data.get("total").and_then(|v| v.as_f64()), Some(3.0));
    }

    #[test]
    fn run_filters_without_run_all_are_honored() {
        // The MCP contract: filters alone imply run_all=false. Passing only
        // include_tags must NOT run the whole suite.
        let mut mgr = setup_manager();
        let resp = execute_mcp(&mut mgr, r#"{"tool": "test_run", "params": {"include_tags": ["slow"]}}"#);
        let val = parse_json(&resp).unwrap();
        assert_eq!(val.get_bool("success"), Some(true));
        let data = val.get("data").unwrap();
        assert_eq!(data.get("total").and_then(|v| v.as_f64()), Some(1.0));

        let mut mgr = setup_manager();
        let resp = execute_mcp(&mut mgr, r#"{"tool": "test_run", "params": {"exclude_tags": ["slow"]}}"#);
        let val = parse_json(&resp).unwrap();
        let data = val.get("data").unwrap();
        assert_eq!(data.get("total").and_then(|v| v.as_f64()), Some(2.0));
    }

    #[test]
    fn progress_rejects_mistyped_run_id() {
        let mut mgr = setup_manager();
        let resp = execute_mcp(&mut mgr, r#"{"tool": "test_progress", "params": {"run_id": 1}}"#);
        let val = parse_json(&resp).unwrap();
        assert_eq!(val.get_bool("success"), Some(false));
        assert!(val.get_str("error").unwrap().contains("must be a string"));

        let resp = execute_mcp(&mut mgr, r#"{"tool": "test_results", "params": {"run_id": 1}}"#);
        let val = parse_json(&resp).unwrap();
        assert_eq!(val.get_bool("success"), Some(false));
        assert!(val.get_str("error").unwrap().contains("must be a string"));
    }

    #[test]
    fn run_mistyped_filter_returns_error_not_full_suite() {
        // include_ids as a string (common client mistake) must error,
        // never fall back to running every test.
        let mut mgr = setup_manager();
        let resp = execute_mcp(&mut mgr, r#"{"tool": "test_run", "params": {"include_ids": "t1"}}"#);
        let val = parse_json(&resp).unwrap();
        assert_eq!(val.get_bool("success"), Some(false));
        assert!(val.get_str("error").unwrap().contains("include_ids"));
    }

    #[test]
    fn run_all_with_include_filters_is_rejected() {
        // Contradictory intent must never silently widen the run past
        // the includes (the destructive tests a tag was scoping out).
        let mut mgr = setup_manager();
        let resp = execute_mcp(
            &mut mgr,
            r#"{"tool": "test_run", "params": {"run_all": true, "include_tags": ["smoke"]}}"#,
        );
        let val = parse_json(&resp).unwrap();
        assert_eq!(val.get_bool("success"), Some(false));
        assert!(val
            .get_str("error")
            .unwrap()
            .contains("conflicts with include filters"));
    }

    #[test]
    fn parameterless_tools_reject_supplied_params() {
        // A "filtered" request answered with the full unfiltered list
        // reads as a filtered result — reject loudly instead.
        let mut mgr = setup_manager();
        let resp = execute_mcp(
            &mut mgr,
            r#"{"tool": "test_list_tags", "params": {"group": "auth"}}"#,
        );
        let val = parse_json(&resp).unwrap();
        assert_eq!(val.get_bool("success"), Some(false));
        assert!(val.get_str("error").unwrap().contains("takes no parameters"));
        // Absent and null params still work.
        let resp = execute_mcp(&mut mgr, r#"{"tool": "test_list_tags", "params": null}"#);
        let val = parse_json(&resp).unwrap();
        assert_eq!(val.get_bool("success"), Some(true));
    }

    #[test]
    fn mistyped_tool_field_is_not_reported_missing() {
        let mut mgr = setup_manager();
        let resp = execute_mcp(&mut mgr, r#"{"tool": 42, "params": {}}"#);
        let val = parse_json(&resp).unwrap();
        assert_eq!(val.get_bool("success"), Some(false));
        let err = val.get_str("error").unwrap();
        assert!(err.contains("must be a string"), "got: {}", err);
        assert!(!err.contains("Missing"), "got: {}", err);
    }

    #[test]
    fn envelope_unknown_keys_error() {
        // "arguments" is the MCP-spec spelling — an easy client mistake
        // that must error, never silently drop the filters and run the
        // whole suite as {"params": {}}.
        let mut mgr = setup_manager();
        let resp = execute_mcp(
            &mut mgr,
            r#"{"tool": "test_run", "arguments": {"include_ids": ["t1"]}}"#,
        );
        let val = parse_json(&resp).unwrap();
        assert_eq!(val.get_bool("success"), Some(false));
        assert!(val.get_str("error").unwrap().contains("arguments"));
    }

    #[test]
    fn run_id_param_unknown_keys_error() {
        // A misspelled run_id key must not silently switch test_progress
        // to "list active runs" — [] would read as "the run vanished".
        let mut mgr = setup_manager();
        let resp = execute_mcp(
            &mut mgr,
            r#"{"tool": "test_progress", "params": {"runId": "run_0001"}}"#,
        );
        let val = parse_json(&resp).unwrap();
        assert_eq!(val.get_bool("success"), Some(false));
        assert!(val.get_str("error").unwrap().contains("runId"));
    }

    #[test]
    fn run_unknown_filter_key_errors() {
        // "tags" is test_discover's field name, not test_run's — an easy
        // agent mistake that must error, never run the whole suite.
        let mut mgr = setup_manager();
        let resp = execute_mcp(&mut mgr, r#"{"tool": "test_run", "params": {"tags": ["fast"]}}"#);
        let val = parse_json(&resp).unwrap();
        assert_eq!(val.get_bool("success"), Some(false));
        assert!(val.get_str("error").unwrap().contains("valid keys"));
    }

    #[test]
    fn run_all_with_exclude_honors_exclusion() {
        let mut mgr = setup_manager();
        let resp = execute_mcp(&mut mgr, r#"{"tool": "test_run", "params": {"run_all": true, "exclude_tags": ["slow"]}}"#);
        let val = parse_json(&resp).unwrap();
        assert_eq!(val.get_bool("success"), Some(true));
        let data = val.get("data").unwrap();
        assert_eq!(data.get("total").and_then(|v| v.as_f64()), Some(2.0));
    }

    #[test]
    fn null_params_treated_as_absent() {
        let mut mgr = setup_manager();
        let resp = execute_mcp(&mut mgr, r#"{"tool": "test_run", "params": null}"#);
        let val = parse_json(&resp).unwrap();
        assert_eq!(val.get_bool("success"), Some(true));
        assert_eq!(val.get("data").unwrap().get("total").and_then(|v| v.as_f64()), Some(3.0));
    }

    #[test]
    fn non_object_params_error_on_progress_and_results() {
        let mut mgr = setup_manager();
        let resp = execute_mcp(&mut mgr, r#"{"tool": "test_progress", "params": "run_0001"}"#);
        let val = parse_json(&resp).unwrap();
        assert_eq!(val.get_bool("success"), Some(false));
        assert!(val.get_str("error").unwrap().contains("must be an object"));

        let resp = execute_mcp(&mut mgr, r#"{"tool": "test_results", "params": "run_0001"}"#);
        let val = parse_json(&resp).unwrap();
        assert_eq!(val.get_bool("success"), Some(false));
    }

    #[test]
    fn empty_exclude_tags_runs_everything() {
        // "Exclude nothing" is a full run, not NoTestsMatched.
        let mut mgr = setup_manager();
        let resp = execute_mcp(&mut mgr, r#"{"tool": "test_run", "params": {"exclude_tags": []}}"#);
        let val = parse_json(&resp).unwrap();
        assert_eq!(val.get_bool("success"), Some(true));
        assert_eq!(val.get("data").unwrap().get("total").and_then(|v| v.as_f64()), Some(3.0));
    }

    #[test]
    fn empty_include_ids_is_no_tests_matched() {
        let mut mgr = setup_manager();
        let resp = execute_mcp(&mut mgr, r#"{"tool": "test_run", "params": {"include_ids": []}}"#);
        let val = parse_json(&resp).unwrap();
        assert_eq!(val.get_bool("success"), Some(false));
        assert!(val.get_str("error").unwrap().contains("no tests matched"));
    }

    #[test]
    fn bare_run_all_false_gets_explanatory_error() {
        // {"run_all": false} with no filter keys must name the cause and
        // the fix, not just report NoTestsMatched.
        let mut mgr = setup_manager();
        let resp = execute_mcp(&mut mgr, r#"{"tool": "test_run", "params": {"run_all": false}}"#);
        let val = parse_json(&resp).unwrap();
        assert_eq!(val.get_bool("success"), Some(false));
        assert!(val.get_str("error").unwrap().contains("nothing is selected"));
    }

    #[test]
    fn progress_null_run_id_lists_active_runs() {
        // Explicit null is the JSON convention for "not set" — same as absent.
        let mut mgr = setup_manager();
        let resp = execute_mcp(&mut mgr, r#"{"tool": "test_progress", "params": {"run_id": null}}"#);
        let val = parse_json(&resp).unwrap();
        assert_eq!(val.get_bool("success"), Some(true));
        assert!(val.get("data").unwrap().as_array().unwrap().is_empty());
    }

    #[test]
    fn parallel_model_returns_error() {
        let mut mgr = setup_manager();
        let resp = execute_mcp(&mut mgr,
            r#"{"tool": "test_run", "params": {"execution_model": {"type": "parallel", "max_concurrency": 8}}}"#);
        let val = parse_json(&resp).unwrap();
        assert_eq!(val.get_bool("success"), Some(false));
        assert!(val.get_str("error").unwrap().contains("parallel execution is not supported"));
    }

    // -- Full workflow (AI perspective) --

    #[test]
    fn full_ai_workflow() {
        let mut mgr = setup_manager();

        // Step 1: AI discovers what tools are available
        let resp = execute_mcp(&mut mgr, r#"{"tool": "tool_list"}"#);
        let val = parse_json(&resp).unwrap();
        assert!(val.get_bool("success").unwrap());

        // Step 2: AI checks what tests exist
        let resp = execute_mcp(&mut mgr, r#"{"tool": "test_summary"}"#);
        let val = parse_json(&resp).unwrap();
        let total = val.get("data").unwrap().get("total_tests").unwrap().as_f64().unwrap();
        assert_eq!(total, 3.0);

        // Step 3: AI discovers auth tests
        let resp = execute_mcp(&mut mgr, r#"{"tool": "test_discover", "params": {"group": "auth"}}"#);
        let val = parse_json(&resp).unwrap();
        let matches = val.get("data").unwrap().get("total_matches").unwrap().as_f64().unwrap();
        assert_eq!(matches, 2.0);

        // Step 4: AI runs just the smoke tests
        let resp = execute_mcp(&mut mgr, r#"{"tool": "test_run", "params": {"run_all": false, "include_tags": ["smoke"]}}"#);
        let val = parse_json(&resp).unwrap();
        let data = val.get("data").unwrap();
        assert_eq!(data.get("passed").and_then(|v| v.as_f64()), Some(2.0));
        let run_id = data.get_str("run_id").unwrap().to_string();

        // Step 5: AI retrieves results by run_id
        let resp = execute_mcp(&mut mgr, &format!(r#"{{"tool": "test_results", "params": {{"run_id": "{}"}}}}"#, run_id));
        let val = parse_json(&resp).unwrap();
        assert!(val.get_bool("success").unwrap());
        let data = val.get("data").unwrap();
        assert_eq!(data.get("passed").and_then(|v| v.as_f64()), Some(2.0));
    }
}
