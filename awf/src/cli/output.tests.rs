use std::fmt;

use serde::Serialize;

use super::OutputFormat;
use super::render;

/// A minimal concrete type that implements both [`Serialize`] and [`Display`],
/// used to exercise every output format in isolation.
#[derive(Serialize)]
struct Report {
    name: String,
    count: u32,
}

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.name, self.count)
    }
}

fn sample() -> Report { Report { name: "widgets".to_string(), count: 42 } }

#[test]
fn human_format_uses_display() {
    // Given: a sample report
    let report = sample();

    // When: rendered as Human
    let output = render(&report, &OutputFormat::Human);

    // Then: the output matches the Display representation
    assert_eq!(output, "widgets (42)");
}

#[test]
fn json_format_is_pretty_printed() {
    // Given: a sample report
    let report = sample();

    // When: rendered as JSON
    let output = render(&report, &OutputFormat::Json);

    // Then: the output is valid JSON containing the expected fields
    let parsed: serde_json::Value =
        serde_json::from_str(&output).expect("output should be valid JSON");
    assert_eq!(parsed["name"], "widgets");
    assert_eq!(parsed["count"], 42);
    // Pretty-printed JSON ends with a newline
    assert!(output.ends_with('\n'));
}

#[test]
fn toml_format_contains_key_value_pairs() {
    // Given: a sample report
    let report = sample();

    // When: rendered as TOML
    let output = render(&report, &OutputFormat::Toml);

    // Then: the output is valid TOML with the expected entries
    assert!(output.contains("name = \"widgets\""), "output: {output}");
    assert!(output.contains("count = 42"), "output: {output}");
}

#[test]
fn xml_format_wraps_content_in_data_element() {
    // Given: a sample report
    let report = sample();

    // When: rendered as XML
    let output = render(&report, &OutputFormat::Xml);

    // Then: output is wrapped in <data>…</data> and contains the JSON payload
    assert!(output.starts_with("<data>"), "output: {output}");
    assert!(output.ends_with("</data>\n"), "output: {output}");
    assert!(output.contains("widgets"), "output: {output}");
    assert!(output.contains("42"), "output: {output}");
}
