use std::path::PathBuf;

use crate::error::{Error, Result};

/// `~/.cache/cargo-agdk/<release_tag>/` (or the OS equivalent via
/// `dirs::cache_dir()`). Keyed on `release_tag` so tag changes land
/// in a sibling dir without clobbering an in-flight working tree on
/// the older toolchain.
pub fn cache_root(release_tag: &str) -> Result<PathBuf> {
    let base = dirs::cache_dir()
        .ok_or_else(|| Error::Other("no user cache directory (HOME unset?)".into()))?;
    Ok(base.join("cargo-agdk").join(release_tag))
}

pub fn android_home(release_tag: &str) -> Result<PathBuf> {
    Ok(cache_root(release_tag)?.join("android-home"))
}

pub fn gradle_user_home(release_tag: &str) -> Result<PathBuf> {
    Ok(cache_root(release_tag)?.join("gradle-user-home"))
}

pub fn ndk_home(release_tag: &str, ndk_version: &str) -> Result<PathBuf> {
    Ok(android_home(release_tag)?.join("ndk").join(ndk_version))
}
