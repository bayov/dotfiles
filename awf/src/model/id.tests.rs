use super::validate_id;
use crate::result::Error;

#[test]
fn simple_hyphenated_id_is_valid() {
    assert!(validate_id("add-oauth").is_ok());
}

#[test]
fn namespaced_id_with_slash_is_valid() {
    assert!(validate_id("infra/deploy").is_ok());
}

#[test]
fn alphanumeric_id_is_valid() {
    assert!(validate_id("deploy123").is_ok());
}

#[test]
fn id_with_underscore_is_valid() {
    assert!(validate_id("my_workflow").is_ok());
}

#[test]
fn mixed_case_id_is_valid() {
    assert!(validate_id("MyWorkflow").is_ok());
}

#[test]
fn empty_string_is_invalid() {
    let err = validate_id("").unwrap_err();
    assert!(matches!(err, Error::InvalidId { .. }));
}

#[test]
fn id_with_space_is_invalid() {
    let err = validate_id("has space").unwrap_err();
    assert!(matches!(err, Error::InvalidId { id, .. } if id == "has space"));
}

#[test]
fn id_with_dot_is_invalid() {
    let err = validate_id("bad.id").unwrap_err();
    assert!(matches!(err, Error::InvalidId { .. }));
}

#[test]
fn id_with_at_sign_is_invalid() {
    let err = validate_id("user@host").unwrap_err();
    assert!(matches!(err, Error::InvalidId { .. }));
}
