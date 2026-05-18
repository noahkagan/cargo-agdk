//! Reproducible AGDK Android APK builds in constrained-egress
//! environments.
//!
//! The AGP + AGDK + NDK pipeline hard-depends on hosts under
//! `*.google.com`, which many agent and CI environments block by
//! allowlist. `cargo-agdk` works around this by vendoring
//! `$ANDROID_HOME` + a warm `$GRADLE_USER_HOME` cache into release
//! assets on a public GitHub repo (the "release host"), then pulling
//! and sha256-verifying them into a per-user cache so `cargo ndk` +
//! `./gradlew --offline` can produce an APK without reaching any
//! Google host.
//!
//! Two roles are involved, and they may be different hosts:
//!
//! - **Publish role.** Has full network egress; runs once per pin
//!   bump. Uses `cargo agdk package` to produce the tarballs and
//!   compute the content-addressed release tag + lock; uploads the
//!   tarballs to the release host.
//! - **Verify role.** Constrained-egress; runs every time a change
//!   needs to be self-tested. Uses `cargo agdk verify <target>`,
//!   which auto-installs the bundle and drives the build offline.
//!
//! Every path the tool reads or writes is configurable via the
//! consumer's `agdk.toml` (defaults follow AGP conventions). Pin
//! formats (how versions are extracted from each pin file) are
//! hardcoded — they're dictated by upstream tools.

pub mod cache;
pub mod clean;
pub mod config;
pub mod error;
pub mod info;
pub mod install;
pub mod lock;
pub mod manifest;
pub mod package;
pub mod target;
pub mod verify;
pub mod version_check;
