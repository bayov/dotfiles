use std::collections::HashMap;

use super::Stage;
use super::Step;
use super::StepKind;
use super::Workflow;
use crate::result::Error;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Deserialize a TOML string into a [`Workflow`], panicking on parse failure.
fn parse(toml: &str) -> Workflow {
    toml::from_str(toml).expect("TOML should parse without error")
}

// ---------------------------------------------------------------------------
// Parse round-trip: full workflow
// ---------------------------------------------------------------------------

#[test]
fn full_workflow_round_trip() {
    // Given: a TOML string representing the canonical feature workflow
    let toml_input = r#"
        name = "feature"
        version = 1
        description = "Ship a new feature from design through merge."

        [commands]
        format = "cargo fmt --check"
        lint = "cargo clippy -- -D warnings"

        [[stages]]
        name = "design"
        description = "Produce a design doc."
        produces = ["design.md"]
        steps = [
            { name = "write-design", type = "agent", prompt = "design.md" },
        ]

        [[stages]]
        name = "implement"
        description = "Write the code."
        requires = ["design.md"]
        steps = [
            { name = "write-code", type = "agent", prompt = "implement.md" },
            { name = "format", type = "command", command = "format" },
            { name = "lint", type = "command", command = "lint" },
        ]

        [[stages]]
        name = "review"
        repeatable = true
        produces = ["changelog-entry.md"]
        steps = [
            { name = "pre-review-test", type = "command", command = "test" },
            { name = "do-review", type = "agent", prompt = "review.md" },
        ]
    "#;

    // When: we parse the TOML and validate
    let workflow = parse(toml_input);
    workflow.validate().expect("full workflow should be valid");

    // Then: top-level fields match
    assert_eq!(workflow.name, "feature");
    assert_eq!(workflow.version, 1);
    assert_eq!(
        workflow.description.as_deref(),
        Some("Ship a new feature from design through merge.")
    );

    // Then: commands table is populated
    let mut expected_commands = HashMap::new();
    expected_commands
        .insert("format".to_owned(), "cargo fmt --check".to_owned());
    expected_commands
        .insert("lint".to_owned(), "cargo clippy -- -D warnings".to_owned());
    assert_eq!(workflow.commands, expected_commands);

    // Then: stage count
    assert_eq!(workflow.stages.len(), 3);

    // Then: "design" stage
    let design = &workflow.stages[0];
    assert_eq!(design.name, "design");
    assert_eq!(design.description.as_deref(), Some("Produce a design doc."));
    assert!(!design.repeatable);
    assert_eq!(design.produces, vec!["design.md"]);
    assert!(design.requires.is_empty());
    assert_eq!(design.steps.len(), 1);
    assert_eq!(design.steps[0], Step {
        name: "write-design".to_owned(),
        kind: StepKind::Agent {
            prompt: Some("design.md".to_owned()),
            instructions: None,
        },
    });

    // Then: "implement" stage
    let implement = &workflow.stages[1];
    assert_eq!(implement.name, "implement");
    assert!(!implement.repeatable);
    assert!(implement.produces.is_empty());
    assert_eq!(implement.requires, vec!["design.md"]);
    assert_eq!(implement.steps.len(), 3);
    assert_eq!(implement.steps[0], Step {
        name: "write-code".to_owned(),
        kind: StepKind::Agent {
            prompt: Some("implement.md".to_owned()),
            instructions: None,
        },
    });
    assert_eq!(implement.steps[1], Step {
        name: "format".to_owned(),
        kind: StepKind::Command {
            command: Some("format".to_owned()),
            run: None,
        },
    });
    assert_eq!(implement.steps[2], Step {
        name: "lint".to_owned(),
        kind: StepKind::Command { command: Some("lint".to_owned()), run: None },
    });

    // Then: "review" stage
    let review = &workflow.stages[2];
    assert_eq!(review.name, "review");
    assert!(review.repeatable);
    assert_eq!(review.produces, vec!["changelog-entry.md"]);
    assert!(review.requires.is_empty());
    assert_eq!(review.steps.len(), 2);
}

// ---------------------------------------------------------------------------
// Defaults
// ---------------------------------------------------------------------------

#[test]
fn defaults_for_optional_fields() {
    // Given: a minimal workflow with a single stage that omits all optional
    // fields
    let toml_input = r#"
        name = "minimal"
        version = 1

        [[stages]]
        name = "build"
        steps = [
            { name = "compile", type = "command", run = "cargo build" },
        ]
    "#;

    // When: we parse the TOML
    let workflow = parse(toml_input);

    // Then: top-level optional fields are absent / defaulted
    assert!(workflow.description.is_none());
    assert!(workflow.commands.is_empty());

    // Then: stage optional fields are defaulted
    let stage = &workflow.stages[0];
    assert!(stage.description.is_none());
    assert!(!stage.repeatable, "repeatable should default to false");
    assert!(stage.produces.is_empty(), "produces should default to []");
    assert!(stage.requires.is_empty(), "requires should default to []");
}

// ---------------------------------------------------------------------------
// Inline `run` command step
// ---------------------------------------------------------------------------

#[test]
fn inline_run_command_step_parses_correctly() {
    // Given: a step using the inline `run` field instead of a `command` alias
    let toml_input = r#"
        name = "ci"
        version = 1

        [[stages]]
        name = "test"
        steps = [
            { name = "unit-tests", type = "command", run = "cargo test --workspace" },
        ]
    "#;

    // When: we parse and validate
    let workflow = parse(toml_input);
    workflow.validate().expect("inline run step should be valid");

    // Then: the step carries the inline run string
    let step = &workflow.stages[0].steps[0];
    assert_eq!(step.kind, StepKind::Command {
        command: None,
        run: Some("cargo test --workspace".to_owned()),
    });
}

// ---------------------------------------------------------------------------
// Inline `instructions` agent step
// ---------------------------------------------------------------------------

#[test]
fn inline_instructions_agent_step_parses_correctly() {
    // Given: a step using the inline `instructions` field instead of a prompt
    // file path
    let toml_input = r#"
        name = "hotfix"
        version = 1

        [[stages]]
        name = "patch"
        steps = [
            { name = "apply-fix", type = "agent", instructions = "Fix the null-pointer crash in auth.rs." },
        ]
    "#;

    // When: we parse and validate
    let workflow = parse(toml_input);
    workflow.validate().expect("inline instructions step should be valid");

    // Then: the step carries the inline instructions string
    let step = &workflow.stages[0].steps[0];
    assert_eq!(step.kind, StepKind::Agent {
        prompt: None,
        instructions: Some("Fix the null-pointer crash in auth.rs.".to_owned()),
    });
}

// ---------------------------------------------------------------------------
// Serialization round-trip
// ---------------------------------------------------------------------------

#[test]
fn serialize_then_deserialize_is_identity() {
    // Given: a workflow built in memory
    let original = Workflow {
        name: "release".to_owned(),
        version: 2,
        description: Some("Cut a release.".to_owned()),
        commands: {
            let mut m = HashMap::new();
            m.insert("tag".to_owned(), "git tag -a".to_owned());
            m
        },
        stages: vec![Stage {
            name: "cut".to_owned(),
            description: None,
            repeatable: false,
            produces: vec!["CHANGELOG.md".to_owned()],
            requires: vec![],
            steps: vec![
                Step {
                    name: "generate-changelog".to_owned(),
                    kind: StepKind::Agent {
                        prompt: Some("changelog.md".to_owned()),
                        instructions: None,
                    },
                },
                Step {
                    name: "push-tag".to_owned(),
                    kind: StepKind::Command {
                        command: Some("tag".to_owned()),
                        run: None,
                    },
                },
            ],
        }],
    };

    // When: we serialize to TOML and deserialize back
    let serialized =
        toml::to_string(&original).expect("serialization should succeed");
    let roundtripped: Workflow =
        toml::from_str(&serialized).expect("deserialization should succeed");

    // Then: the roundtripped value matches the original
    assert_eq!(original, roundtripped);
}

// ---------------------------------------------------------------------------
// Validation: mutual exclusivity
// ---------------------------------------------------------------------------

#[test]
fn command_step_with_both_command_and_run_fails_validation() {
    // Given: a workflow with a command step that sets both `command` and `run`
    let workflow = Workflow {
        name: "bad".to_owned(),
        version: 1,
        description: None,
        commands: HashMap::new(),
        stages: vec![Stage {
            name: "build".to_owned(),
            description: None,
            repeatable: false,
            produces: vec![],
            requires: vec![],
            steps: vec![Step {
                name: "ambiguous-cmd".to_owned(),
                kind: StepKind::Command {
                    command: Some("lint".to_owned()),
                    run: Some("cargo clippy".to_owned()),
                },
            }],
        }],
    };

    // When: we validate the workflow
    let result = workflow.validate();

    // Then: validation fails with a WorkflowValidation error
    assert!(
        matches!(result, Err(Error::WorkflowValidation { .. })),
        "expected WorkflowValidation error, got: {result:?}"
    );
}

#[test]
fn command_step_with_neither_command_nor_run_fails_validation() {
    // Given: a command step with both fields absent
    let workflow = Workflow {
        name: "bad".to_owned(),
        version: 1,
        description: None,
        commands: HashMap::new(),
        stages: vec![Stage {
            name: "build".to_owned(),
            description: None,
            repeatable: false,
            produces: vec![],
            requires: vec![],
            steps: vec![Step {
                name: "empty-cmd".to_owned(),
                kind: StepKind::Command { command: None, run: None },
            }],
        }],
    };

    // When / Then
    assert!(matches!(
        workflow.validate(),
        Err(Error::WorkflowValidation { .. })
    ));
}

#[test]
fn agent_step_with_both_prompt_and_instructions_fails_validation() {
    // Given: an agent step that sets both `prompt` and `instructions`
    let workflow = Workflow {
        name: "bad".to_owned(),
        version: 1,
        description: None,
        commands: HashMap::new(),
        stages: vec![Stage {
            name: "design".to_owned(),
            description: None,
            repeatable: false,
            produces: vec![],
            requires: vec![],
            steps: vec![Step {
                name: "write".to_owned(),
                kind: StepKind::Agent {
                    prompt: Some("design.md".to_owned()),
                    instructions: Some("Write a design doc.".to_owned()),
                },
            }],
        }],
    };

    // When / Then
    assert!(matches!(
        workflow.validate(),
        Err(Error::WorkflowValidation { .. })
    ));
}

#[test]
fn agent_step_with_neither_prompt_nor_instructions_fails_validation() {
    // Given: an agent step with both fields absent
    let workflow = Workflow {
        name: "bad".to_owned(),
        version: 1,
        description: None,
        commands: HashMap::new(),
        stages: vec![Stage {
            name: "design".to_owned(),
            description: None,
            repeatable: false,
            produces: vec![],
            requires: vec![],
            steps: vec![Step {
                name: "write".to_owned(),
                kind: StepKind::Agent { prompt: None, instructions: None },
            }],
        }],
    };

    // When / Then
    assert!(matches!(
        workflow.validate(),
        Err(Error::WorkflowValidation { .. })
    ));
}

// ---------------------------------------------------------------------------
// Validation: uniqueness
// ---------------------------------------------------------------------------

#[test]
fn duplicate_stage_names_fail_validation() {
    // Given: a workflow with two stages that share the name "build"
    let workflow = Workflow {
        name: "dup-stages".to_owned(),
        version: 1,
        description: None,
        commands: HashMap::new(),
        stages: vec![
            Stage {
                name: "build".to_owned(),
                description: None,
                repeatable: false,
                produces: vec![],
                requires: vec![],
                steps: vec![Step {
                    name: "compile".to_owned(),
                    kind: StepKind::Command {
                        command: None,
                        run: Some("cargo build".to_owned()),
                    },
                }],
            },
            Stage {
                name: "build".to_owned(),
                description: None,
                repeatable: false,
                produces: vec![],
                requires: vec![],
                steps: vec![Step {
                    name: "test".to_owned(),
                    kind: StepKind::Command {
                        command: None,
                        run: Some("cargo test".to_owned()),
                    },
                }],
            },
        ],
    };

    // When / Then
    assert!(matches!(
        workflow.validate(),
        Err(Error::WorkflowValidation { .. })
    ));
}

#[test]
fn duplicate_step_names_within_stage_fail_validation() {
    // Given: a stage with two steps that both have the name "check"
    let workflow = Workflow {
        name: "dup-steps".to_owned(),
        version: 1,
        description: None,
        commands: HashMap::new(),
        stages: vec![Stage {
            name: "verify".to_owned(),
            description: None,
            repeatable: false,
            produces: vec![],
            requires: vec![],
            steps: vec![
                Step {
                    name: "check".to_owned(),
                    kind: StepKind::Command {
                        command: None,
                        run: Some("cargo check".to_owned()),
                    },
                },
                Step {
                    name: "check".to_owned(),
                    kind: StepKind::Command {
                        command: None,
                        run: Some("cargo clippy".to_owned()),
                    },
                },
            ],
        }],
    };

    // When / Then
    assert!(matches!(
        workflow.validate(),
        Err(Error::WorkflowValidation { .. })
    ));
}
