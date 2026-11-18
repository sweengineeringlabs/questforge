//! Contract tests for `QuestforgeCapability`.

use questforge_api::QuestforgeCapability;

struct StubCapability {
    name: &'static str,
}

impl QuestforgeCapability for StubCapability {
    fn capability_name(&self) -> &str {
        self.name
    }
}

#[test]
fn test_capability_name_returns_expected_happy() {
    let cap = StubCapability { name: "doctor" };
    assert_eq!(cap.capability_name(), "doctor");
}

#[test]
fn test_capability_name_empty_error() {
    let cap = StubCapability { name: "" };
    assert!(cap.capability_name().is_empty());
}

#[test]
fn test_capability_name_trimmed_edge() {
    let cap = StubCapability {
        name: "quest-doctor",
    };
    assert!(!cap.capability_name().trim().is_empty());
}
