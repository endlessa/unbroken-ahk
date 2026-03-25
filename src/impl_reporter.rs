//! Concrete implementation of TestReporter.
//!
//! Produces JSON (for AI/MCP) or human-readable text (for console).

use crate::json::{ToJson, to_json_pretty};
use crate::reporter::{ReportFormat, TestReporter};
use crate::types::{RunProgress, RunSummary, TestStatus};

/// Standard reporter supporting JSON and text output.
pub struct StandardReporter;

impl StandardReporter {
    pub fn new() -> Self {
        Self
    }
}

impl TestReporter for StandardReporter {
    fn format_summary(&self, summary: &RunSummary, format: ReportFormat) -> String {
        match format {
            ReportFormat::Json => to_json_pretty(&summary.to_json()),
            ReportFormat::Text => format_summary_text(summary),
        }
    }

    fn format_progress(&self, progress: &RunProgress, format: ReportFormat) -> String {
        match format {
            ReportFormat::Json => to_json_pretty(&progress.to_json()),
            ReportFormat::Text => format_progress_text(progress),
        }
    }
}

fn format_summary_text(s: &RunSummary) -> String {
    let mut out = String::new();
    out.push_str(&format!("=== Run Summary: {} ===\n", s.run_id));
    out.push_str(&format!(
        "Total: {}  Passed: {}  Failed: {}  Skipped: {}  Errored: {}\n",
        s.total, s.passed, s.failed, s.skipped, s.errored
    ));
    out.push_str(&format!("Duration: {}ms\n", s.total_duration_ms));
    out.push('\n');

    // List failures first
    let failures: Vec<_> = s.results.iter().filter(|r| {
        matches!(r.status, TestStatus::Failed | TestStatus::Error)
    }).collect();

    if !failures.is_empty() {
        out.push_str("--- Failures ---\n");
        for f in &failures {
            let status_str = match f.status {
                TestStatus::Failed => "FAIL",
                TestStatus::Error => "ERROR",
                _ => "",
            };
            out.push_str(&format!("  [{}] {} ({}ms)\n", status_str, f.test_id, f.duration_ms));
            if let Some(ref msg) = f.message {
                out.push_str(&format!("    {}\n", msg));
            }
        }
        out.push('\n');
    }

    // List all results
    out.push_str("--- All Results ---\n");
    for r in &s.results {
        let status_str = match r.status {
            TestStatus::Passed => "PASS",
            TestStatus::Failed => "FAIL",
            TestStatus::Error => "ERROR",
            TestStatus::Skipped => "SKIP",
        };
        out.push_str(&format!("  [{}] {} ({}ms)\n", status_str, r.test_id, r.duration_ms));
    }

    out
}

fn format_progress_text(p: &RunProgress) -> String {
    let bar_width = 30;
    let filled = if p.total > 0 {
        (p.completed as usize * bar_width) / p.total as usize
    } else {
        0
    };
    let empty = bar_width - filled;

    let bar: String = core::iter::repeat('#')
        .take(filled)
        .chain(core::iter::repeat('-').take(empty))
        .collect();

    format!(
        "[{}] {:.1}% ({}/{}) | P:{} F:{} S:{} | {}ms elapsed",
        bar,
        p.percent_complete,
        p.completed,
        p.total,
        p.passed,
        p.failed,
        p.skipped,
        p.elapsed_ms,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{RunConfig, TestResult};

    #[test]
    fn text_progress_format() {
        let reporter = StandardReporter::new();
        let progress = RunProgress {
            run_id: "run1".into(),
            total: 10,
            completed: 5,
            passed: 4,
            failed: 1,
            skipped: 0,
            running: 1,
            percent_complete: 50.0,
            elapsed_ms: 1500,
        };
        let text = reporter.format_progress(&progress, ReportFormat::Text);
        assert!(text.contains("50.0%"));
        assert!(text.contains("5/10"));
    }

    #[test]
    fn json_summary_format() {
        let reporter = StandardReporter::new();
        let summary = RunSummary {
            run_id: "run1".into(),
            config: RunConfig::default(),
            results: vec![TestResult {
                test_id: "t1".into(),
                status: TestStatus::Passed,
                duration_ms: 42,
                message: None,
                stdout: None,
                stderr: None,
            }],
            total: 1,
            passed: 1,
            failed: 0,
            skipped: 0,
            errored: 0,
            total_duration_ms: 42,
            started_at: 1000,
            completed_at: 1042,
        };
        let json = reporter.format_summary(&summary, ReportFormat::Json);
        assert!(json.contains("\"run_id\": \"run1\""));
        assert!(json.contains("\"passed\": 1"));
    }
}
