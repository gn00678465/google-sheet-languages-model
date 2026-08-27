//! Core domain logic for gslm.
//!
//! This crate has no Node/napi dependency so it can be reused by a standalone
//! CLI binary later (see ADR-0002). Everything here is pure, in-memory
//! conversion: no file or network I/O.
//!
//! Vocabulary follows `CONTEXT.md`: Locale, Key, Translation, Catalog, Model,
//! Format, Key separator, Tab.

mod catalog;
mod error;
mod flatten;
mod model;
mod unflatten;

pub use catalog::{Catalog, Format};
pub use error::ConversionError;
pub use flatten::{DEFAULT_SEPARATOR, flatten};
pub use model::{Locale, Model, Table};
pub use unflatten::unflatten;

/// Version of the core crate (crates are not published; the npm package
/// version is surfaced separately by the binding).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
