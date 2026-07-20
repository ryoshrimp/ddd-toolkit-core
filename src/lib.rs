//! Traits and reference implementations for Domain-Driven Design building
//! blocks: entities, value objects, aggregates, repositories, and event
//! dispatch.
//!
//! - [`domain`] - the domain-layer traits: [`domain::Entity`],
//!   [`domain::AggregateRoot`], [`domain::ValueObject`]/[`domain::Wrapped`],
//!   [`domain::EntityId`], [`domain::SecretVo`], [`domain::EnumVo`],
//!   [`domain::DomainEvent`].
//! - [`port`] - the ports a domain depends on: repository (`Load`/`Save`/
//!   `Delete`), [`port::event::EventDispatcher`], [`port::clock::Clock`],
//!   [`port::id::IdGenerator`].
//! - [`adapter`] - real adapters for those ports (behind the `chrono`/`uuid`
//!   features).
//! - [`mock`] - in-memory/fixed adapters, useful as test doubles or a
//!   starting point for a real backend.
//! - [`application`] - [`application::usecase::UseCase`].
//!
//! Pair this crate with
//! [`ddd-toolkit-macro`](https://docs.rs/ddd-toolkit-macro) for
//! `#[derive(...)]` support on the value-object-shaped traits, or depend on
//! [`ddd-toolkit`](https://docs.rs/ddd-toolkit) to get both behind a single
//! dependency.

pub mod adapter;
pub mod application;
/// The domain layer: entities, value objects, aggregates, and events.
pub mod domain;
pub mod mock;
pub mod port;

#[cfg(test)]
mod testing;
