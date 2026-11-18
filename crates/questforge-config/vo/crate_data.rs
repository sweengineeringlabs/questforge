//! [`CrateData`] -- shared data definition.

/// Shared core data representation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CrateData {
    id: String,
}

impl CrateData {
    /// Creates a new [`CrateData`].
    pub fn try_new(id: impl Into<String>) -> Result<Self, crate::error::CrateError> {
        let id = id.into();
        if id.is_empty() {
            return Err(crate::error::CrateError::Internal {
                message: "id cannot be empty".to_string(),
            });
        }
        Ok(Self { id })
    }

    /// Returns the ID.
    pub fn id(&self) -> &str {
        &self.id
    }
}
