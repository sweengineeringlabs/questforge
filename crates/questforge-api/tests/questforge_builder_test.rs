//! Contract tests for `QuestforgeBuilder`.

use questforge_api::{BuilderError, CrateError, QuestforgeBuilder, QuestforgeCapability};

struct StubCapability;

impl QuestforgeCapability for StubCapability {
    fn capability_name(&self) -> &str {
        "stub"
    }
}

struct StubBuilder {
    fail: bool,
}

impl QuestforgeBuilder for StubBuilder {
    fn build(self) -> Result<impl QuestforgeCapability, BuilderError> {
        if self.fail {
            return Err(CrateError::Internal {
                message: "missing config".to_string(),
            });
        }
        Ok(StubCapability)
    }
}

#[test]
fn test_build_succeeds_happy() {
    let cap = StubBuilder { fail: false }.build();
    assert!(cap.is_ok());
}

#[test]
fn test_build_missing_config_error() {
    let res = StubBuilder { fail: true }.build();
    assert!(res.is_err());
}

#[test]
fn test_build_returns_named_service_edge() {
    assert!(matches!(
        StubBuilder { fail: false }.build(),
        Ok(cap) if cap.capability_name() == "stub"
    ));
}
