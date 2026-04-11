//! Output helpers for the `awf` CLI.
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

use owo_colors::OwoColorize as _;

// ---------------------------------------------------------------------------
// Output helpers
// ---------------------------------------------------------------------------

/// Renders `val` via its [`Display`] implementation and writes it to stdout
/// via [`anstream`].
///
/// The rendered string may contain ANSI escape codes (added by [`owo_colors`]
/// inside `Display` impls); [`anstream`] strips them automatically when stdout
/// does not support color.
pub fn print_output<T: Display>(val: &T) {
    anstream::print!("{}", render(val));
}

/// Prints `"error: {msg}"` to stderr with a red `"error:"` prefix.
pub fn eprint_error(msg: impl fmt::Display) {
    anstream::eprintln!("{} {msg}", "error:".red());
}

/// Prints `"warning: {msg}"` to stderr with a yellow `"warning:"` prefix.
pub fn eprint_warning(msg: impl fmt::Display) {
    anstream::eprintln!("{} {msg}", "warning:".yellow());
}

// ---------------------------------------------------------------------------
// render
// ---------------------------------------------------------------------------

/// Renders `val` via its [`Display`] implementation and returns the result as
/// a [`String`].
pub fn render<T: Display>(val: &T) -> String { format!("{val}") }

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[path = "output.tests.rs"]
mod tests;
