use std::collections::HashMap;

use super::Stage;
use super::StageKind;
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
        kind = "agent"
        args = { prompt = "design.md" }

        [[stages]]
        name = "implement"
        description = "Write the code."
        kind = "agent"
        args = { prompt = "implement.md" }

        [[stages]]
        name = "format"
        kind = "command"
        args = { command = "format" }

        [[stages]]
        name = "lint"
        kind = "command"
        args = { command = "lint" }

        [[stages]]
        name = "review"
        produces = ["changelog-entry.md"]
        kind = "agent"
        args = { prompt = "review.md" }
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
    assert_eq!(workflow.stages.len(), 5);

    // Then: "design" stage
    let design = &workflow.stages[0];
    assert_eq!(design.name, "design");
    assert_eq!(design.description.as_deref(), Some("Produce a design doc."));
    assert_eq!(design.produces, vec!["design.md"]);
    assert_eq!(design.kind, StageKind::Agent {
        prompt: Some("design.md".to_owned()),
        instructions: None,
    });

    // Then: "implement" stage
    let implement = &workflow.stages[1];
    assert_eq!(implement.name, "implement");
    assert!(implement.produces.is_empty());
    assert_eq!(implement.kind, StageKind::Agent {
        prompt: Some("implement.md".to_owned()),
        instructions: None,
    });

    // Then: "format" stage
    let format_stage = &workflow.stages[2];
    assert_eq!(format_stage.name, "format");
    assert_eq!(format_stage.kind, StageKind::Command {
        command: Some("format".to_owned()),
        run: None,
    });

    // Then: "review" stage
    let review = &workflow.stages[4];
    assert_eq!(review.name, "review");
    assert_eq!(review.produces, vec!["changelog-entry.md"]);
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
        kind = "command"
        args = { run = "cargo build" }
    "#;

    // When: we parse the TOML
    let workflow = parse(toml_input);

    // Then: top-level optional fields are absent / defaulted
    assert!(workflow.description.is_none());
    assert!(workflow.commands.is_empty());

    // Then: stage optional fields are defaulted
    let stage = &workflow.stages[0];
    assert!(stage.description.is_none());
    assert!(stage.produces.is_empty(), "produces should default to []");
}

// ---------------------------------------------------------------------------
// Inline `run` command stage
// ---------------------------------------------------------------------------

#[test]
fn inline_run_command_stage_parses_correctly() {
    // Given: a stage using the inline `run` field instead of a `command` alias
    let toml_input = r#"
        name = "ci"
        version = 1

        [[stages]]
        name = "test"
        kind = "command"
        args = { run = "cargo test --workspace" }
    "#;

    // When: we parse and validate
    let workflow = parse(toml_input);
    workflow.validate().expect("inline run stage should be valid");

    // Then: the stage carries the inline run string
    assert_eq!(workflow.stages[0].kind, StageKind::Command {
        command: None,
        run: Some("cargo test --workspace".to_owned()),
    });
}

// ---------------------------------------------------------------------------
// Inline `instructions` agent stage
// ---------------------------------------------------------------------------

#[test]
fn inline_instructions_agent_stage_parses_correctly() {
    // Given: a stage using the inline `instructions` field instead of a prompt
    // file path
    let toml_input = r#"
        name = "hotfix"
        version = 1

        [[stages]]
        name = "patch"
        kind = "agent"
        args = { instructions = "Fix the null-pointer crash in auth.rs." }
    "#;

    // When: we parse and validate
    let workflow = parse(toml_input);
    workflow.validate().expect("inline instructions stage should be valid");

    // Then: the stage carries the inline instructions string
    assert_eq!(workflow.stages[0].kind, StageKind::Agent {
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
        stages: vec![
            Stage {
                name: "generate-changelog".to_owned(),
                description: None,
                skippable: false,
                produces: vec!["CHANGELOG.md".to_owned()],
                kind: StageKind::Agent {
                    prompt: Some("changelog.md".to_owned()),
                    instructions: None,
                },
            },
            Stage {
                name: "push-tag".to_owned(),
                description: None,
                skippable: false,
                produces: vec![],
                kind: StageKind::Command {
                    command: Some("tag".to_owned()),
                    run: None,
                },
            },
        ],
        default_flow_dir: None,
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
fn command_stage_with_both_command_and_run_fails_validation() {
    // Given: a workflow with a command stage that sets both `command` and `run`
    let workflow = Workflow {
        name: "bad".to_owned(),
        version: 1,
        description: None,
        commands: HashMap::new(),
        stages: vec![Stage {
            name: "build".to_owned(),
            description: None,
            skippable: false,
            produces: vec![],
            kind: StageKind::Command {
                command: Some("lint".to_owned()),
                run: Some("cargo clippy".to_owned()),
            },
        }],
        default_flow_dir: None,
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
fn command_stage_with_neither_command_nor_run_fails_validation() {
    // Given: a command stage with both fields absent
    let workflow = Workflow {
        name: "bad".to_owned(),
        version: 1,
        description: None,
        commands: HashMap::new(),
        stages: vec![Stage {
            name: "build".to_owned(),
            description: None,
            skippable: false,
            produces: vec![],
            kind: StageKind::Command { command: None, run: None },
        }],
        default_flow_dir: None,
    };

    // When / Then
    assert!(matches!(
        workflow.validate(),
        Err(Error::WorkflowValidation { .. })
    ));
}

#[test]
fn agent_stage_with_both_prompt_and_instructions_fails_validation() {
    // Given: an agent stage that sets both `prompt` and `instructions`
    let workflow = Workflow {
        name: "bad".to_owned(),
        version: 1,
        description: None,
        commands: HashMap::new(),
        stages: vec![Stage {
            name: "design".to_owned(),
            description: None,
            skippable: false,
            produces: vec![],
            kind: StageKind::Agent {
                prompt: Some("design.md".to_owned()),
                instructions: Some("Write a design doc.".to_owned()),
            },
        }],
        default_flow_dir: None,
    };

    // When / Then
    assert!(matches!(
        workflow.validate(),
        Err(Error::WorkflowValidation { .. })
    ));
}

#[test]
fn agent_stage_with_neither_prompt_nor_instructions_fails_validation() {
    // Given: an agent stage with both fields absent
    let workflow = Workflow {
        name: "bad".to_owned(),
        version: 1,
        description: None,
        commands: HashMap::new(),
        stages: vec![Stage {
            name: "design".to_owned(),
            description: None,
            skippable: false,
            produces: vec![],
            kind: StageKind::Agent { prompt: None, instructions: None },
        }],
        default_flow_dir: None,
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
                skippable: false,
                produces: vec![],
                kind: StageKind::Command {
                    command: None,
                    run: Some("cargo build".to_owned()),
                },
            },
            Stage {
                name: "build".to_owned(),
                description: None,
                skippable: false,
                produces: vec![],
                kind: StageKind::Command {
                    command: None,
                    run: Some("cargo test".to_owned()),
                },
            },
        ],
        default_flow_dir: None,
    };

    // When / Then
    assert!(matches!(
        workflow.validate(),
        Err(Error::WorkflowValidation { .. })
    ));
}
