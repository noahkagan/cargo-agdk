//! Per-bundle cache under `~/.cache/cargo-agdk/<tag>/`. Keyed on
//! the release tag so different pin tuples land in sibling dirs.

use std::path::PathBuf;

use crate::error::{Error, Result};
use crate::pins::Pins;
use crate::release;

pub fn root(pins: &Pins) -> Result<PathBuf> {
    let base = dirs::cache_dir()
        .ok_or_else(|| Error::Other("no user cache directory (HOME unset?)".into()))?;
    Ok(base.join("cargo-agdk").join(release::tag(pins)))
}

pub fn android_home(pins: &Pins) -> Result<PathBuf> {
    Ok(root(pins)?.join("android-home"))
}

pub fn gradle_user_home(pins: &Pins) -> Result<PathBuf> {
    Ok(root(pins)?.join("gradle-user-home"))
}

pub fn ndk_home(pins: &Pins) -> Result<PathBuf> {
    Ok(android_home(pins)?.join("ndk").join(&pins.ndk))
}
