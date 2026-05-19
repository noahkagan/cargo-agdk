//! Structural constants for the toolchain bundle's release assets.
//! Asset filenames are fixed — they're the names the bundle is
//! uploaded under on the release-host. The release-host itself comes
//! from `agdk.toml` and is passed in at call sites.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetKind {
    Ndk,
    Sdk,
    GradleCache,
}

impl AssetKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            AssetKind::Ndk => "ndk",
            AssetKind::Sdk => "sdk",
            AssetKind::GradleCache => "gradle-cache",
        }
    }

    pub fn filename(&self) -> &'static str {
        match self {
            AssetKind::Ndk => "ndk-linux-x86_64.tar.zst",
            AssetKind::Sdk => "sdk-pieces-linux-x86_64.tar.zst",
            AssetKind::GradleCache => "gradle-cache.tar.zst",
        }
    }

    /// Where to extract this asset — relative to the cache root.
    pub fn extract_into(&self) -> &'static str {
        match self {
            AssetKind::Ndk | AssetKind::Sdk => "android-home",
            AssetKind::GradleCache => "gradle-user-home",
        }
    }
}

pub const ALL: &[AssetKind] = &[AssetKind::Ndk, AssetKind::Sdk, AssetKind::GradleCache];

pub fn asset_url(kind: AssetKind, release_tag: &str, release_host: &str) -> String {
    format!(
        "https://github.com/{}/releases/download/{}/{}",
        release_host,
        release_tag,
        kind.filename(),
    )
}
