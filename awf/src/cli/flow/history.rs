//! Handler for `awf flow history`.

use std::env;
use std::fmt;

use chrono::Local;
use owo_colors::OwoColorize as _;

use crate::cli::output::print_output;

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

/// A single entry in the history output, mirroring [`StageHistoryEntry`] with
/// timestamps serialized as ISO 8601 strings.
#[derive(Debug)]
pub struct FlowHistoryEntry {
    pub stage: String,
    pub time: String,
    pub status: String,
    pub force_reason: Option<String>,
}

/// Output payload for `awf flow history`.
#[derive(Debug)]
pub struct FlowHistoryOutput {
    pub flow_path: String,
    pub flow_started_at: String,
    pub entries: Vec<FlowHistoryEntry>,
}

impl fmt::Display for FlowHistoryOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "{} flow started at {}",
            self.flow_path.cyan(),
            pretty_dt(&self.flow_started_at).bright_black(),
        )?;
        for entry in &self.entries {
            match entry.status.as_str() {
                "completed" => writeln!(
                    f,
                    "    {} {} completed at {}",
                    "✓".green(),
                    entry.stage.green(),
                    pretty_dt(&entry.time).bright_black(),
                )?,
                "failed" => writeln!(
                    f,
                    "    {} {} failed at {}",
                    "✗".red(),
                    entry.stage.red(),
                    pretty_dt(&entry.time).bright_black(),
                )?,
                "force_completed" => {
                    writeln!(
                        f,
                        "    {} {} force completed at {}",
                        "⚠".yellow(),
                        entry.stage.yellow(),
                        pretty_dt(&entry.time).bright_black(),
                    )?;
                    if let Some(ref reason) = entry.force_reason {
                        writeln!(f, "        {}", reason.bright_black())?;
                    }
                }
                "skipped" => writeln!(
                    f,
                    "    {} {} skipped at {}",
                    "⏭".yellow(),
                    entry.stage.yellow(),
                    pretty_dt(&entry.time).bright_black(),
                )?,
                _ => {}
            }
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

/// Handles `awf flow history [--flow=<path>]`.
///
/// # Errors
///
/// Returns an error for unexpected I/O failures.
pub fn history(flow_flag: Option<&str>) -> color_eyre::Result<()> {
    let project_root =
        crate::cli::find_project_root_or_exit(&env::current_dir()?)?;
    let config = crate::config::load(&project_root)?;
    let flow_dir = super::resolve_or_exit(flow_flag, &config)?;

    let flow = super::load_flow_or_exit(&flow_dir)?;

    let entries: Vec<FlowHistoryEntry> = flow
        .stage_history
        .iter()
        .map(|entry| {
            let status = match entry.status {
                crate::model::flow::StageStatus::Completed => "completed",
                crate::model::flow::StageStatus::Failed => "failed",
                crate::model::flow::StageStatus::ForceCompleted => {
                    "force_completed"
                }
                crate::model::flow::StageStatus::Skipped => "skipped",
            };
            FlowHistoryEntry {
                stage: entry.stage.clone(),
                time: entry.time.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
                status: status.to_owned(),
                force_reason: entry.force_reason.clone(),
            }
        })
        .collect();

    let output = FlowHistoryOutput {
        flow_path: flow_dir.display().to_string(),
        flow_started_at: flow
            .created_at
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string(),
        entries,
    };
    print_output(&output);
    Ok(())
}

// ---------------------------------------------------------------------------
// Private helper
// ---------------------------------------------------------------------------

/// Parses an ISO 8601 timestamp and formats it in the local timezone as
/// `"YYYY-MM-DD HH:MM:SS ±HH:MM"`. Returns the input unchanged if it cannot
/// be parsed (e.g. `"123ms"` duration strings).
fn pretty_dt(s: &str) -> String {
    chrono::DateTime::parse_from_rfc3339(s).map_or_else(
        |_| s.to_owned(),
        |dt| {
            dt.with_timezone(&Local).format("%Y-%m-%d %H:%M:%S %:z").to_string()
        },
    )
}
