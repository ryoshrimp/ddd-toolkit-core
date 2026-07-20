//! The ports a domain depends on: repository, event dispatch, clock, and id
//! generation.

/// A source of the current time.
#[cfg(feature = "chrono")]
pub mod clock;
/// Publishing recorded [`crate::domain::DomainEvent`]s.
pub mod event;
/// Generating [`crate::domain::EntityId`]s.
pub mod id;
/// Loading, saving, and deleting [`crate::domain::AggregateRoot`]s.
pub mod repository;

mod error;

pub use error::*;
