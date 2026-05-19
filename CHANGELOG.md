# Changelog

## v0.2.0 — 2026-05-19

Architectural rewrite. cargo-agdk now owns bundle production +
hosting; consumers run a single `cargo agdk verify <package>` with
zero configuration.

Removed:

- `agdk.toml` config file (no per-consumer file needed).
- `release-host` field (hard-coded to this repo).
- `[[target]]` list (cargo package name is the positional arg;
  Gradle flavor inferred kebab→camel).
- Local `toolchain.lock` (manifest is published with each release).
- `cargo agdk package` subcommand (replaced by `publish`).
- `--config <path>` flag.

Added:

- `cargo agdk publish --agp X --ndk Y --gradle Z` — maintainer
  command that primes a vendored stock AGDK sample, tars the
  bundle, and uploads to this repo's releases via `gh`.
- Vendored `stock-sample/` Gradle project for cache priming.
- `cargo agdk info` — prints resolved pins and the expected
  release URL.
- `--project <path>` publish override for unusual consumer dep
  graphs the stock cache doesn't cover.

## v0.1.0 — 2026-05-18

Initial release. Configurable paths (lockfile, android-project, pin
files) with sensible AGP-convention defaults. Pin formats are fixed
for v0.1.0.
