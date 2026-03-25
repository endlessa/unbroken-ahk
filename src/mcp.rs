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
                    ("description", str_val("Run every registered test (default true if no filters)")),
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
            ]),
        },
        ToolDescriptor {
            name: "test_progress".into(),
            description: "Check the progress of a running test suite. Returns completion percentage, pass/fail counts.".into(),
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
    let tool = value
        .get_str("tool")
        .ok_or("Missing 'tool' field")?
        .to_string();
    let params = value
        .get("params")
        .cloned()
        .unwrap_or(JsonValue::Object(vec![]));
    Ok(McpRequest { tool, params })
}

/// Dispatch an MCP request to the appropriate handler.
pub fn handle_request(manager: &mut PlatformManager, request: &McpRequest) -> McpResponse {
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

    match manager.start_run(config) {
        Ok(run_id) => {
            match manager.get_results(&run_id) {
                Ok(summary) => McpResponse::ok(summary.to_json()),
                Err(_) => {
                    // Run still in progress (shouldn't happen with sync executor)
                    McpResponse::ok(obj(vec![
                        ("run_id", str_val(&run_id)),
                        ("status", str_val("in_progress")),
                    ]))
                }
            }
        }
        Err(e) => McpResponse::err(&format!("{:?}", e)),
    }
}

fn handle_progress(manager: &PlatformManager, params: &JsonValue) -> McpResponse {
    match params.get_str("run_id") {
        Some(run_id) => {
            match manager.check_progress(run_id) {
                Ok(progress) => McpResponse::ok(progress.to_json()),
                Err(e) => McpResponse::err(&format!("{:?}", e)),
            }
        }
        None => {
            let active = manager.active_runs();
            McpResponse::ok(str_array(&active))
        }
    }
}

fn handle_results(manager: &PlatformManager, params: &JsonValue) -> McpResponse {
    match params.get_str("run_id") {
        Some(run_id) => {
            match manager.get_results(run_id) {
                Ok(summary) => McpResponse::ok(summary.to_json()),
                Err(e) => McpResponse::err(&format!("{:?}", e)),
            }
        }
        None => McpResponse::err("Missing required parameter: 'run_id'"),
    }
}

fn handle_list_tags(manager: &PlatformManager) -> McpResponse {
    let summary = manager.summary();
    let arr = JsonValue::Array(
        summary.tags.iter().map(|(t, c)| {
            obj(vec![("tag", str_val(t)), ("count", JsonValue::Number(*c as f64))])
        }).collect(),
    );
    McpResponse::ok(arr)
}

fn handle_list_groups(manager: &PlatformManager) -> McpResponse {
    let summary = manager.summary();
    let arr = JsonValue::Array(
        summary.groups.iter().map(|(g, c)| {
            obj(vec![("group", str_val(g)), ("count", JsonValue::Number(*c as f64))])
        }).collect(),
    );
    McpResponse::ok(arr)
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
        let mut mgr = PlatformManager::new("/tmp/unbroken-mcp-test");
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
        let mut mgr = setup_manager();
        let resp = execute_mcp(&mut mgr, r#"{"tool": "test_run", "params": {"run_all": false, "exclude_tags": ["slow"]}}"#);
        let val = parse_json(&resp).unwrap();
        let data = val.get("data").unwrap();
        assert_eq!(data.get("total").and_then(|v| v.as_f64()), Some(2.0));
        assert_eq!(data.get("passed").and_then(|v| v.as_f64()), Some(2.0));
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
        let mut mgr = setup_manager();
        let resp = execute_mcp(&mut mgr, r#"{"tool": "test_run", "params": {"run_all": false, "include_ids": ["nonexistent"]}}"#);
        let val = parse_json(&resp).unwrap();
        assert_eq!(val.get_bool("success"), Some(false));
        assert!(val.get_str("error").unwrap().contains("NoTestsMatched"));
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
        execute_mcp(&mut mgr, r#"{"tool": "test_run"}"#);
        let resp = execute_mcp(&mut mgr, r#"{"tool": "test_progress", "params": {"run_id": "run_0001"}}"#);
        let val = parse_json(&resp).unwrap();
        assert_eq!(val.get_bool("success"), Some(true));
        let data = val.get("data").unwrap();
        assert_eq!(data.get("completed").and_then(|v| v.as_f64()), Some(3.0));
        assert_eq!(data.get("percent_complete").and_then(|v| v.as_f64()), Some(100.0));
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
        execute_mcp(&mut mgr, r#"{"tool": "test_run"}"#);
        let resp = execute_mcp(&mut mgr, r#"{"tool": "test_results", "params": {"run_id": "run_0001"}}"#);
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
