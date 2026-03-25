//! Reporter — formats and delivers results to callers.
//!
//! The reporter takes a RunSummary and produces output suitable for
//! the requesting interface (JSON for MCP/AI, human-readable for console).

use crate::types::RunProgress;
use crate::types::RunSummary;

/// Output format for reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportFormat {
    /// Structured JSON suitable for machine consumption (MCP/AI).
    Json,
    /// Human-readable text for console output.
    Text,
}

/// Formats test results for delivery to callers.
pub trait TestReporter {
    /// Format a complete run summary into a string.
    fn format_summary(&self, summary: &RunSummary, format: ReportFormat) -> String;

    /// Format a progress snapshot into a string.
    fn format_progress(&self, progress: &RunProgress, format: ReportFormat) -> String;
}
