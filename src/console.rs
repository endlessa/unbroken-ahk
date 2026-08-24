//! Console interface for human operators.
//!
//! Parses text commands, dispatches to the PlatformManager, and
//! formats output for terminal display. All I/O is JSON under the hood.

use crate::discovery::DiscoveryQuery;
use crate::impl_manager::PlatformManager;
use crate::json::{parse_json, FromJson, ToJson, to_json_pretty};
use crate::manager::TestManager;
use crate::reporter::ReportFormat;
use crate::types::RunConfig;

/// Result of processing a console command.
#[derive(Debug)]
pub struct ConsoleOutput {
    pub text: String,
    pub json: String,
}

/// Parse and execute a console command against the platform manager.
///
/// Supported commands:
///   help                          — Show available commands
///   summary                       — Overview of all registered tests
///   discover                      — List all tests
///   discover <pattern>            — Search tests by name
///   discover --tag <tag>          — Filter tests by tag
///   discover --group <group>      — Filter tests by group
///   run                           — Run all tests
///   run <json_config>             — Run with JSON configuration
///   run --tag <tag>               — Run tests matching a tag
///   run --id <id1> <id2> ...      — Run specific tests by ID
///   progress <run_id>             — Check progress of a run
///   results <run_id>              — Get results of a completed run
///   tags                          — List all available tags
///   groups                        — List all available groups
pub fn execute_command(manager: &mut PlatformManager, input: &str) -> ConsoleOutput {
    let input = input.trim();
    if input.is_empty() {
        return ConsoleOutput {
            text: "Type 'help' for available commands.".into(),
            json: "{}".into(),
        };
    }

    let parts: Vec<&str> = split_args(input);
    let command = parts[0].to_lowercase();
    let args = &parts[1..];
    // The raw text after the command word, whitespace preserved — needed
    // for inline JSON configs, where tokenizing would corrupt whitespace
    // inside string values.
    let rest = input[parts[0].len()..].trim_start();

    match command.as_str() {
        "help" => cmd_help(),
        "summary" => cmd_summary(manager),
        "discover" | "search" | "find" => cmd_discover(manager, args),
        "run" | "execute" | "start" => cmd_run(manager, args, rest),
        "progress" | "status" => cmd_progress(manager, args),
        "results" | "result" => cmd_results(manager, args),
        "tags" => cmd_tags(manager),
        "groups" => cmd_groups(manager),
        _ => ConsoleOutput {
            text: format!("Unknown command: '{}'. Type 'help' for available commands.", command),
            json: "{}".into(),
        },
    }
}

fn cmd_help() -> ConsoleOutput {
    let text = "\
=== Unbroken Test Platform ===

Commands:
  summary                        Overview of all registered tests
  discover                       List all tests
  discover <pattern>             Search tests by name pattern
  discover --pattern <pattern>   Same; use --pattern=<p> if it starts with '-'
  discover --tag <tag>           Filter tests by tag
  discover --group <group>       Filter tests by group
  tags                           List all available tags
  groups                         List all available groups
  run                            Run all tests
  run --tag <tag>                Run tests matching a tag
  run --id <id1> <id2> ...       Run specific tests by ID
  run --fail-fast                Stop on first failure
  run <json>                     Run with JSON configuration
  progress <run_id>              Check progress of a running suite
  results <run_id>               Get results of a completed run
  help                           Show this message
";
    ConsoleOutput {
        text: text.into(),
        json: "{}".into(),
    }
}

fn cmd_summary(manager: &PlatformManager) -> ConsoleOutput {
    let summary = manager.summary();
    let json = to_json_pretty(&summary.to_json());
    let mut text = format!("Total tests: {}\n", summary.total_tests);
    if !summary.tags.is_empty() {
        text.push_str("\nTags:\n");
        for (tag, count) in &summary.tags {
            text.push_str(&format!("  {} ({})\n", tag, count));
        }
    }
    if !summary.groups.is_empty() {
        text.push_str("\nGroups:\n");
        for (group, count) in &summary.groups {
            text.push_str(&format!("  {} ({})\n", group, count));
        }
    }
    ConsoleOutput { text, json }
}

fn cmd_discover(manager: &PlatformManager, args: &[&str]) -> ConsoleOutput {
    let mut query = DiscoveryQuery::default();

    let mut i = 0;
    while i < args.len() {
        let (flag, inline) = split_flag(args[i]);
        match flag {
            "--tag" | "-t" => match take_value(inline, args, i, "--tag") {
                Ok((v, used)) => {
                    query.tags.push(v.to_string());
                    i += used;
                }
                Err(e) => return error_output(&e),
            },
            // Scalar flags reject repetition: last-wins would silently
            // drop the first value — the same intent-drop class the
            // unknown-flag and typo checks exist to prevent. (--tag and
            // --id accumulate; that contrast is why this must be loud.)
            "--group" | "-g" => match take_value(inline, args, i, "--group") {
                Ok((v, used)) => {
                    if query.group.is_some() {
                        return error_output("--group given more than once — discover takes at most one group");
                    }
                    query.group = Some(v.to_string());
                    i += used;
                }
                Err(e) => return error_output(&e),
            },
            "--limit" | "-l" => match take_value(inline, args, i, "--limit") {
                Ok((v, used)) => match v.parse() {
                    Ok(n) => {
                        if query.limit.is_some() {
                            return error_output("--limit given more than once");
                        }
                        query.limit = Some(n);
                        i += used;
                    }
                    Err(_) => {
                        return error_output(&format!("--limit: '{}' is not a number", v))
                    }
                },
                Err(e) => return error_output(&e),
            },
            // Same pattern as the bare argument, but reachable for
            // patterns that begin with '-' via --pattern=<value> —
            // a bare one would be rejected as an unknown flag.
            "--pattern" | "-p" => match take_value(inline, args, i, "--pattern") {
                Ok((v, used)) => {
                    if query.name_pattern.is_some() {
                        return error_output(
                            "multiple name patterns given — discover takes at most one",
                        );
                    }
                    query.name_pattern = Some(v.to_string());
                    i += used;
                }
                Err(e) => return error_output(&e),
            },
            // A misspelled flag must error, not silently become a name
            // pattern that searches for the wrong thing.
            other if other.starts_with('-') => {
                return error_output(&format!("unknown flag '{}'", other));
            }
            other => {
                if query.name_pattern.is_some() {
                    return error_output(
                        "multiple name patterns given — discover takes at most one",
                    );
                }
                query.name_pattern = Some(other.to_string());
                i += 1;
            }
        }
    }

    let result = manager.discover(&query);
    let json = to_json_pretty(&result.to_json());

    let mut text = format!("Found {} test(s):\n\n", result.total_matches);
    for test in &result.tests {
        let tags_str = if test.tags.is_empty() {
            String::new()
        } else {
            format!(" [{}]", test.tags.join(", "))
        };
        let group_str = match &test.group {
            Some(g) => format!(" ({})", g),
            None => String::new(),
        };
        text.push_str(&format!("  {} — {}{}{}\n", test.id, test.name, group_str, tags_str));
        if let Some(ref desc) = test.description {
            text.push_str(&format!("    {}\n", desc));
        }
    }

    if !result.available_tags.is_empty() {
        text.push_str(&format!("\nAvailable tags: {}\n", result.available_tags.join(", ")));
    }
    if !result.available_groups.is_empty() {
        text.push_str(&format!("Available groups: {}\n", result.available_groups.join(", ")));
    }

    ConsoleOutput { text, json }
}

fn cmd_run(manager: &mut PlatformManager, args: &[&str], rest: &str) -> ConsoleOutput {
    let config = if args.is_empty() {
        RunConfig::default()
    } else if rest.starts_with('{') {
        // JSON config — parse the raw remainder so whitespace inside
        // string values survives intact.
        match parse_json(rest) {
            Ok(val) => match RunConfig::from_json(&val) {
                Ok(c) => c,
                Err(e) => return error_output(&format!("Invalid config JSON: {}", e)),
            },
            Err(e) => return error_output(&format!("Invalid JSON: {}", e)),
        }
    } else {
        // Parse flag-based config
        match parse_run_args(args) {
            Ok(c) => c,
            Err(e) => return error_output(&e),
        }
    };

    let config_json = to_json_pretty(&config.to_json());

    match manager.start_run(config) {
        Ok(run_id) => {
            match manager.get_results(&run_id) {
                Ok(summary) => {
                    let reporter = crate::impl_reporter::StandardReporter::new();
                    let text = crate::reporter::TestReporter::format_summary(
                        &reporter, &summary, ReportFormat::Text,
                    );
                    let json = to_json_pretty(&summary.to_json());
                    ConsoleOutput { text, json }
                }
                Err(_) => ConsoleOutput {
                    text: format!("Run started: {}\nUse 'progress {}' to check status.", run_id, run_id),
                    json: config_json,
                },
            }
        }
        Err(crate::manager::ManagerError::PersistFailed(run_id, msg)) => error_output(&format!(
            "Run {} EXECUTED but its summary could not be persisted ({}). \
             Use 'results {}' to see the outcome — do not re-run.",
            run_id, msg, run_id
        )),
        Err(e) => error_output(&format!("Run failed: {}", e)),
    }
}

fn cmd_progress(manager: &PlatformManager, args: &[&str]) -> ConsoleOutput {
    if args.is_empty() {
        let active = manager.active_runs();
        if active.is_empty() {
            return ConsoleOutput {
                text: "No active runs.".into(),
                json: "[]".into(),
            };
        }
        let text = format!("Active runs: {}\n", active.join(", "));
        let json = to_json_pretty(&crate::json::str_array(&active));
        return ConsoleOutput { text, json };
    }

    match manager.check_progress(args[0]) {
        Ok(progress) => {
            let reporter = crate::impl_reporter::StandardReporter::new();
            let text = crate::reporter::TestReporter::format_progress(
                &reporter, &progress, ReportFormat::Text,
            );
            let json = to_json_pretty(&progress.to_json());
            ConsoleOutput { text, json }
        }
        Err(e) => error_output(&format!("{}", e)),
    }
}

fn cmd_results(manager: &PlatformManager, args: &[&str]) -> ConsoleOutput {
    if args.is_empty() {
        return error_output("Usage: results <run_id>");
    }

    match manager.get_results(args[0]) {
        Ok(summary) => {
            let reporter = crate::impl_reporter::StandardReporter::new();
            let text = crate::reporter::TestReporter::format_summary(
                &reporter, &summary, ReportFormat::Text,
            );
            let json = to_json_pretty(&summary.to_json());
            ConsoleOutput { text, json }
        }
        Err(e) => error_output(&format!("{}", e)),
    }
}

fn cmd_tags(manager: &PlatformManager) -> ConsoleOutput {
    let summary = manager.summary();
    let mut text = String::from("Tags:\n");
    for (tag, count) in &summary.tags {
        text.push_str(&format!("  {} ({})\n", tag, count));
    }
    if summary.tags.is_empty() {
        text.push_str("  (none)\n");
    }
    let json = to_json_pretty(&crate::json_types::counts_json("tag", &summary.tags));
    ConsoleOutput { text, json }
}

fn cmd_groups(manager: &PlatformManager) -> ConsoleOutput {
    let summary = manager.summary();
    let mut text = String::from("Groups:\n");
    for (group, count) in &summary.groups {
        text.push_str(&format!("  {} ({})\n", group, count));
    }
    if summary.groups.is_empty() {
        text.push_str("  (none)\n");
    }
    let json = to_json_pretty(&crate::json_types::counts_json("group", &summary.groups));
    ConsoleOutput { text, json }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_run_args(args: &[&str]) -> Result<RunConfig, String> {
    let mut config = RunConfig {
        run_all: false,
        ..Default::default()
    };

    let mut i = 0;
    while i < args.len() {
        let (flag, inline) = split_flag(args[i]);
        match flag {
            "--all" | "-a" => {
                if inline.is_some() {
                    return Err("--all takes no value".into());
                }
                config.run_all = true;
                i += 1;
            }
            "--tag" | "-t" => {
                let (v, used) = take_value(inline, args, i, "--tag")?;
                config.include_tags.push(v.to_string());
                i += used;
            }
            "--exclude" | "-e" => {
                let (v, used) = take_value(inline, args, i, "--exclude")?;
                config.exclude_tags.push(v.to_string());
                i += used;
            }
            "--id" => {
                let mut got = false;
                if let Some(v) = inline {
                    if v.is_empty() {
                        return Err("--id requires at least one test ID".into());
                    }
                    config.include_ids.push(v.to_string());
                    got = true;
                }
                i += 1;
                while i < args.len() && !args[i].starts_with('-') {
                    config.include_ids.push(args[i].to_string());
                    got = true;
                    i += 1;
                }
                if !got {
                    return Err("--id requires at least one test ID; use \
                                --id=<id> for an ID that begins with '-'"
                        .into());
                }
            }
            // Scalar flags reject repetition — see cmd_discover's guard:
            // last-wins would silently not run the first pattern's tests.
            "--pattern" | "-p" => {
                let (v, used) = take_value(inline, args, i, "--pattern")?;
                if config.name_pattern.is_some() {
                    return Err(
                        "--pattern given more than once — run takes at most one name pattern"
                            .into(),
                    );
                }
                config.name_pattern = Some(v.to_string());
                i += used;
            }
            "--fail-fast" | "-f" => {
                if inline.is_some() {
                    return Err("--fail-fast takes no value".into());
                }
                config.fail_fast = true;
                i += 1;
            }
            "--timeout" => {
                let (v, used) = take_value(inline, args, i, "--timeout")?;
                if config.timeout_ms.is_some() {
                    return Err("--timeout given more than once".into());
                }
                let ms: u64 = v
                    .parse()
                    .map_err(|_| format!("--timeout: '{}' is not a number", v))?;
                // Same bound the JSON config path enforces: a value above
                // 2^53 would persist clamped — a timeout the caller never
                // asked for.
                if ms > crate::json_types::MAX_SAFE_JSON_INT as u64 {
                    return Err(format!(
                        "--timeout: '{}' exceeds the exact JSON integer range (2^53 - 1)",
                        v
                    ));
                }
                config.timeout_ms = Some(ms);
                i += used;
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown flag '{}'", other));
            }
            _ => {
                // Treat bare args as test IDs
                config.include_ids.push(args[i].to_string());
                i += 1;
            }
        }
    }

    // If no include-side filters were set, run everything. Exclusions
    // don't count: they apply under run_all, so 'run --exclude slow'
    // means "everything except slow".
    if !config.has_include_filters() {
        config.run_all = true;
    }

    Ok(config)
}

fn split_args(input: &str) -> Vec<&str> {
    input.split_whitespace().collect()
}

/// Split "--flag=value" into ("--flag", Some("value")). Only tokens that
/// look like flags are split — a bare value containing '=' is untouched.
fn split_flag(arg: &str) -> (&str, Option<&str>) {
    if arg.starts_with('-') {
        if let Some((flag, value)) = arg.split_once('=') {
            return (flag, Some(value));
        }
    }
    (arg, None)
}

/// The value for a value-taking flag, in either spelling:
///   --flag value  — the next argument, rejected when it looks like a
///                   flag (silently consuming a typo'd flag would change
///                   the run's meaning);
///   --flag=value  — inline, where the value may be ANYTHING — the
///                   escape hatch for legitimately registered tags,
///                   patterns, and ids that begin with '-'.
/// Returns the value and how many argument tokens were consumed.
fn take_value<'a>(
    inline: Option<&'a str>,
    args: &[&'a str],
    i: usize,
    flag: &str,
) -> Result<(&'a str, usize), String> {
    if let Some(v) = inline {
        if v.is_empty() {
            return Err(format!("{} requires a value", flag));
        }
        return Ok((v, 1));
    }
    match args.get(i + 1) {
        Some(&next) if !next.starts_with('-') => Ok((next, 2)),
        Some(next) => Err(format!(
            "{} requires a value; '{}' looks like a flag — use {}=<value> \
             to pass a value that begins with '-'",
            flag, next, flag
        )),
        None => Err(format!("{} requires a value", flag)),
    }
}

fn error_output(message: &str) -> ConsoleOutput {
    ConsoleOutput {
        text: format!("Error: {}", message),
        json: to_json_pretty(&crate::json::obj(vec![
            ("error", crate::json::str_val(message)),
        ])),
    }
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
        let mut mgr = PlatformManager::new(&crate::test_util::temp_storage_dir("console-test"));
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

    #[test]
    fn help_command() {
        let mut mgr = setup_manager();
        let out = execute_command(&mut mgr, "help");
        assert!(out.text.contains("Commands:"));
        assert!(out.text.contains("discover"));
        assert!(out.text.contains("run"));
    }

    #[test]
    fn empty_input() {
        let mut mgr = setup_manager();
        let out = execute_command(&mut mgr, "");
        assert!(out.text.contains("help"));
    }

    #[test]
    fn unknown_command() {
        let mut mgr = setup_manager();
        let out = execute_command(&mut mgr, "foobar");
        assert!(out.text.contains("Unknown command"));
    }

    #[test]
    fn summary_command() {
        let mut mgr = setup_manager();
        let out = execute_command(&mut mgr, "summary");
        assert!(out.text.contains("Total tests: 3"));
        assert!(out.text.contains("smoke"));
        assert!(out.text.contains("auth"));
        assert!(out.json.contains("\"total_tests\": 3"));
    }

    #[test]
    fn discover_all() {
        let mut mgr = setup_manager();
        let out = execute_command(&mut mgr, "discover");
        assert!(out.text.contains("Found 3 test(s)"));
        assert!(out.text.contains("auth_basic"));
        assert!(out.text.contains("net_ping"));
    }

    #[test]
    fn discover_by_pattern() {
        let mut mgr = setup_manager();
        let out = execute_command(&mut mgr, "discover auth_*");
        assert!(out.text.contains("Found 2 test(s)"));
        assert!(out.text.contains("auth_basic"));
        assert!(out.text.contains("auth_token"));
    }

    #[test]
    fn discover_by_tag() {
        let mut mgr = setup_manager();
        let out = execute_command(&mut mgr, "discover --tag slow");
        assert!(out.text.contains("Found 1 test(s)"));
        assert!(out.text.contains("net_ping"));
    }

    #[test]
    fn discover_by_group() {
        let mut mgr = setup_manager();
        let out = execute_command(&mut mgr, "discover --group auth");
        assert!(out.text.contains("Found 2 test(s)"));
    }

    #[test]
    fn run_all() {
        let mut mgr = setup_manager();
        let out = execute_command(&mut mgr, "run");
        assert!(out.text.contains("Run Summary"));
        assert!(out.text.contains("Total: 3"));
        assert!(out.text.contains("Passed: 2"));
        assert!(out.text.contains("Failed: 1"));
        // JSON output should also be present
        assert!(out.json.contains("\"passed\": 2"));
    }

    #[test]
    fn run_by_tag() {
        let mut mgr = setup_manager();
        let out = execute_command(&mut mgr, "run --tag smoke");
        assert!(out.text.contains("Passed: 2"));
        // Should not include the slow test (results list test IDs, so
        // assert on the id — the name never appears in run output).
        assert!(!out.text.contains(" t3 "));
    }

    #[test]
    fn run_by_id() {
        let mut mgr = setup_manager();
        let out = execute_command(&mut mgr, "run --id t1 t3");
        assert!(out.text.contains("Total: 2"));
    }

    #[test]
    fn run_with_fail_fast() {
        let mut mgr = setup_manager();
        // t3 fails, so with fail_fast the run should stop and skip remaining
        let out = execute_command(&mut mgr, "run --id t3 t1 --fail-fast");
        assert!(out.text.contains("FAIL"));
    }

    #[test]
    fn run_with_json_config() {
        let mut mgr = setup_manager();
        let out = execute_command(&mut mgr, r#"run {"run_all": false, "include_tags": ["smoke"]}"#);
        assert!(out.text.contains("Passed: 2"));
    }

    #[test]
    fn progress_no_active() {
        let mut mgr = setup_manager();
        let out = execute_command(&mut mgr, "progress");
        assert!(out.text.contains("No active runs"));
    }

    #[test]
    fn results_after_run() {
        let mut mgr = setup_manager();
        let run_out = execute_command(&mut mgr, "run");
        let run_id = crate::json::parse_json(&run_out.json).unwrap()
            .get_str("run_id").unwrap().to_string();
        let out = execute_command(&mut mgr, &format!("results {}", run_id));
        assert!(out.text.contains("Run Summary"));
    }

    #[test]
    fn results_missing_run() {
        let mut mgr = setup_manager();
        let out = execute_command(&mut mgr, "results bogus_id");
        assert!(out.text.contains("Error"));
    }

    #[test]
    fn run_flag_missing_value_errors() {
        // A truncated command must error, never fall back to running
        // the entire suite.
        let mut mgr = setup_manager();
        let out = execute_command(&mut mgr, "run --tag");
        assert!(out.text.contains("Error"));
        assert!(out.text.contains("--tag requires a value"));

        let out = execute_command(&mut mgr, "run --id");
        assert!(out.text.contains("Error"));

        let out = execute_command(&mut mgr, "run --timeout abc");
        assert!(out.text.contains("Error"));

        // The console spelling enforces the same 2^53 bound as the JSON
        // config path — a clamped-on-write timeout is a value the caller
        // never asked for.
        let out = execute_command(&mut mgr, "run --timeout 18446744073709551615");
        assert!(out.text.contains("exceeds"), "got: {}", out.text);

        let out = execute_command(&mut mgr, "run --bogus-flag");
        assert!(out.text.contains("unknown flag"));

        // A flag as the "value" of a value-taking flag means the value was
        // forgotten — must error, never consume the flag — and the error
        // must name the =<value> escape hatch.
        let out = execute_command(&mut mgr, "run --tag --fail-fast");
        assert!(out.text.contains("--tag requires a value"));
        assert!(out.text.contains("--tag=<value>"), "got: {}", out.text);
        let out = execute_command(&mut mgr, "discover --tag --group auth");
        assert!(out.text.contains("--tag requires a value"));
    }

    #[test]
    fn dash_prefixed_values_pass_via_equals_form() {
        // Tags are unvalidated at registration, so a tag beginning with
        // '-' is legal — the =<value> spelling must be able to address it
        // even though the space-separated spelling rejects flag-alikes.
        let mut mgr = setup_manager();
        mgr.register_runnable(
            TestDefinition {
                id: "t9".into(),
                name: "weird_tag_test".into(),
                tags: vec!["-net".into()],
                group: None,
                description: None,
                metadata: vec![],
            },
            Box::new(StubTest { id: "t9".into(), pass: true }),
        )
        .unwrap();

        let out = execute_command(&mut mgr, "discover --tag=-net");
        assert!(out.text.contains("weird_tag_test"), "got: {}", out.text);

        let out = execute_command(&mut mgr, "run --exclude=-net");
        assert!(out.text.contains("Total: 3"), "got: {}", out.text);
        assert!(!out.json.contains("\"t9\""));

        // Inline values reach the same validation as spaced ones.
        let out = execute_command(&mut mgr, "discover --limit=-3");
        assert!(out.text.contains("not a number"), "got: {}", out.text);
        // Boolean flags take no value in either spelling.
        let out = execute_command(&mut mgr, "run --fail-fast=yes");
        assert!(out.text.contains("takes no value"), "got: {}", out.text);
    }

    #[test]
    fn repeated_scalar_flags_error_instead_of_last_wins() {
        // Last-wins would silently drop the first value (and its tests);
        // list flags (--tag, --id) accumulate, so scalars must be loud.
        let mut mgr = setup_manager();
        let out = execute_command(&mut mgr, "run --pattern auth_* --pattern net_*");
        assert!(out.text.contains("more than once"), "got: {}", out.text);
        let out = execute_command(&mut mgr, "discover --group auth --group network");
        assert!(out.text.contains("more than once"), "got: {}", out.text);
        let out = execute_command(&mut mgr, "run --timeout 5 --timeout 10");
        assert!(out.text.contains("more than once"), "got: {}", out.text);
        let out = execute_command(&mut mgr, "discover --limit 1 --limit 2");
        assert!(out.text.contains("more than once"), "got: {}", out.text);
    }

    #[test]
    fn dash_prefixed_name_pattern_reachable_via_pattern_flag() {
        // Names are unvalidated, so a name beginning with '-' is legal.
        // A bare dash-prefixed pattern reads as an unknown flag (typo
        // safety), so discover needs --pattern=<value> to reach it.
        let mut mgr = setup_manager();
        mgr.register_runnable(
            TestDefinition {
                id: "t8".into(),
                name: "-dash_name".into(),
                tags: vec![],
                group: None,
                description: None,
                metadata: vec![],
            },
            Box::new(StubTest { id: "t8".into(), pass: true }),
        )
        .unwrap();

        let out = execute_command(&mut mgr, "discover -dash_name");
        assert!(out.text.contains("unknown flag"), "got: {}", out.text);

        let out = execute_command(&mut mgr, "discover --pattern=-dash*");
        assert!(out.text.contains("-dash_name"), "got: {}", out.text);

        // The spaced spelling works for ordinary patterns too, and the
        // one-pattern rule spans both spellings.
        let out = execute_command(&mut mgr, "discover --pattern auth_*");
        assert!(out.text.contains("auth_basic"), "got: {}", out.text);
        let out = execute_command(&mut mgr, "discover auth_* --pattern=net_*");
        assert!(out.text.contains("at most one"), "got: {}", out.text);
    }

    #[test]
    fn discover_bad_limit_errors() {
        let mut mgr = setup_manager();
        let out = execute_command(&mut mgr, "discover --limit abc");
        assert!(out.text.contains("Error"));
        assert!(out.text.contains("not a number"));
    }

    #[test]
    fn run_exclude_only_runs_the_rest() {
        let mut mgr = setup_manager();
        let out = execute_command(&mut mgr, "run --exclude slow");
        assert!(out.text.contains("Total: 2"), "got: {}", out.text);
        // Run output carries test IDs, not names — assert on the id.
        assert!(!out.json.contains("\"t3\""));
    }

    #[test]
    fn discover_unknown_flag_errors() {
        let mut mgr = setup_manager();
        let out = execute_command(&mut mgr, "discover --tga slow");
        assert!(out.text.contains("unknown flag"));
        let out = execute_command(&mut mgr, "discover auth_basic net_ping");
        assert!(out.text.contains("multiple name patterns"));
    }

    #[test]
    fn run_json_config_preserves_inner_whitespace() {
        let mut mgr = setup_manager();
        mgr.register_runnable(
            TestDefinition {
                id: "ws".into(),
                name: "double  space".into(),
                tags: vec![],
                group: None,
                description: None,
                metadata: vec![],
            },
            Box::new(StubTest { id: "ws".into(), pass: true }),
        ).unwrap();
        // The two spaces inside the pattern must survive tokenization.
        let out = execute_command(
            &mut mgr,
            r#"run {"run_all": false, "name_pattern": "double  space"}"#,
        );
        assert!(out.text.contains("Total: 1"), "got: {}", out.text);
        assert!(out.json.contains("\"ws\""));
    }

    #[test]
    fn tags_command() {
        let mut mgr = setup_manager();
        let out = execute_command(&mut mgr, "tags");
        assert!(out.text.contains("smoke"));
        assert!(out.text.contains("fast"));
        assert!(out.text.contains("slow"));
    }

    #[test]
    fn groups_command() {
        let mut mgr = setup_manager();
        let out = execute_command(&mut mgr, "groups");
        assert!(out.text.contains("auth"));
        assert!(out.text.contains("network"));
    }

    #[test]
    fn discover_shows_description() {
        let mut mgr = setup_manager();
        let out = execute_command(&mut mgr, "discover auth_basic");
        assert!(out.text.contains("Basic authentication test"));
    }

    #[test]
    fn console_output_has_both_formats() {
        let mut mgr = setup_manager();
        let out = execute_command(&mut mgr, "run");
        // Text format for humans
        assert!(out.text.contains("==="));
        // JSON format for debugging
        assert!(out.json.starts_with('{'));
    }
}
