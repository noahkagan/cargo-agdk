//! Read `agdk.toml` from the consumer's workspace root.
//!
//! Default discovery walks up from cwd looking for `agdk.toml`; the
//! `--config <path>` override skips the walk. The directory holding
//! the config file IS the workspace root that all other configured
//! paths resolve against.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::error::{Error, Result};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(rename = "release-host")]
    pub release_host: String,

    #[serde(default = "default_lockfile")]
    pub lockfile: PathBuf,

    #[serde(rename = "android-project", default = "default_android_project")]
    pub android_project: PathBuf,

    #[serde(rename = "agp-file", default = "default_agp_file")]
    pub agp_file: PathBuf,

    #[serde(rename = "ndk-file", default = "default_ndk_file")]
    pub ndk_file: PathBuf,

    #[serde(rename = "gradle-file", default = "default_gradle_file")]
    pub gradle_file: PathBuf,

    #[serde(default, rename = "target")]
    pub targets: Vec<crate::target::Target>,

    #[serde(skip)]
    pub workspace_root: PathBuf,
}

fn default_lockfile() -> PathBuf {
    PathBuf::from("android/toolchain.lock")
}
fn default_android_project() -> PathBuf {
    PathBuf::from("android")
}
fn default_agp_file() -> PathBuf {
    PathBuf::from("android/gradle/libs.versions.toml")
}
fn default_ndk_file() -> PathBuf {
    PathBuf::from("android/ndk.version")
}
fn default_gradle_file() -> PathBuf {
    PathBuf::from("android/gradle/wrapper/gradle-wrapper.properties")
}

impl Config {
    /// Load and validate the config. When `override_path` is `Some`,
    /// the file at that path is read and its parent directory becomes
    /// the workspace root. Otherwise the loader walks up from cwd
    /// looking for `agdk.toml`.
    pub fn load(override_path: Option<&Path>) -> Result<Self> {
        let config_path = match override_path {
            Some(p) => {
                if !p.exists() {
                    return Err(Error::Other(format!(
                        "--config path does not exist: {}",
                        p.display(),
                    )));
                }
                p.to_path_buf()
            }
            None => find_config()?,
        };
        let absolute = if config_path.is_absolute() {
            config_path.clone()
        } else {
            std::env::current_dir()?.join(&config_path)
        };
        let workspace = absolute
            .parent()
            .ok_or_else(|| {
                Error::Other(format!(
                    "config path has no parent directory: {}",
                    config_path.display(),
                ))
            })?
            .to_path_buf();
        let text = std::fs::read_to_string(&config_path)?;
        let mut cfg: Config = toml::from_str(&text).map_err(|e| Error::Parse {
            file: config_path.clone(),
            reason: e.to_string(),
        })?;
        if cfg.targets.is_empty() {
            return Err(Error::NoTargets);
        }
        cfg.workspace_root = workspace;
        Ok(cfg)
    }

    pub fn abs(&self, relative: &Path) -> PathBuf {
        self.workspace_root.join(relative)
    }
}

fn find_config() -> Result<PathBuf> {
    let cwd = std::env::current_dir()?;
    let mut p = cwd.clone();
    loop {
        let candidate = p.join("agdk.toml");
        if candidate.exists() {
            return Ok(candidate);
        }
        if !p.pop() {
            return Err(Error::ConfigNotFound(cwd));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
release-host = "acme/android-toolchain"

[[target]]
name = "main"
package = "my-game"
flavor = "main"
cdylib = "my_game_lib"
"#;

    #[test]
    fn parses_minimal_config() {
        let cfg: Config = toml::from_str(SAMPLE).unwrap();
        assert_eq!(cfg.release_host, "acme/android-toolchain");
        assert_eq!(cfg.lockfile, PathBuf::from("android/toolchain.lock"));
        assert_eq!(cfg.android_project, PathBuf::from("android"));
        assert_eq!(cfg.targets.len(), 1);
        assert_eq!(cfg.targets[0].name, "main");
        assert_eq!(cfg.targets[0].flavor, "main");
    }

    #[test]
    fn overrides_defaults() {
        let src = r#"
release-host = "acme/x"
lockfile = "custom/toolchain.lock"
android-project = "platform/android"

[[target]]
name = "main"
package = "p"
flavor = "f"
cdylib = "c"
"#;
        let cfg: Config = toml::from_str(src).unwrap();
        assert_eq!(cfg.lockfile, PathBuf::from("custom/toolchain.lock"));
        assert_eq!(cfg.android_project, PathBuf::from("platform/android"));
    }

    #[test]
    fn rejects_unknown_field() {
        let src = r#"
release-host = "acme/x"
typo-field = "oops"

[[target]]
name = "main"
package = "p"
flavor = "f"
cdylib = "c"
"#;
        let err = toml::from_str::<Config>(src).unwrap_err().to_string();
        assert!(err.contains("typo-field"), "got: {err}");
    }
}
