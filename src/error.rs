use std::path::PathBuf;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    #[error("network error fetching {url}: {source}")]
    Network {
        url: String,
        #[source]
        source: Box<ureq::Error>,
    },

    #[error(
        "checksum mismatch for `{asset}`: expected {expected}, got {actual}. \
         The release likely hasn't been packaged yet — run \
         `cargo agdk package` from a full-egress publish host and upload \
         the resulting tarballs to the configured release-host."
    )]
    ChecksumMismatch {
        asset: String,
        expected: String,
        actual: String,
    },

    #[error("toolchain not installed at {0}; run `cargo agdk install` first")]
    NotInstalled(PathBuf),

    #[error(
        "AGP version mismatch: {file} says {in_repo}, lock pinned to {pinned}. \
         Either revert the AGP bump in the repo, or re-package the toolchain via \
         `cargo agdk package` on a full-egress publish host and upload the \
         resulting tarballs to the configured release-host."
    )]
    AgpMismatch {
        file: PathBuf,
        in_repo: String,
        pinned: String,
    },

    #[error(
        "NDK version mismatch: {file} says {in_repo}, lock pinned to {pinned}. \
         Re-package the toolchain from a full-egress publish host or revert the NDK bump."
    )]
    NdkMismatch {
        file: PathBuf,
        in_repo: String,
        pinned: String,
    },

    #[error(
        "Gradle version mismatch: {file} says {in_repo}, lock pinned to {pinned}. \
         Re-package the toolchain from a full-egress publish host or revert the Gradle bump."
    )]
    GradleMismatch {
        file: PathBuf,
        in_repo: String,
        pinned: String,
    },

    #[error("cargo ndk exited with status {0}")]
    CargoNdkFailed(i32),

    #[error("gradle exited with status {0}")]
    GradleFailed(i32),

    #[error("expected APK at {0} but it's missing")]
    ApkNotFound(PathBuf),

    #[error("unknown target `{0}`; check the [[target]] entries in agdk.toml")]
    UnknownTarget(String),

    #[error(
        "no agdk.toml found by walking up from {0}. Create one at your \
         workspace root, or pass --config <path>."
    )]
    ConfigNotFound(PathBuf),

    #[error("agdk.toml has no [[target]] entries; at least one is required")]
    NoTargets,

    #[error(
        "release-host in agdk.toml is `{0}` — that's a placeholder. \
         Edit agdk.toml and set release-host to a real `<owner>/<repo>` \
         before publishing."
    )]
    PlaceholderReleaseHost(String),

    #[error("could not parse {file}: {reason}")]
    Parse { file: PathBuf, reason: String },

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;
