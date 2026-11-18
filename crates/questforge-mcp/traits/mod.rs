//! Service Provider Interface (SPI) traits for `questforge-mcp`.
//!
//! ```compile_fail
//! use questforge_mcp::traits::llm_provider_spi::LlmProviderSpi;
//! ```

mod llm_provider_spi;

pub use self::llm_provider_spi::LlmProviderSpi;
