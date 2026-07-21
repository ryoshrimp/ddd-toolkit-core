# Contributing to ddd-toolkit-core

Thanks for your interest in contributing!

## Reporting bugs / requesting features

Please use the issue templates (bug report / feature request) when opening an
issue - they help make sure we get the context needed to act on it quickly.

## Development setup

- Rust 1.85+ (Edition 2024)
- `cargo test` / `cargo fmt` / `cargo clippy` as usual, no special setup required.

## Running checks locally

Before opening a PR, run the same checks CI runs (see
[`.github/workflows/ci.yml`](.github/workflows/ci.yml)):

```sh
cargo test --no-default-features
cargo test
cargo test --features chrono
cargo test --features uuid
cargo test --features serde
cargo test --all-features

cargo fmt --check
cargo clippy --all-features --all-targets -- -D warnings
```

## Submitting changes

`main` is protected: direct pushes are disabled and all changes go through a
pull request that must pass CI before merging. This project doesn't require a
mandatory review at this stage (it's solo-maintained), but please describe the
change and the reasoning behind it in the PR description - that's what gets
read during merge.

## License

By contributing, you agree that your contributions will be dual-licensed
under the [MIT](LICENSE-MIT) and [Apache-2.0](LICENSE-APACHE) licenses, same
as the rest of this project.
