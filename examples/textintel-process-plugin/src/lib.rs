//! Reference pre-filter event processor built on the textintel crate.
//!
//! The library half owns settings, mapping, bounded caches, and the enrich
//! core so the process binary stays thin and the latency bench can drive the
//! same code path the host invokes.
//!
//! - [`processor`] — engine orchestration: [`TextIntelProcessor`] owns the
//!   textintel engine plus generation-scoped analysis caches.
//! - [`mapping`] — output rendering: fingerprints become annotations and the
//!   canonical text views.
//! - [`moderation`] — chat moderation: configured-term matching plus the
//!   plugin-owned verdict automations filter on.
//! - [`settings`] — lenient per-field settings parsing.
//! - [`cache`] — the generic FIFO-bounded cache.

pub mod cache;
pub mod mapping;
pub mod moderation;
pub mod processor;
pub mod settings;

pub use cache::BoundedCache;
pub use mapping::INTEL_SCHEMA_VERSION;
pub use moderation::{sanitize_bad_words, TermMatch};
pub use processor::{build_engine, TextIntelProcessor};
pub use settings::{EmojiMode, EngineMode, TextIntelSettings};
