//! Per-bundle manifest published alongside the three tarballs on
//! every release. Maps the bundle's pin tuple to the assets' sha256s
//! so the consumer can verify without a local lockfile.

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

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

    /// Where to extract under the per-bundle cache root.
    pub fn extract_into(&self) -> &'static str {
        match self {
            AssetKind::Ndk | AssetKind::Sdk => "android-home",
            AssetKind::GradleCache => "gradle-user-home",
        }
    }
}

pub const ALL: &[AssetKind] = &[AssetKind::Ndk, AssetKind::Sdk, AssetKind::GradleCache];

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    pub agp_version: String,
    pub ndk_version: String,
    pub gradle_version: String,
    pub ndk_sha256: String,
    pub sdk_sha256: String,
    pub gradle_cache_sha256: String,
}

impl Manifest {
    pub fn sha256_for(&self, kind: AssetKind) -> &str {
        match kind {
            AssetKind::Ndk => &self.ndk_sha256,
            AssetKind::Sdk => &self.sdk_sha256,
            AssetKind::GradleCache => &self.gradle_cache_sha256,
        }
    }

    pub fn to_toml(&self) -> String {
        toml::to_string_pretty(self).expect("manifest serializes")
    }

    pub fn from_toml(text: &str) -> Result<Self> {
        toml::from_str(text).map_err(|e| Error::Other(format!("manifest parse: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Manifest {
        Manifest {
            agp_version: "8.4.0".into(),
            ndk_version: "27.2.12479018".into(),
            gradle_version: "8.6".into(),
            ndk_sha256: "ndk".into(),
            sdk_sha256: "sdk".into(),
            gradle_cache_sha256: "gc".into(),
        }
    }

    #[test]
    fn roundtrips_through_toml() {
        let m = fixture();
        let back = Manifest::from_toml(&m.to_toml()).unwrap();
        assert_eq!(m, back);
    }

    #[test]
    fn sha256_for_dispatches() {
        let m = fixture();
        assert_eq!(m.sha256_for(AssetKind::Ndk), "ndk");
        assert_eq!(m.sha256_for(AssetKind::Sdk), "sdk");
        assert_eq!(m.sha256_for(AssetKind::GradleCache), "gc");
    }
}
