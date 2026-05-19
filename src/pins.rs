//! Read the pinned AGP / NDK / Gradle versions from a consumer's
//! Android project files. Paths follow AGP conventions:
//!
//! - `<android-project>/gradle/libs.versions.toml` — AGP version
//!   under `[versions]` table, key `agp`.
//! - `<android-project>/ndk.version` — whole-file content, trimmed.
//! - `<android-project>/gradle/wrapper/gradle-wrapper.properties` —
//!   `distributionUrl=...gradle-X-bin.zip`.

use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pins {
    pub agp: String,
    pub ndk: String,
    pub gradle: String,
}

impl Pins {
    /// Major version segment of `ndk` (the one the toolchain-bundle
    /// tag uses). Falls back to the full string if there's no `.`.
    pub fn ndk_major(&self) -> &str {
        self.ndk.split('.').next().unwrap_or(&self.ndk)
    }
}

pub fn read(android_project: &Path) -> Result<Pins> {
    Ok(Pins {
        agp: read_agp(android_project)?,
        ndk: read_ndk(android_project)?,
        gradle: read_gradle(android_project)?,
    })
}

fn read_agp(android_project: &Path) -> Result<String> {
    let path = android_project.join("gradle/libs.versions.toml");
    let text = std::fs::read_to_string(&path)?;
    let parsed: toml::Value = toml::from_str(&text).map_err(|e| Error::Parse {
        file: path.clone(),
        reason: e.to_string(),
    })?;
    parsed
        .get("versions")
        .and_then(|t| t.get("agp"))
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .ok_or(Error::Parse {
            file: path,
            reason: r#"no `agp = "…"` key under [versions]"#.into(),
        })
}

fn read_ndk(android_project: &Path) -> Result<String> {
    let path = android_project.join("ndk.version");
    let raw = std::fs::read_to_string(&path)?;
    Ok(raw.trim().to_string())
}

fn read_gradle(android_project: &Path) -> Result<String> {
    let path: PathBuf = android_project.join("gradle/wrapper/gradle-wrapper.properties");
    let text = std::fs::read_to_string(&path)?;
    parse_gradle_distribution_url(&text).ok_or(Error::Parse {
        file: path,
        reason: "no `distributionUrl=…gradle-X-bin.zip` line found".into(),
    })
}

fn parse_gradle_distribution_url(text: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let rest = line.trim().strip_prefix("distributionUrl=")?;
        let idx = rest.find("gradle-")?;
        let after = &rest[idx + "gradle-".len()..];
        let end = after.find("-bin")?;
        Some(after[..end].to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ndk_major_extracts_major() {
        let p = Pins {
            agp: "8.4.0".into(),
            ndk: "27.2.12479018".into(),
            gradle: "8.6".into(),
        };
        assert_eq!(p.ndk_major(), "27");
    }

    #[test]
    fn ndk_major_falls_back_when_no_dot() {
        let p = Pins {
            agp: "8.4.0".into(),
            ndk: "27".into(),
            gradle: "8.6".into(),
        };
        assert_eq!(p.ndk_major(), "27");
    }

    #[test]
    fn parses_gradle_url() {
        let txt =
            "distributionUrl=https\\://services.gradle.org/distributions/gradle-8.6-bin.zip\n";
        assert_eq!(parse_gradle_distribution_url(txt), Some("8.6".into()));
    }

    #[test]
    fn rejects_url_without_gradle_dash() {
        assert_eq!(parse_gradle_distribution_url("foo=bar\n"), None);
    }
}
