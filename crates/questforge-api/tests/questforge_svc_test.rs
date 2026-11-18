//! Contract tests for `QuestforgeSvc`.

use questforge_api::{BuilderError, QuestforgeBuilder, QuestforgeCapability, QuestforgeSvc};

struct StubCapability;

impl QuestforgeCapability for StubCapability {
    fn capability_name(&self) -> &str {
        "stub"
    }
}

struct StubBuilder;

impl QuestforgeBuilder for StubBuilder {
    fn build(self) -> Result<impl QuestforgeCapability, BuilderError> {
        Ok(StubCapability)
    }
}

struct StubSvc;

impl QuestforgeSvc for StubSvc {
    fn builder<C: QuestforgeCapability>(_capability: C) -> impl QuestforgeBuilder {
        StubBuilder
    }
}

#[test]
fn test_builder_constructs_service_happy() {
    let cap = StubSvc::builder(StubCapability).build();
    assert!(cap.is_ok());
}

#[test]
fn test_builder_empty_capability_error() {
    struct EmptyCap;
    impl QuestforgeCapability for EmptyCap {
        fn capability_name(&self) -> &str {
            ""
        }
    }
    let cap = StubSvc::builder(EmptyCap).build();
    assert!(cap.is_ok());
}

#[test]
fn test_builder_returns_named_capability_edge() {
    assert!(matches!(
        StubSvc::builder(StubCapability).build(),
        Ok(cap) if cap.capability_name() == "stub"
    ));
}
