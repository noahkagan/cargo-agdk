//! Structural constants for the toolchain bundle's release assets.
//! Asset filenames are fixed — they're the names the bundle is
//! uploaded under on the release-host. The release-host itself comes
//! from `agdk.toml` and is passed in at call sites.

pub struct Asset {
    /// Short human-readable name (used in logs + lock-file lookup).
    pub name: &'static str,
    /// Release-asset filename on the release host.
    pub filename: &'static str,
    /// Where to extract the archive — relative to the cache root.
    pub extract_into: &'static str,
}

pub const ASSET_NDK: Asset = Asset {
    name: "ndk",
    filename: "ndk-linux-x86_64.tar.zst",
    extract_into: "android-home",
};
pub const ASSET_SDK: Asset = Asset {
    name: "sdk",
    filename: "sdk-pieces-linux-x86_64.tar.zst",
    extract_into: "android-home",
};
pub const ASSET_GRADLE_CACHE: Asset = Asset {
    name: "gradle-cache",
    filename: "gradle-cache.tar.zst",
    extract_into: "gradle-user-home",
};

pub const ASSETS: &[&Asset] = &[&ASSET_NDK, &ASSET_SDK, &ASSET_GRADLE_CACHE];

pub fn asset_url(asset: &Asset, release_tag: &str, release_host: &str) -> String {
    format!(
        "https://github.com/{}/releases/download/{}/{}",
        release_host, release_tag, asset.filename,
    )
}
