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
    let bar_width: usize = 30;
    // Clamp, and do the arithmetic in u64: completed > total should be
    // impossible, but a rendering function must never underflow, overflow
    // (32-bit usize on wasm32), or OOM on inconsistent counts.
    let filled = if p.total > 0 {
        (((p.completed as u64 * bar_width as u64) / p.total as u64) as usize).min(bar_width)
    } else {
        0
    };
    let empty = bar_width - filled;

    let bar: String = core::iter::repeat('#')
        .take(filled)
        .chain(core::iter::repeat('-').take(empty))
        .collect();

    // The finished marker is what stops a human from polling forever: a
    // finished legacy run can truthfully sit below 100% (fewer results
    // than selected tests), and percent alone would read as "still
    // running".
    let state = if p.finished { "finished" } else { "running" };
    format!(
        "[{}] {:.1}% ({}/{}) | P:{} F:{} E:{} S:{} | {}ms elapsed | {}",
        bar,
        p.percent_complete,
        p.completed,
        p.total,
        p.passed,
        p.failed,
        p.errored,
        p.skipped,
        p.elapsed_ms,
        state,
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
            errored: 0,
            skipped: 0,
            running: 1,
            percent_complete: 50.0,
            elapsed_ms: 1500,
            finished: false,
        };
        let text = reporter.format_progress(&progress, ReportFormat::Text);
        // Invariant: the text renderer is the ONLY place bar fill and the
        // running/finished marker are computed, so the whole line is
        // pinned exactly: 5/10 fills 15 of 30 bar cells proportionally
        // ('#' then '-'), the P/F/E/S counts are labeled, and an
        // unfinished run ends with "| running" — never "finished", the
        // poll-until signal.
        assert_eq!(
            text,
            "[###############---------------] 50.0% (5/10) | P:4 F:1 E:0 S:0 | 1500ms elapsed | running"
        );
    }

    #[test]
    fn progress_count_labels_stay_bound_to_their_fields() {
        // Invariant: each of the four count labels renders ITS field —
        // with all four counts distinct, any swap of format arguments
        // (e.g. errored/skipped, indistinguishable when both are 0)
        // mislabels at least one and fails here.
        let reporter = StandardReporter::new();
        let progress = RunProgress {
            run_id: "run1".into(),
            total: 20,
            completed: 10,
            passed: 1,
            failed: 2,
            errored: 3,
            skipped: 4,
            running: 0,
            percent_complete: 50.0,
            elapsed_ms: 100,
            finished: false,
        };
        let text = reporter.format_progress(&progress, ReportFormat::Text);
        assert!(text.contains("P:1 F:2 E:3 S:4"), "got: {}", text);
    }

    #[test]
    fn zero_total_progress_renders_empty_bar_without_panicking() {
        // Invariant: a zero-total run renders a fully-empty bar and
        // "0/0" — the total > 0 guard is what stands between rendering
        // and a divide-by-zero panic, and no other layer protects it.
        let reporter = StandardReporter::new();
        let progress = RunProgress {
            run_id: "run1".into(),
            total: 0,
            completed: 0,
            passed: 0,
            failed: 0,
            errored: 0,
            skipped: 0,
            running: 0,
            percent_complete: 0.0,
            elapsed_ms: 0,
            finished: false,
        };
        let text = reporter.format_progress(&progress, ReportFormat::Text);
        assert_eq!(
            text,
            "[------------------------------] 0.0% (0/0) | P:0 F:0 E:0 S:0 | 0ms elapsed | running"
        );
    }

    #[test]
    fn overshooting_completed_clamps_bar_at_full_without_panicking() {
        // Invariant: inconsistent counts (completed > total) clamp the
        // bar at exactly full — without the .min(bar_width) clamp,
        // `bar_width - filled` underflows (debug panic, or a release
        // wrap that OOMs building the '-' run).
        let reporter = StandardReporter::new();
        let progress = RunProgress {
            run_id: "run1".into(),
            total: 10,
            completed: 15,
            passed: 15,
            failed: 0,
            errored: 0,
            skipped: 0,
            running: 0,
            percent_complete: 150.0,
            elapsed_ms: 5,
            finished: false,
        };
        let text = reporter.format_progress(&progress, ReportFormat::Text);
        // 30 '#' cells, zero '-' cells.
        assert!(
            text.starts_with("[##############################] "),
            "got: {}",
            text
        );
    }

    #[test]
    fn summary_text_lists_failures_first_with_messages() {
        // Invariant: the failures block is the only place a failure's
        // diagnostic message reaches a human, and it comes BEFORE the
        // full listing: it holds exactly the Failed|Error results in
        // result order, labels Error as "[ERROR]", and indents each
        // failure's message beneath its line — passed tests appear only
        // in "--- All Results ---".
        let reporter = StandardReporter::new();
        let mk = |id: &str, status: TestStatus, ms: u64, msg: Option<&str>| TestResult {
            test_id: id.into(),
            status,
            duration_ms: ms,
            message: msg.map(|m| m.into()),
            stdout: None,
            stderr: None,
        };
        let summary = RunSummary {
            run_id: "run1".into(),
            config: RunConfig::default(),
            results: vec![
                mk("t1", TestStatus::Passed, 5, None),
                mk("t2", TestStatus::Failed, 7, Some("assertion mismatch: expected 5")),
                mk("t3", TestStatus::Error, 3, None),
            ],
            total: 3,
            passed: 1,
            failed: 1,
            skipped: 0,
            errored: 1,
            total_duration_ms: 15,
            started_at: 1000,
            completed_at: 1015,
        };
        let text = reporter.format_summary(&summary, ReportFormat::Text);
        // The whole failures block, pinned contiguously: t1 (passed) is
        // absent, t2 carries its indented message, t3 is labeled ERROR.
        assert!(
            text.contains(
                "--- Failures ---\n  [FAIL] t2 (7ms)\n    assertion mismatch: expected 5\n  [ERROR] t3 (3ms)\n\n"
            ),
            "got: {}",
            text
        );
        let failures_at = text.find("--- Failures ---").unwrap();
        let all_at = text.find("--- All Results ---").unwrap();
        assert!(failures_at < all_at);
        // Every result, the passed one included, still shows in the
        // full listing.
        assert!(text.contains("  [PASS] t1 (5ms)\n"));
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
