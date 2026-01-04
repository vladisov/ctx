//! Core domain models and logic for ctx
//!
//! This crate contains:
//! - Domain models (Pack, Artifact, Snapshot)
//! - Render engine (deterministic payload generation)
//! - Core business logic

pub mod artifact;
pub mod error;
pub mod pack;
pub mod render;
pub mod snapshot;

pub use artifact::{Artifact, ArtifactType};
pub use error::{CoreError, Result};
pub use pack::{Pack, RenderPolicy};
pub use render::{RenderEngine, RenderRequest, RenderResult};
pub use snapshot::{Snapshot, SnapshotItem};
