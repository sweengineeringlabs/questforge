//! Core QuestForge service contract.

use crate::{QuestforgeBuilder, QuestforgeCapability};

/// Primary service facade contract for the `questforge` domain.
pub trait QuestforgeSvc: Send + Sync {
    /// Constructs a builder for the QuestForge service given a capability.
    fn builder<C: QuestforgeCapability>(capability: C) -> impl QuestforgeBuilder;
}
