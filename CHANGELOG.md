# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

While Nohrs is pre-alpha (`0.x`), minor versions may include breaking changes.

## [Unreleased]

### Changed

- Split the single `nohrs` crate into a Cargo workspace of six layered crates
  (`nohrs-core`, `nohrs-models`, `nohrs-services`, `nohrs-ui`, `nohrs-pages`, and
  the `nohrs` binary), with a strict downward dependency direction. Shared
  package metadata, dependency versions, and lints are inherited from
  `[workspace.*]`. The toolkit-free crates build on Linux CI via
  `default-members`; the GUI crates build with `--workspace` (macOS). See
  [ADR 0003](docs/adr/0003-cargo-workspace-layer-split.md).
- `FileEntryDto` moved to `nohrs-models` so the UI layer no longer depends on
  services; the app shell (`NohrsApp`) moved from `nohrs-ui` to the `nohrs`
  binary to keep `nohrs-ui` reusable and dependency-correct.

### Added

- OSS hygiene baseline: CI workflow, Dependabot config, issue/PR templates,
  `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `SECURITY.md`, and this changelog.
- Package metadata (`description`, `repository`, `homepage`, `license`,
  `keywords`, `categories`, `authors`, `rust-version`) so `cargo publish` works.

[Unreleased]: https://github.com/noh-rs/nohrs/commits/develop
