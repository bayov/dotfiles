//! Output formatting for the `awf` CLI.
//!
//! This module defines the [`OutputFormat`] enum (used as a global Clap flag),
//! the [`render`] function that serializes any value implementing both
//! [`serde::Serialize`] and [`std::fmt::Display`] into the requested format,
//! and helpers for printing colored output via [`anstream`] + [`owo_colors`].
//!
//! ## Color support
//!
//! Colors are enabled automatically when stdout/stderr is a TTY.  Use the
//! following environment variables to override:
//!
//! - `NO_COLOR=1`    — strip all ANSI codes (honored by `anstream`)
//! - `FORCE_COLOR=1` — force ANSI codes even when not a TTY
//!
//! [`anstream`] handles detection and stripping; [`owo_colors`] generates the
//! ANSI codes inside [`Display`] implementations.

use std::fmt;
use std::fmt::Display;

use clap::ValueEnum;
use owo_colors::OwoColorize as _;
use serde::Serialize;

/// The output format to use when printing results.
///
/// Passed to the CLI via `--output-format <FORMAT>` (default: `human`).
#[derive(Clone, Debug, Default, ValueEnum)]
pub enum OutputFormat {
    /// Human-readable text via the value's [`Display`] implementation.
    #[default]
    Human,
    /// Pretty-printed JSON via `serde_json`.
    Json,
    /// TOML via the `toml` crate.
    Toml,
    /// Minimal XML: the JSON representation wrapped in a `<data>` element.
    Xml,
}

// ---------------------------------------------------------------------------
// Output helpers
// ---------------------------------------------------------------------------

/// Renders `val` and writes it to stdout via [`anstream`].
///
/// For the `human` format the rendered string may contain ANSI escape codes
/// (added by [`owo_colors`] inside `Display` impls); [`anstream`] strips them
/// automatically when stdout does not support color.
pub fn print_output<T: Serialize + Display>(val: &T, fmt: &OutputFormat) {
    anstream::print!("{}", render(val, fmt));
}

/// Prints `"error: {msg}"` to stderr with a bold-red `"error:"` prefix.
pub fn eprint_error(msg: impl fmt::Display) {
    anstream::eprintln!("{} {msg}", "error:".red().bold());
}

/// Prints `"warning: {msg}"` to stderr with a bold-yellow `"warning:"` prefix.
pub fn eprint_warning(msg: impl fmt::Display) {
    anstream::eprintln!("{} {msg}", "warning:".yellow().bold());
}

// ---------------------------------------------------------------------------
// render
// ---------------------------------------------------------------------------

/// Serializes `val` into the requested output format and returns it as a
/// [`String`].
///
/// - **Human** — delegates to `val`'s [`Display`] implementation.
/// - **Json** — pretty-printed JSON produced by `serde_json`.
/// - **Toml** — TOML produced by the `toml` crate.
/// - **Xml** — minimal XML: the JSON representation of the value is wrapped in
///   a `<data>` element.  This is an intentionally lightweight v1 approach; a
///   proper XML serialiser can be introduced later.
///
/// # Panics
///
/// Panics if serialisation fails (e.g. the type contains a non-serialisable
/// map key). In practice all types used with this function should be trivially
/// serialisable.
pub fn render<T: Serialize + Display>(val: &T, fmt: &OutputFormat) -> String {
    match fmt {
        OutputFormat::Human => format!("{val}"),
        OutputFormat::Json => serde_json::to_string_pretty(val)
            .expect("JSON serialisation failed"),
        OutputFormat::Toml => {
            toml::to_string(val).expect("TOML serialisation failed")
        }
        OutputFormat::Xml => {
            // Minimal XML: embed the JSON payload inside a <data> element.
            // A dedicated XML serialiser can replace this in a future
            // iteration.
            let json = serde_json::to_string_pretty(val)
                .expect("JSON serialisation failed for XML");
            format!("<data>\n{json}\n</data>")
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[path = "output.tests.rs"]
mod tests;
