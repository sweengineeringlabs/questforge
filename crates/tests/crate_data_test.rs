//! Tests for [`CrateData`].

#[test]
fn test_try_new_invalid_error() {
    let result = crate::vo::CrateData::try_new("");
    assert!(result.is_err());
}

#[test]
fn test_try_new_nominal_happy() {
    let result = crate::vo::CrateData::try_new("valid");
    assert!(result.is_ok());
}
