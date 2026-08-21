//! Core domain logic for gslm.
//!
//! This crate has no Node/napi dependency so it can be reused by a standalone
//! CLI binary later (see ADR-0002).

mod flatten;

pub use flatten::{DEFAULT_SEPARATOR, FlattenError, flatten};

/// Version of the core crate, surfaced through the Node binding.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
