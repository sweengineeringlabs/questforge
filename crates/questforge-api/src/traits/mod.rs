//! Domain service traits.
//!
//! ```compile_fail
//! use questforge_api::traits::questforge_svc::QuestforgeSvc;
//! ```
//!
//! ```compile_fail
//! use questforge_api::traits::questforge_capability::QuestforgeCapability;
//! ```

mod llm_credential_provider;
mod questforge_capability;
mod questforge_svc;

pub use self::llm_credential_provider::{LlmCredentialProvider, LlmEndpointConfig};
pub use self::questforge_capability::QuestforgeCapability;
pub use self::questforge_svc::QuestforgeSvc;
