//! Map a `Pins` tuple to a GitHub release tag + asset URLs.
//!
//! The release host is hard-coded to `noahkagan/cargo-agdk` —
//! cargo-agdk's own repo. v0.2.0 deliberately removed the
//! `release-host` config: bundles are central, not per-consumer.

use crate::manifest::AssetKind;
use crate::pins::Pins;

pub const HOST: &str = "noahkagan/cargo-agdk";

/// `android-toolchain-agp{AGP}-ndk{NDK_MAJOR}-gradle{GRADLE}`.
/// One bundle per (AGP, NDK_MAJOR, Gradle) tuple. No content-hash
/// suffix — the maintainer commits to one canonical bundle per
/// tuple.
pub fn tag(pins: &Pins) -> String {
    format!(
        "android-toolchain-agp{}-ndk{}-gradle{}",
        pins.agp,
        pins.ndk_major(),
        pins.gradle,
    )
}

pub fn manifest_url(pins: &Pins) -> String {
    format!(
        "https://github.com/{}/releases/download/{}/manifest.toml",
        HOST,
        tag(pins),
    )
}

pub fn asset_url(pins: &Pins, kind: AssetKind) -> String {
    format!(
        "https://github.com/{}/releases/download/{}/{}",
        HOST,
        tag(pins),
        kind.filename(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pins() -> Pins {
        Pins {
            agp: "8.4.0".into(),
            ndk: "27.2.12479018".into(),
            gradle: "8.6".into(),
        }
    }

    #[test]
    fn tag_format() {
        assert_eq!(tag(&pins()), "android-toolchain-agp8.4.0-ndk27-gradle8.6");
    }

    #[test]
    fn asset_url_format() {
        assert_eq!(
            asset_url(&pins(), AssetKind::Ndk),
            "https://github.com/noahkagan/cargo-agdk/releases/download/android-toolchain-agp8.4.0-ndk27-gradle8.6/ndk-linux-x86_64.tar.zst",
        );
    }
}
