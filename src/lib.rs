//! Reproducible AGDK Android APK builds in constrained-egress
//! environments.
//!
//! The AGP + AGDK + NDK pipeline hard-depends on hosts under
//! `*.google.com`, which many agent and CI environments block by
//! allowlist. `cargo-agdk` works around this with a content-addressed
//! toolchain bundle (NDK + SDK pieces + warm Gradle cache) hosted as
//! GitHub release assets on cargo-agdk's own repository. The bundle
//! is identified by the consumer's pinned (AGP, NDK_MAJOR, Gradle)
//! tuple — `cargo agdk verify` reads the pins from conventional
//! paths, downloads the matching bundle, and runs cargo-ndk +
//! gradlew offline.
//!
//! Two roles:
//!
//! - **Maintainer / publish role** (`cargo agdk publish ...`): a
//!   full-egress host with the NDK installed. Builds a bundle for a
//!   given pin tuple by priming a vendored stock AGDK sample
//!   project, then uploads it to cargo-agdk's own GitHub releases
//!   via `gh`. Runs once per pin tuple, ever.
//! - **Consumer / verify role** (`cargo agdk verify <package>`):
//!   constrained-egress; runs every change. Anonymous fetch from the
//!   release host (cargo-agdk's repo, public), sha-verified by the
//!   manifest the maintainer uploaded alongside the bundle.

pub mod cache;
pub mod error;
pub mod manifest;
pub mod pins;
pub mod publish;
pub mod release;
pub mod verify;
