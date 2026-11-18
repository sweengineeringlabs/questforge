//! Core QuestForge capability trait.

/// Capability contract exposed by QuestForge domain services.
pub trait QuestforgeCapability: Send + Sync {
    /// Returns the name of this capability.
    fn capability_name(&self) -> &str;
}
