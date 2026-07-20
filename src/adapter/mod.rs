//! Real (non-test-double) implementations of the [`crate::port`] traits.

/// [`crate::port::clock::Clock`] backed by the system wall clock.
#[cfg(feature = "chrono")]
pub mod clock;
/// [`crate::port::id::IdGenerator`] implementations backed by the `uuid` crate.
#[cfg(feature = "uuid")]
pub mod id;
