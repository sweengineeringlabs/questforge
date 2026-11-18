//! Builder contract for QuestForge services.

use crate::{BuilderError, QuestforgeCapability};

/// Constructs configured QuestForge service instances.
pub trait QuestforgeBuilder: Sized + Send + Sync {
    /// Builds the configured capability.
    fn build(self) -> Result<impl QuestforgeCapability, BuilderError>;
}
