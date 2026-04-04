//! CLI layer for `awf`: Clap subcommand definitions and dispatch.
//!
//! This module declares the top-level [`Cli`] struct (parsed by Clap), the
//! [`Commands`] enum that routes to either the `workflow` or `flow` subcommand
//! group, and all nested subcommand enums.
//!
//! Handlers are currently stubbed out and will be implemented in C3–C15.
//! The global `--output-format` flag (default `human`) is threaded through
//! every handler so that all output can be rendered in the requested format via
//! [`output::render`].

pub mod flow;
pub mod output;
pub mod workflow;

use std::path::PathBuf;

use clap::Parser;
use clap::Subcommand;

use crate::cli::output::OutputFormat;

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
    #[arg(long, global = true, default_value = "human")]
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
    /// Validate a workflow definition
    Validate { name: String },
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
        name: String,
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
    Next {
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
    /// Register an artifact
    Artifact {
        #[command(subcommand)]
        command: ArtifactCommands,
    },
    /// Advance to the next stage
    Advance {
        #[arg(long)]
        flow: Option<String>,
        #[arg(long)]
        force: bool,
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
// Artifact subcommands
// ---------------------------------------------------------------------------

/// Subcommands for `awf flow artifact`.
#[derive(Subcommand)]
pub enum ArtifactCommands {
    /// Register a file as a named artifact
    Put {
        name: String,
        file: PathBuf,
        #[arg(long)]
        flow: Option<String>,
    },
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

/// Dispatches a parsed [`Cli`] to the appropriate handler.
///
/// All handler bodies are currently stubs returning `Ok(())`; they will be
/// filled in during C3–C15.
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
    _fmt: OutputFormat,
) -> color_eyre::Result<()> {
    match command {
        WorkflowCommands::New { name } => workflow::new(&name),
        WorkflowCommands::Validate { name } => workflow::validate(&name),
        WorkflowCommands::List => workflow::list(),
    }
}

/// Dispatches `awf flow` subcommands.
///
/// C6–C8 are fully implemented; the remaining variants are stubs that will be
/// filled in during C9–C15.
#[allow(
    clippy::unnecessary_wraps,
    clippy::match_same_arms,
    clippy::needless_pass_by_value
)]
fn dispatch_flow(
    command: FlowCommands,
    fmt: OutputFormat,
) -> color_eyre::Result<()> {
    match command {
        FlowCommands::New { name, workflow } => flow::new(&name, &workflow),
        FlowCommands::List { all } => flow::list(all),
        FlowCommands::Status { flow: flow_flag } => {
            flow::status(flow_flag.as_deref(), &fmt)
        }
        FlowCommands::Next { flow: _ } => Ok(()),
        FlowCommands::Run { step: _, flow: _ } => Ok(()),
        FlowCommands::Step { command } => dispatch_step(command),
        FlowCommands::Artifact { command } => dispatch_artifact(command),
        FlowCommands::Advance { flow: _, force: _ } => Ok(()),
        FlowCommands::History { flow: _ } => Ok(()),
        FlowCommands::Archive { flow: _ } => Ok(()),
    }
}

/// Dispatches `awf flow step` subcommands.
///
/// Stub — will be implemented in C3–C15.
#[allow(clippy::unnecessary_wraps, clippy::needless_pass_by_value)]
fn dispatch_step(command: StepCommands) -> color_eyre::Result<()> {
    match command {
        StepCommands::Done { step: _, flow: _ } => Ok(()),
    }
}

/// Dispatches `awf flow artifact` subcommands.
///
/// Stub — will be implemented in C3–C15.
#[allow(clippy::unnecessary_wraps, clippy::needless_pass_by_value)]
fn dispatch_artifact(command: ArtifactCommands) -> color_eyre::Result<()> {
    match command {
        ArtifactCommands::Put { name: _, file: _, flow: _ } => Ok(()),
    }
}
