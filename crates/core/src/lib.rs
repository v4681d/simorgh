//! simorgh-core — engine traits, pipeline orchestration, commitment policies,
//! configuration, and latency instrumentation.
//!
//! Everything AI-related in Simorgh goes through the traits in [`traits`]:
//! [`traits::SttEngine`], [`traits::Translator`], [`traits::TtsEngine`].
//! Segmentation/commitment strategies implement [`traits::CommitPolicy`].

pub mod config;
pub mod error;
pub mod latency;
pub mod pipeline;
pub mod policy;
pub mod traits;
pub mod types;

pub use error::{Error, Result};
