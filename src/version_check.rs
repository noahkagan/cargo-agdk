//! Read each pinned version out of the consumer's pin files. Path
//! locations are config-driven (`agp-file`, `ndk-file`,
//! `gradle-file` in agdk.toml). Parsing rules are fixed for v0.1.0:
//!
//! - AGP file: `agp = "X"` under `[versions]` in TOML (matches AGP's
//!   `libs.versions.toml` catalog format).
//! - NDK file: whole-file content, trimmed.
//! - Gradle file: `distributionUrl=...gradle-X-bin.zip` line in a
//!   `gradle-wrapper.properties` file — a Java properties file, not
//!   TOML, so parsed by hand.

use std::path::Path;

use crate::config::Config;
use crate::error::{Error, PinKind, Result};
use crate::lock::Lock;

pub fn read_agp_version(config: &Config) -> Result<String> {
    let path = config.abs(&config.agp_file);
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

pub fn read_ndk_version(config: &Config) -> Result<String> {
    let raw = std::fs::read_to_string(config.abs(&config.ndk_file))?;
    Ok(raw.trim().to_string())
}

pub fn read_gradle_version(config: &Config) -> Result<String> {
    let path = config.abs(&config.gradle_file);
    let text = std::fs::read_to_string(&path)?;
    parse_gradle_version(&text).ok_or(Error::Parse {
        file: path,
        reason: "no `distributionUrl=…gradle-X-bin.zip` line found".into(),
    })
}

fn parse_gradle_version(text: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let rest = line.trim().strip_prefix("distributionUrl=")?;
        let idx = rest.find("gradle-")?;
        let after = &rest[idx + "gradle-".len()..];
        let end = after.find("-bin")?;
        Some(after[..end].to_string())
    })
}

fn check_pin(kind: PinKind, file: &Path, in_repo: String, pinned: &str) -> Result<()> {
    if in_repo == pinned {
        return Ok(());
    }
    Err(Error::PinMismatch {
        kind,
        file: file.to_path_buf(),
        in_repo,
        pinned: pinned.to_string(),
    })
}

/// Fail fast if the repo's pinned AGP / NDK / Gradle doesn't match
/// the toolchain bundle's pin snapshot. Better an upfront error than
/// a confusing gradle failure mid-build.
pub fn assert_pinned(config: &Config, lock: &Lock) -> Result<()> {
    check_pin(
        PinKind::Agp,
        &config.abs(&config.agp_file),
        read_agp_version(config)?,
        &lock.agp_version,
    )?;
    check_pin(
        PinKind::Ndk,
        &config.abs(&config.ndk_file),
        read_ndk_version(config)?,
        &lock.ndk_version,
    )?;
    check_pin(
        PinKind::Gradle,
        &config.abs(&config.gradle_file),
        read_gradle_version(config)?,
        &lock.gradle_version,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_gradle_finds_version() {
        let txt =
            "distributionUrl=https\\://services.gradle.org/distributions/gradle-8.6-bin.zip\n";
        assert_eq!(parse_gradle_version(txt), Some("8.6".into()));
    }

    #[test]
    fn parse_gradle_misses_when_missing() {
        assert_eq!(parse_gradle_version("# empty\n"), None);
    }
}
