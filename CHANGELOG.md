# Changelog

All notable changes to this crate are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2026-07-20

### Added

- Crates.io metadata (`license`, `description`, `repository`, `documentation`,
  `readme`, `keywords`, `categories`), `LICENSE-MIT`/`LICENSE-APACHE`, and a
  `README.md` with a verified-working example, in preparation for the first
  crates.io release.
- Crate-level and per-item docs closing every `missing_docs` gap on the
  public API surface.

### Fixed

- `EventDispatcher::dispatch` now reports undelivered events via
  `DispatchError<E>` when dispatch fails partway through, instead of
  silently dropping them.
- `testing::block_on` now times out instead of spinning forever when a
  future never completes.
- Fixed a broken intra-doc link (`application::UseCase` ->
  `application::usecase::UseCase`).

### Changed

- `UuidV4Generator`/`UuidV7Generator` now share their generation logic
  instead of duplicating it.

### Documentation

- Clarified the `Delete` port's idempotent-on-missing-id contract.
- Clarified the `Save` port's `Conflict` contract.
- Clarified identity equality vs. derived `PartialEq` on `Entity`.
- Documented the `type_name` invariant on `ValidationError`.
- Documented that redaction is a `Debug`-impl responsibility on `SecretVo`.
- Documented the `variants()`/`from_str` consistency expectation on `EnumVo`.

## [0.1.0] - 2026-07-20

### Added

- `domain`: `Entity`, `AggregateRoot`, `ValueObject`, `EntityId`, `SecretVo`,
  `EnumVo`, `DomainEvent`, `ValidationError`.
- `port`: `Load`/`Save`/`Delete` repository traits, `EventDispatcher`,
  `Clock`, `IdGenerator`, `PortError`.
- `adapter` (behind the `chrono`/`uuid` features): `SystemClock`,
  `UuidV4Generator`, `UuidV7Generator`.
- `mock`: `InMemoryStore`, `FixedClock`, `FixedIdGenerator`.
- `application`: `UseCase`.

### Fixed

- `UseCase` regained its `Sync` bound.
- `InMemoryStore::save` is now atomic across events and aggregates.
- Poisoned mutexes in `mock` are recovered from instead of panicking forever.
- `SystemClock` tests no longer assert wall-clock monotonicity.

[Unreleased]: https://github.com/ryoshrimp/ddd-toolkit-core/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/ryoshrimp/ddd-toolkit-core/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/ryoshrimp/ddd-toolkit-core/releases/tag/v0.1.0
