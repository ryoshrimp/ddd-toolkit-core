//! Test doubles for the [`crate::port`] traits: fixed/in-memory
//! implementations useful in tests, or as a starting point for a real
//! adapter.

/// A [`crate::port::clock::Clock`] that always returns a configured time.
#[cfg(feature = "chrono")]
pub mod clock;
/// An [`crate::port::id::IdGenerator`] that always returns a fixed id.
pub mod id;
/// A `Load`/`Save`/`Delete` implementation backed by an in-memory `HashMap`.
pub mod repository;
