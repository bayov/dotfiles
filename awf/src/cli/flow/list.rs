//! Handler for `awf flow list`.

use std::env;
use std::fmt;

use owo_colors::OwoColorize as _;

use crate::cli::output::print_output;
use crate::store::fs;

// ---------------------------------------------------------------------------
// Output type
// ---------------------------------------------------------------------------

/// A single flow entry in the list output.
#[derive(Debug)]
pub struct FlowListEntry {
    pub path: String,
    pub workflow: String,
    pub stage: String,
    pub archived: bool,
}

/// Output payload for `awf flow list`.
#[derive(Debug)]
pub struct FlowListOutput {
    pub flows: Vec<FlowListEntry>,
    /// Controls the empty-list hint and whether the ARCHIVED column is shown.
    pub include_archived: bool,
}

impl fmt::Display for FlowListOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.flows.is_empty() {
            if self.include_archived {
                return writeln!(f, "No flows found.");
            }
            return writeln!(
                f,
                "No flows found. Create one with: awf flow new \
                 --flow=<path> --workflow=<workflow>"
            );
        }

        for entry in &self.flows {
            write!(
                f,
                "• {}  (workflow: {}, stage: {}",
                entry.path.cyan(),
                entry.workflow.blue(),
                entry.stage.magenta(),
            )?;
            if self.include_archived && entry.archived {
                write!(f, ", {}", "archived".bright_black())?;
            }
            writeln!(f, ")")?;
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

/// Handles `awf flow list [--all]`.
///
/// Scans `config.flow_search_dirs` for flows. Without `--all`, archived flows
/// are excluded.
///
/// # Errors
///
/// Returns an error for unexpected I/O failures or if no project root can be
/// found.
pub fn list(include_archived: bool) -> color_eyre::Result<()> {
    let project_root =
        crate::cli::find_project_root_or_exit(&env::current_dir()?)?;
    let config = crate::config::load(&project_root)?;

    let all_flows = fs::scan_flows(&config.flow_search_dirs)?;
    let flows: Vec<_> = if include_archived {
        all_flows
    } else {
        all_flows.into_iter().filter(|f| !f.archived).collect()
    };

    let output = FlowListOutput {
        flows: flows
            .into_iter()
            .map(|f| FlowListEntry {
                path: f.path.display().to_string(),
                workflow: f.workflow,
                stage: f.current_stage,
                archived: f.archived,
            })
            .collect(),
        include_archived,
    };
    print_output(&output);
    Ok(())
}
