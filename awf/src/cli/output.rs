//! Output formatting for the `awf` CLI.
//!
//! This module defines the [`OutputFormat`] enum (used as a global Clap flag)
//! and the [`render`] function that serializes any value implementing both
//! [`serde::Serialize`] and [`std::fmt::Display`] into the requested format.

use std::fmt::Display;

use clap::ValueEnum;
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
