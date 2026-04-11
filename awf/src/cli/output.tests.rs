use std::fmt;

use super::render;

/// A minimal concrete type that implements [`Display`], used to exercise the
/// render function in isolation.
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
fn render_uses_display() {
    // Given: a sample report
    let report = sample();

    // When: rendered
    let output = render(&report);

    // Then: the output matches the Display representation
    assert_eq!(output, "widgets (42)");
}
