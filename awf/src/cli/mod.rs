//! CLI layer for `awf`: Clap subcommand definitions and dispatch.
//!
//! This module declares the top-level [`Cli`] struct (parsed by Clap), the
//! [`Commands`] enum that routes to either the `workflow` or `flow` subcommand
//! group, and all nested subcommand enums.
//!
//! The global `--output-format` flag (default `human`) is threaded through
//! every handler so that all output can be rendered in the requested format via
//! [`output::render`].

pub mod flow;
pub mod output;
pub mod workflow;

use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use std::process;

use clap::Parser;
use clap::Subcommand;
use owo_colors::OwoColorize as _;

use crate::cli::output::OutputFormat;
use crate::cli::output::eprint_error;
use crate::config::WorkflowSearchEntry;
use crate::model::workflow::StepKind;
use crate::model::workflow::Workflow;
use crate::result::Error;
use crate::store::fs;

// ---------------------------------------------------------------------------
// Top-level CLI
// ---------------------------------------------------------------------------

/// Top-level CLI entry point parsed by Clap.
#[derive(Parser)]
#[command(name = "awf", about = "Agent workflow CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Output format for all command results.
    #[arg(
        long,
        global = true,
        default_value = "human",
        env = "AWF_OUTPUT_FORMAT"
    )]
    pub output_format: OutputFormat,
}

// ---------------------------------------------------------------------------
// Top-level subcommands
// ---------------------------------------------------------------------------

/// Top-level subcommand groups.
#[derive(Subcommand)]
pub enum Commands {
    /// Manage workflow definitions
    Workflow {
        #[command(subcommand)]
        command: WorkflowCommands,
    },
    /// Manage flow instances
    Flow {
        #[command(subcommand)]
        command: FlowCommands,
    },
}

// ---------------------------------------------------------------------------
// Workflow subcommands
// ---------------------------------------------------------------------------

/// Subcommands for `awf workflow`.
#[derive(Subcommand)]
pub enum WorkflowCommands {
    /// Create a new workflow template
    New { name: String },
    /// Validate a workflow definition (omit name to validate all)
    Validate { name: Option<String> },
    /// List available workflow definitions
    List,
}

// ---------------------------------------------------------------------------
// Flow subcommands
// ---------------------------------------------------------------------------

/// Subcommands for `awf flow`.
#[derive(Subcommand)]
pub enum FlowCommands {
    /// Create a new flow instance
    New {
        /// Flow name or path.
        ///
        /// - Bare name (e.g. `my-flow`): uses the workflow's
        ///   `default_flow_dir` to build the path.
        /// - Absolute path (`/…`): used as-is.
        /// - Relative path (`./…`): resolved from the current directory.
        /// - Tilde path (`~/…`): expanded via `$HOME`.
        flow: String,
        #[arg(long)]
        workflow: String,
    },
    /// List flows
    List {
        #[arg(long)]
        all: bool,
    },
    /// Show current flow status
    Status {
        #[arg(long)]
        flow: Option<String>,
    },
    /// Show next required action
    #[command(name = "next-step")]
    NextStep {
        #[arg(long)]
        flow: Option<String>,
    },
    /// Execute a command step
    Run {
        step: String,
        #[arg(long)]
        flow: Option<String>,
    },
    /// Mark an agent step complete
    Step {
        #[command(subcommand)]
        command: StepCommands,
    },
    /// Validate flow artifacts
    Validate {
        #[arg(long)]
        flow: Option<String>,
    },
    /// Advance to the next stage
    #[command(name = "next-stage")]
    NextStage {
        #[arg(long)]
        flow: Option<String>,
        #[arg(long)]
        force: Option<String>,
    },
    /// Show transition and command history
    History {
        #[arg(long)]
        flow: Option<String>,
    },
    /// Archive a completed flow
    Archive {
        #[arg(long)]
        flow: Option<String>,
    },
}

// ---------------------------------------------------------------------------
// Step subcommands
// ---------------------------------------------------------------------------

/// Subcommands for `awf flow step`.
#[derive(Subcommand)]
pub enum StepCommands {
    /// Mark a step as done
    Done {
        step: String,
        #[arg(long)]
        flow: Option<String>,
    },
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

/// Dispatches a parsed [`Cli`] to the appropriate handler.
pub fn dispatch(cli: Cli) -> color_eyre::Result<()> {
    match cli.command {
        Commands::Workflow { command } => {
            dispatch_workflow(command, cli.output_format)
        }
        Commands::Flow { command } => dispatch_flow(command, cli.output_format),
    }
}

/// Dispatches `awf workflow` subcommands.
#[allow(clippy::needless_pass_by_value)]
fn dispatch_workflow(
    command: WorkflowCommands,
    fmt: OutputFormat,
) -> color_eyre::Result<()> {
    match command {
        WorkflowCommands::New { name } => workflow::new(&name, &fmt),
        WorkflowCommands::Validate { name } => {
            workflow::validate(name.as_deref(), &fmt)
        }
        WorkflowCommands::List => workflow::list(&fmt),
    }
}

/// Dispatches `awf flow` subcommands.
#[allow(clippy::needless_pass_by_value)]
fn dispatch_flow(
    command: FlowCommands,
    fmt: OutputFormat,
) -> color_eyre::Result<()> {
    match command {
        FlowCommands::New { flow, workflow } => {
            flow::new(&flow, &workflow, &fmt)
        }
        FlowCommands::List { all } => flow::list(all, &fmt),
        FlowCommands::Status { flow: flow_flag } => {
            flow::status(flow_flag.as_deref(), &fmt)
        }
        FlowCommands::NextStep { flow: flow_flag } => {
            flow::next(flow_flag.as_deref(), &fmt)
        }
        FlowCommands::Run { step, flow: flow_flag } => {
            flow::run(&step, flow_flag.as_deref(), &fmt)
        }
        FlowCommands::Step { command } => dispatch_step(command, fmt),
        FlowCommands::Validate { flow: flow_flag } => {
            flow::validate(flow_flag.as_deref(), &fmt)
        }
        FlowCommands::NextStage { flow: flow_flag, force } => {
            flow::advance(flow_flag.as_deref(), force, &fmt)
        }
        FlowCommands::History { flow: flow_flag } => {
            flow::history(flow_flag.as_deref(), &fmt)
        }
        FlowCommands::Archive { flow: flow_flag } => {
            flow::archive(flow_flag.as_deref(), &fmt)
        }
    }
}

/// Dispatches `awf flow step` subcommands.
#[allow(clippy::needless_pass_by_value)]
fn dispatch_step(
    command: StepCommands,
    fmt: OutputFormat,
) -> color_eyre::Result<()> {
    match command {
        StepCommands::Done { step, flow: flow_flag } => {
            flow::step_done(&step, flow_flag.as_deref(), &fmt)
        }
    }
}

// ---------------------------------------------------------------------------
// Shared CLI helpers
// ---------------------------------------------------------------------------

/// Finds the nearest project root, printing a friendly error and exiting when
/// no `.agents/` directory is found up the directory tree.
pub fn find_project_root_or_exit(start: &Path) -> color_eyre::Result<PathBuf> {
    match crate::store::resolve::find_project_root(start) {
        Ok(p) => Ok(p),
        Err(Error::ProjectRootNotFound { .. }) => {
            eprint_error(
                "not inside an awf project \
                 (no .agents/ directory found in this directory or any parent)",
            );
            anstream::eprintln!(
                "  {} Run this command from inside a directory that contains \
                 a .agents/ folder.",
                "Hint:"
            );
            process::exit(1);
        }
        Err(e) => Err(e.into()),
    }
}

/// Loads a workflow by name, printing rich diagnostic context and exiting when
/// not found or ambiguous.
///
/// On `WorkflowNotFound` the output includes the configured search
/// directories, up to 10 available workflows, and a "Did you mean …?" hint
/// when an existing workflow name is within edit-distance 2 of the query.
pub fn load_workflow_or_exit(
    entries: &[WorkflowSearchEntry],
    name: &str,
) -> color_eyre::Result<Workflow> {
    match fs::load_workflow(entries, name) {
        Ok(wf) => Ok(wf),
        Err(Error::WorkflowNotFound { name: ref n }) => {
            eprint_error(format_args!("workflow {n:?} not found"));

            // Search directories.
            anstream::eprintln!("\n  Searched in:");
            for entry in entries {
                anstream::eprintln!(
                    "    {}  {}",
                    format!("[{}]", entry.scope).cyan(),
                    entry.dir.display(),
                );
            }

            // Available workflows.
            match fs::list_workflows(entries) {
                Ok(listings) if listings.is_empty() => {
                    anstream::eprintln!(
                        "\n  {}",
                        "No workflows found in any search directory."
                            .bright_black()
                    );
                }
                Ok(listings) => {
                    const MAX_SHOWN: usize = 10;
                    anstream::eprintln!("\n  {}:", "Available workflows");
                    for w in listings.iter().take(MAX_SHOWN) {
                        anstream::eprintln!(
                            "    {}:{}",
                            w.scope.bright_black(),
                            w.name.cyan(),
                        );
                    }
                    if listings.len() > MAX_SHOWN {
                        anstream::eprintln!(
                            "    {} and {} more…",
                            "…".bright_black(),
                            listings.len() - MAX_SHOWN,
                        );
                    }

                    // Fuzzy "Did you mean?" suggestion.
                    // Strip an optional scope prefix for matching.
                    let bare = n.split_once(':').map_or(n.as_str(), |(_, b)| b);
                    if let Some(suggestion) = suggest_workflow(bare, &listings)
                    {
                        anstream::eprintln!(
                            "\n  {} {}?",
                            "Did you mean".yellow(),
                            suggestion.yellow(),
                        );
                    }
                }
                Err(_) => {}
            }

            process::exit(1);
        }
        Err(Error::AmbiguousWorkflow { name: ref n, ref scopes }) => {
            eprint_error(format_args!(
                "workflow {n:?} is ambiguous — found in multiple scopes: {scopes}"
            ));
            anstream::eprintln!(
                "  {} Use a scope-qualified name such as {}.",
                "Hint:",
                format!("<scope>:{n}").cyan(),
            );
            process::exit(1);
        }
        Err(e) => Err(e.into()),
    }
}

/// Collects all validation errors for a workflow definition: structural
/// invariants, command-key references, and prompt-file existence.
///
/// `config_commands` contains repo/user-level command aliases that are also
/// valid targets for a step's `command` key (in addition to
/// `workflow.commands`).
pub fn collect_workflow_errors(
    workflow: &Workflow,
    entries: &[WorkflowSearchEntry],
    config_commands: &HashMap<String, String>,
) -> Vec<String> {
    let mut errors: Vec<String> = Vec::new();

    // Structural validation.
    if let Err(e) = workflow.validate() {
        errors.push(e.to_string());
    }

    // Every `command` key must exist in workflow.commands or config commands.
    for stage in &workflow.stages {
        for step in &stage.steps {
            if let StepKind::Command { command: Some(key), .. } = &step.kind
                && !workflow.commands.contains_key(key.as_str())
                && !config_commands.contains_key(key.as_str())
            {
                errors.push(format!(
                    "step {:?} in stage {:?}: command key {key:?} is not \
                     defined in [commands]",
                    step.name, stage.name
                ));
            }
        }
    }

    // Every `prompt` file must exist on disk.
    let wf_dir = fs::find_workflow_dir(entries, &workflow.name);
    if let Some(wf_dir) = wf_dir {
        for stage in &workflow.stages {
            for step in &stage.steps {
                if let StepKind::Agent { prompt: Some(filename), .. } =
                    &step.kind
                    && !wf_dir.join(filename).exists()
                {
                    errors.push(format!(
                        "step {:?} in stage {:?}: prompt file {filename:?} \
                         not found in workflow directory",
                        step.name, stage.name
                    ));
                }
            }
        }
    }

    errors
}

/// Validates a workflow definition and exits with a diagnostic if any check
/// fails. Silent on success.
///
/// This is the same set of checks as `awf workflow validate` but intended for
/// use as a pre-flight guard in flow commands.
pub fn validate_workflow_or_exit(
    workflow: &Workflow,
    entries: &[WorkflowSearchEntry],
    config_commands: &HashMap<String, String>,
) {
    let errors = collect_workflow_errors(workflow, entries, config_commands);
    if !errors.is_empty() {
        eprint_error(format_args!(
            "workflow {:?} failed validation",
            workflow.name
        ));
        for error in &errors {
            anstream::eprintln!("  {} {error}", "·".red());
        }
        process::exit(1);
    }
}

/// Returns a scope-qualified suggestion (`"scope:name"`) for the closest
/// workflow to `query`, or `None` if no workflow is within edit-distance 2.
fn suggest_workflow(
    query: &str,
    listings: &[fs::WorkflowListing],
) -> Option<String> {
    listings
        .iter()
        .map(|w| (w, edit_distance(query, &w.name)))
        .filter(|(_, d)| *d <= 2 && *d > 0)
        .min_by_key(|(_, d)| *d)
        .map(|(w, _)| format!("{}:{}", w.scope, w.name))
}

/// Levenshtein edit distance between two strings.
fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let (m, n) = (a.len(), b.len());
    let mut dp = vec![vec![0usize; n + 1]; m + 1];
    for (i, row) in dp.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, cell) in dp[0].iter_mut().enumerate() {
        *cell = j;
    }
    for i in 1..=m {
        for j in 1..=n {
            dp[i][j] = if a[i - 1] == b[j - 1] {
                dp[i - 1][j - 1]
            } else {
                1 + dp[i - 1][j].min(dp[i][j - 1]).min(dp[i - 1][j - 1])
            };
        }
    }
    dp[m][n]
}
