//! Read each pinned version out of the consumer's pin files. Path
//! locations are config-driven (`agp-file`, `ndk-file`,
//! `gradle-file` in agdk.toml). Parsing rules are fixed for v0.1.0:
//!
//! - AGP file: `agp = "X"` key in TOML (matches AGP's
//!   `libs.versions.toml` `[versions]` block).
//! - NDK file: whole-file content, trimmed.
//! - Gradle file: extract `X` from
//!   `distributionUrl=...gradle-X-bin.zip` (gradle-wrapper.properties).

use crate::config::Config;
use crate::error::{Error, Result};
use crate::lock::Lock;

/// Tiny grep — `agp = "8.4.0"` lives in `[versions]` of the AGP
/// libs catalog. Pulling the `toml` crate for one lookup is wasted
/// dep weight; this stays correct for the `key = "value"` shape
/// that's used in practice.
fn parse_versioned_key(text: &str, key: &str) -> Option<String> {
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(key) {
            let rest = rest.trim_start();
            if !rest.starts_with('=') {
                continue;
            }
            let rest = rest[1..].trim();
            if let Some(v) = rest.strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
                return Some(v.to_string());
            }
        }
    }
    None
}

pub fn read_agp_version(config: &Config) -> Result<String> {
    let path = config.agp_file_abs();
    let text = std::fs::read_to_string(&path)?;
    parse_versioned_key(&text, "agp").ok_or_else(|| Error::Parse {
        file: path,
        reason: "no `agp = \"…\"` line found".into(),
    })
}

pub fn read_ndk_version(config: &Config) -> Result<String> {
    let path = config.ndk_file_abs();
    let raw = std::fs::read_to_string(&path)?;
    Ok(raw.trim().to_string())
}

/// Extract `X` from `distributionUrl=...gradle-X-bin.zip`.
pub fn read_gradle_version(config: &Config) -> Result<String> {
    let path = config.gradle_file_abs();
    let text = std::fs::read_to_string(&path)?;
    for line in text.lines() {
        if let Some(rest) = line.trim().strip_prefix("distributionUrl=") {
            if let Some(idx) = rest.find("gradle-") {
                let after = &rest[idx + "gradle-".len()..];
                if let Some(end) = after.find("-bin") {
                    return Ok(after[..end].to_string());
                }
            }
        }
    }
    Err(Error::Parse {
        file: path,
        reason: "no `distributionUrl=…gradle-X-bin.zip` line found".into(),
    })
}

/// Fail fast if the repo's pinned AGP / NDK / Gradle doesn't match
/// the toolchain bundle's pin snapshot. Better an upfront error
/// than a confusing gradle failure mid-build.
pub fn assert_pinned(config: &Config, lock: &Lock) -> Result<()> {
    let agp = read_agp_version(config)?;
    if agp != lock.agp_version {
        return Err(Error::AgpMismatch {
            file: config.agp_file_abs(),
            in_repo: agp,
            pinned: lock.agp_version.clone(),
        });
    }
    let ndk = read_ndk_version(config)?;
    if ndk != lock.ndk_version {
        return Err(Error::NdkMismatch {
            file: config.ndk_file_abs(),
            in_repo: ndk,
            pinned: lock.ndk_version.clone(),
        });
    }
    let gradle = read_gradle_version(config)?;
    if gradle != lock.gradle_version {
        return Err(Error::GradleMismatch {
            file: config.gradle_file_abs(),
            in_repo: gradle,
            pinned: lock.gradle_version.clone(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"[versions]
agp = "8.4.0"
gamesActivity = "4.4.2"
appcompat = "1.6.1"

[libraries]
games-activity = { group = "androidx.games", name = "games-activity", version.ref = "gamesActivity" }
"#;

    #[test]
    fn parses_agp_from_libs_versions_toml() {
        assert_eq!(parse_versioned_key(SAMPLE, "agp"), Some("8.4.0".into()));
    }

    #[test]
    fn parses_games_activity_version() {
        assert_eq!(
            parse_versioned_key(SAMPLE, "gamesActivity"),
            Some("4.4.2".into())
        );
    }

    #[test]
    fn returns_none_for_missing_key() {
        assert_eq!(parse_versioned_key(SAMPLE, "kotlin"), None);
    }

    #[test]
    fn does_not_match_substring() {
        assert_eq!(parse_versioned_key(SAMPLE, "app"), None);
    }

    #[test]
    fn handles_whitespace_variants() {
        let txt = "agp  =   \"9.0.0\"\n";
        assert_eq!(parse_versioned_key(txt, "agp"), Some("9.0.0".into()));
    }
}
