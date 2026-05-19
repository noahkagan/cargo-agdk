use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PinKind {
    Agp,
    Ndk,
    Gradle,
}

impl std::fmt::Display for PinKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            PinKind::Agp => "AGP",
            PinKind::Ndk => "NDK",
            PinKind::Gradle => "Gradle",
        })
    }
}

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
        "no published bundle for AGP {agp}, NDK {ndk}, Gradle {gradle} \
         at {url}. Ask the cargo-agdk maintainer to publish one \
         (`cargo agdk publish --agp {agp} --ndk {ndk} --gradle {gradle}`), \
         or pin your project to a supported tuple."
    )]
    NoBundle {
        agp: String,
        ndk: String,
        gradle: String,
        url: String,
    },

    #[error(
        "checksum mismatch for `{asset}`: expected {expected}, got {actual}. \
         The release at {tag} may be corrupt — re-publish it."
    )]
    ChecksumMismatch {
        asset: String,
        expected: String,
        actual: String,
        tag: String,
    },

    #[error("could not parse {file}: {reason}")]
    Parse { file: PathBuf, reason: String },

    #[error("cargo ndk exited with status {0}")]
    CargoNdkFailed(i32),

    #[error("gradle exited with status {0}")]
    GradleFailed(i32),

    #[error("expected APK at {0} but it's missing")]
    ApkNotFound(PathBuf),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;
