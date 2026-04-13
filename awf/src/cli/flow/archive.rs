//! Handler for `awf flow archive`.

use std::env;
use std::fmt;

use owo_colors::OwoColorize as _;

use crate::cli::output::eprint_error;
use crate::cli::output::print_output;
use crate::result::Error;
use crate::store::fs;

// ---------------------------------------------------------------------------
// Output type
// ---------------------------------------------------------------------------

/// Output payload for `awf flow archive`.
#[derive(Debug)]
pub struct FlowArchiveOutput {
    pub archived_flow: String,
}

impl fmt::Display for FlowArchiveOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Archived flow: {}", self.archived_flow.cyan())
    }
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

/// Handles `awf flow archive [--flow=<path>]`.
///
/// # Errors
///
/// Returns an error for unexpected I/O failures.
pub fn archive(flow_flag: Option<&str>) -> color_eyre::Result<()> {
    let project_root =
        crate::cli::find_project_root_or_exit(&env::current_dir()?)?;
    let config = crate::config::load(&project_root)?;
    let flow_dir = super::resolve_or_exit(flow_flag, &config)?;

    match fs::archive_flow(&flow_dir) {
        Ok(()) => {
            let output = FlowArchiveOutput {
                archived_flow: flow_dir.display().to_string(),
            };
            print_output(&output);
            Ok(())
        }
        Err(Error::FlowNotFound { .. }) => {
            eprint_error(format_args!(
                "flow not found at {}",
                flow_dir.display()
            ));
            std::process::exit(1);
        }
        Err(e) => Err(e.into()),
    }
}
