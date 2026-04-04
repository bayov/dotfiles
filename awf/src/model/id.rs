//! Identifier validation for workflow and flow IDs.

use crate::result::Error;
use crate::result::Result;

/// Validates that `s` is a legal `awf` identifier.
///
/// **Allowed characters:** `[a-zA-Z0-9_/\-]+`
/// The string must be non-empty and contain only ASCII letters, digits,
/// underscores (`_`), forward-slashes (`/`), and hyphens (`-`).
/// Forward-slashes are used as a namespace separator (e.g. `"infra/deploy"`).
///
/// # Errors
///
/// Returns [`Error::InvalidId`] when `s` is empty or contains a character
/// outside the allowed set.
pub fn validate_id(s: &str) -> Result<()> {
    if s.is_empty() {
        return Err(Error::InvalidId {
            id: s.to_owned(),
            reason: "must not be empty".to_owned(),
        });
    }

    if let Some(bad) = s.chars().find(
        |c| !matches!(c, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '/' | '-'),
    ) {
        return Err(Error::InvalidId {
            id: s.to_owned(),
            reason: format!("contains invalid character {bad:?}"),
        });
    }

    Ok(())
}

#[cfg(test)]
#[path = "id.tests.rs"]
mod tests;
