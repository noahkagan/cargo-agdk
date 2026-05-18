//! One target = one thing you can `cargo agdk verify`. Each target
//! names a cargo package, a Gradle product flavor, and the cdylib
//! the package builds. Targets are declared in the consumer's
//! `agdk.toml`.

use serde::Deserialize;

use crate::config::Config;
use crate::error::{Error, Result};

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Target {
    /// Lookup key — the positional argument to `cargo agdk verify`.
    pub name: String,
    /// Cargo package name passed to `cargo build -p <package>`.
    pub package: String,
    /// Gradle product flavor (camelCase). `verify` runs
    /// `./gradlew assemble<Capitalized>Debug`.
    pub flavor: String,
    /// cdylib name — the bundle's jniLib lands at
    /// `<android-project>/app/src/<flavor>/jniLibs/<abi>/lib<cdylib>.so`.
    /// Tracked for documentation; the bundle's filename is determined
    /// by the cargo package's `[lib]` name at build time.
    pub cdylib: String,
}

impl Target {
    /// `assemble<Flavor>Debug` task name. `flavor[0]` uppercased.
    pub fn flavor_capitalized(&self) -> String {
        let mut chars = self.flavor.chars();
        match chars.next() {
            Some(c) => c.to_ascii_uppercase().to_string() + chars.as_str(),
            None => String::new(),
        }
    }
}

pub fn lookup<'a>(config: &'a Config, name: &str) -> Result<&'a Target> {
    config
        .targets
        .iter()
        .find(|t| t.name == name)
        .ok_or_else(|| Error::UnknownTarget(name.into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Target {
        Target {
            name: "main".into(),
            package: "my-game".into(),
            flavor: "myGame".into(),
            cdylib: "my_game_lib".into(),
        }
    }

    #[test]
    fn flavor_capitalization() {
        assert_eq!(fixture().flavor_capitalized(), "MyGame");
    }

    #[test]
    fn lookup_finds_by_name() {
        let cfg = Config {
            release_host: "acme/x".into(),
            lockfile: Default::default(),
            android_project: Default::default(),
            agp_file: Default::default(),
            ndk_file: Default::default(),
            gradle_file: Default::default(),
            targets: vec![fixture()],
            workspace_root: Default::default(),
        };
        assert_eq!(lookup(&cfg, "main").unwrap().package, "my-game");
        assert!(matches!(
            lookup(&cfg, "missing"),
            Err(Error::UnknownTarget(_))
        ));
    }
}
