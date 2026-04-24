use std::collections::HashMap;

use super::LinearKind;
use super::Stage;
use super::StageKind;
use super::Workflow;
use crate::result::Error;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse(toml: &str) -> Workflow {
    toml::from_str(toml).expect("TOML should parse without error")
}

// ---------------------------------------------------------------------------
// Parse round-trip: full workflow
// ---------------------------------------------------------------------------

#[test]
fn full_workflow_round_trip() {
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
        args = { prompt_file = "design.md" }

        [[stages]]
        name = "implement"
        description = "Write the code."
        kind = "agent"
        args = { prompt_file = "implement.md" }

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
        args = { prompt_file = "review.md" }
    "#;

    let workflow = parse(toml_input);
    workflow.validate().expect("full workflow should be valid");

    assert_eq!(workflow.name, "feature");
    assert_eq!(workflow.version, 1);
    assert_eq!(
        workflow.description.as_deref(),
        Some("Ship a new feature from design through merge.")
    );

    let mut expected_commands = HashMap::new();
    expected_commands
        .insert("format".to_owned(), "cargo fmt --check".to_owned());
    expected_commands
        .insert("lint".to_owned(), "cargo clippy -- -D warnings".to_owned());
    assert_eq!(workflow.commands, expected_commands);

    assert_eq!(workflow.stages.len(), 5);

    let design = &workflow.stages[0];
    assert_eq!(design.name, "design");
    assert_eq!(design.description.as_deref(), Some("Produce a design doc."));
    assert_eq!(design.produces, vec!["design.md"]);
    assert_eq!(
        design.kind,
        StageKind::Linear(LinearKind::Agent {
            prompt_file: Some("design.md".to_owned()),
            prompt: None,
        })
    );

    let implement = &workflow.stages[1];
    assert_eq!(implement.name, "implement");
    assert!(implement.produces.is_empty());
    assert_eq!(
        implement.kind,
        StageKind::Linear(LinearKind::Agent {
            prompt_file: Some("implement.md".to_owned()),
            prompt: None,
        })
    );

    let format_stage = &workflow.stages[2];
    assert_eq!(format_stage.name, "format");
    assert_eq!(
        format_stage.kind,
        StageKind::Linear(LinearKind::Command {
            command: Some("format".to_owned()),
            run: None,
        })
    );

    let review = &workflow.stages[4];
    assert_eq!(review.name, "review");
    assert_eq!(review.produces, vec!["changelog-entry.md"]);
}

// ---------------------------------------------------------------------------
// Task-runner stage parses correctly
// ---------------------------------------------------------------------------

#[test]
fn task_runner_stage_parses_correctly() {
    let toml_input = r#"
        name = "impl-wf"
        version = 1

        [[stages]]
        name = "task-breakdown"
        produces = ["tasks/foo.toml", "tasks/bar.toml"]
        kind = "agent"
        args = { prompt = "Break work into tasks." }

        [[stages]]
        name = "implement"
        type = "task-runner"
        task_lists = ["tasks/foo.toml", "tasks/bar.toml"]
        parallel = true
    "#;

    let workflow = parse(toml_input);
    workflow.validate().expect("task-runner workflow should be valid");

    assert_eq!(workflow.stages.len(), 2);

    let impl_stage = &workflow.stages[1];
    assert_eq!(impl_stage.name, "implement");
    assert_eq!(impl_stage.kind, StageKind::TaskRunner {
        task_lists: vec![
            "tasks/foo.toml".to_owned(),
            "tasks/bar.toml".to_owned(),
        ],
        parallel: true,
    });
}

#[test]
fn task_runner_stage_parallel_defaults_to_false() {
    let toml_input = r#"
        name = "wf"
        version = 1

        [[stages]]
        name = "implement"
        task_lists = ["tasks/foo.toml"]
    "#;

    let workflow = parse(toml_input);
    assert_eq!(workflow.stages[0].kind, StageKind::TaskRunner {
        task_lists: vec!["tasks/foo.toml".to_owned()],
        parallel: false,
    });
}

// ---------------------------------------------------------------------------
// Defaults
// ---------------------------------------------------------------------------

#[test]
fn defaults_for_optional_fields() {
    let toml_input = r#"
        name = "minimal"
        version = 1

        [[stages]]
        name = "build"
        kind = "command"
        args = { run = "cargo build" }
    "#;

    let workflow = parse(toml_input);

    assert!(workflow.description.is_none());
    assert!(workflow.commands.is_empty());

    let stage = &workflow.stages[0];
    assert!(stage.description.is_none());
    assert!(stage.produces.is_empty(), "produces should default to []");
}

// ---------------------------------------------------------------------------
// Inline `run` command stage
// ---------------------------------------------------------------------------

#[test]
fn inline_run_command_stage_parses_correctly() {
    let toml_input = r#"
        name = "ci"
        version = 1

        [[stages]]
        name = "test"
        kind = "command"
        args = { run = "cargo test --workspace" }
    "#;

    let workflow = parse(toml_input);
    workflow.validate().expect("inline run stage should be valid");

    assert_eq!(
        workflow.stages[0].kind,
        StageKind::Linear(LinearKind::Command {
            command: None,
            run: Some("cargo test --workspace".to_owned()),
        })
    );
}

// ---------------------------------------------------------------------------
// Inline `prompt` agent stage
// ---------------------------------------------------------------------------

#[test]
fn inline_prompt_agent_stage_parses_correctly() {
    let toml_input = r#"
        name = "hotfix"
        version = 1

        [[stages]]
        name = "patch"
        kind = "agent"
        args = { prompt = "Fix the null-pointer crash in auth.rs." }
    "#;

    let workflow = parse(toml_input);
    workflow.validate().expect("inline prompt stage should be valid");

    assert_eq!(
        workflow.stages[0].kind,
        StageKind::Linear(LinearKind::Agent {
            prompt: Some("Fix the null-pointer crash in auth.rs.".to_owned()),
            prompt_file: None,
        })
    );
}

// ---------------------------------------------------------------------------
// Serialization round-trip
// ---------------------------------------------------------------------------

#[test]
fn serialize_then_deserialize_is_identity() {
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
                kind: StageKind::Linear(LinearKind::Agent {
                    prompt_file: Some("changelog.md".to_owned()),
                    prompt: None,
                }),
            },
            Stage {
                name: "push-tag".to_owned(),
                description: None,
                skippable: false,
                produces: vec![],
                kind: StageKind::Linear(LinearKind::Command {
                    command: Some("tag".to_owned()),
                    run: None,
                }),
            },
        ],
        default_flow_dir: None,
    };

    let serialized =
        toml::to_string(&original).expect("serialization should succeed");
    let roundtripped: Workflow =
        toml::from_str(&serialized).expect("deserialization should succeed");

    assert_eq!(original, roundtripped);
}

// ---------------------------------------------------------------------------
// Validation: mutual exclusivity
// ---------------------------------------------------------------------------

#[test]
fn command_stage_with_both_command_and_run_fails_validation() {
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
            kind: StageKind::Linear(LinearKind::Command {
                command: Some("lint".to_owned()),
                run: Some("cargo clippy".to_owned()),
            }),
        }],
        default_flow_dir: None,
    };

    let result = workflow.validate();

    assert!(
        matches!(result, Err(Error::WorkflowValidation { .. })),
        "expected WorkflowValidation error, got: {result:?}"
    );
}

#[test]
fn command_stage_with_neither_command_nor_run_fails_validation() {
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
            kind: StageKind::Linear(LinearKind::Command {
                command: None,
                run: None,
            }),
        }],
        default_flow_dir: None,
    };

    assert!(matches!(
        workflow.validate(),
        Err(Error::WorkflowValidation { .. })
    ));
}

#[test]
fn agent_stage_with_both_prompt_and_prompt_file_fails_validation() {
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
            kind: StageKind::Linear(LinearKind::Agent {
                prompt: Some("Write a design doc.".to_owned()),
                prompt_file: Some("design.md".to_owned()),
            }),
        }],
        default_flow_dir: None,
    };

    assert!(matches!(
        workflow.validate(),
        Err(Error::WorkflowValidation { .. })
    ));
}

#[test]
fn agent_stage_with_neither_prompt_nor_prompt_file_fails_validation() {
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
            kind: StageKind::Linear(LinearKind::Agent {
                prompt: None,
                prompt_file: None,
            }),
        }],
        default_flow_dir: None,
    };

    assert!(matches!(
        workflow.validate(),
        Err(Error::WorkflowValidation { .. })
    ));
}

#[test]
fn task_runner_stage_with_empty_task_lists_fails_validation() {
    let workflow = Workflow {
        name: "bad".to_owned(),
        version: 1,
        description: None,
        commands: HashMap::new(),
        stages: vec![Stage {
            name: "implement".to_owned(),
            description: None,
            skippable: false,
            produces: vec![],
            kind: StageKind::TaskRunner { task_lists: vec![], parallel: false },
        }],
        default_flow_dir: None,
    };

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
                kind: StageKind::Linear(LinearKind::Command {
                    command: None,
                    run: Some("cargo build".to_owned()),
                }),
            },
            Stage {
                name: "build".to_owned(),
                description: None,
                skippable: false,
                produces: vec![],
                kind: StageKind::Linear(LinearKind::Command {
                    command: None,
                    run: Some("cargo test".to_owned()),
                }),
            },
        ],
        default_flow_dir: None,
    };

    assert!(matches!(
        workflow.validate(),
        Err(Error::WorkflowValidation { .. })
    ));
}
