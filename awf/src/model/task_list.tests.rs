use super::*;

fn parse(toml: &str) -> TaskList {
    toml::from_str(toml).expect("TOML should parse without error")
}

#[test]
fn full_task_list_round_trip() {
    let input = r#"
        name = "foo"
        description = "Implement the authentication module"
        depends_on = ["bar"]

        [[tasks]]
        id = "1.1"
        description = "Scaffold the auth module"
        type = "agent"

        [[tasks]]
        id = "1.2"
        description = "Write unit tests"
        type = "agent"

        [[tasks]]
        id = "1.3"
        description = "Run formatter"
        type = "command"
        command = "format"
    "#;

    let list = parse(input);
    assert_eq!(list.name, "foo");
    assert_eq!(list.description, "Implement the authentication module");
    assert_eq!(list.depends_on, vec!["bar"]);
    assert_eq!(list.tasks.len(), 3);

    assert_eq!(list.tasks[0].id, "1.1");
    assert_eq!(list.tasks[0].kind, TaskKind::Agent);

    assert_eq!(list.tasks[2].id, "1.3");
    assert_eq!(list.tasks[2].kind, TaskKind::Command {
        command: "format".to_owned()
    });
}

#[test]
fn empty_depends_on_defaults() {
    let input = r#"
        name = "baz"
        description = "No deps"

        [[tasks]]
        id = "1"
        description = "Do something"
        type = "agent"
    "#;

    let list = parse(input);
    assert!(list.depends_on.is_empty());
}

#[test]
fn serialize_then_deserialize_is_identity() {
    let original = TaskList {
        name: "auth".to_owned(),
        description: "Auth module".to_owned(),
        depends_on: vec!["setup".to_owned()],
        tasks: vec![
            Task {
                id: "1.1".to_owned(),
                description: "Scaffold".to_owned(),
                kind: TaskKind::Agent,
            },
            Task {
                id: "1.2".to_owned(),
                description: "Test".to_owned(),
                kind: TaskKind::Command { command: "test".to_owned() },
            },
        ],
    };

    let serialized = toml::to_string(&original).unwrap();
    let roundtripped: TaskList = toml::from_str(&serialized).unwrap();
    assert_eq!(original, roundtripped);
}
