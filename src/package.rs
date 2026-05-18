use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

use sha2::{Digest, Sha256};

use crate::config::Config;
use crate::error::{Error, Result};
use crate::lock::Lock;
use crate::manifest;
use crate::version_check;

/// Publish-role: bundle the locally-installed Android toolchain into
/// the three release-asset tarballs, derive the content-addressed
/// `release_tag` (`android-toolchain-agp{AGP}-ndk{NDK_MAJOR}-gradle{GRADLE}-{SHA8}`),
/// and write everything to the configured lockfile. Run on a
/// full-egress host that has `$ANDROID_HOME` and a warm
/// `$GRADLE_USER_HOME` in place (one prior `./gradlew assembleDebug`).
pub fn run(config: &Config, output: &Path) -> Result<()> {
    refuse_placeholder_host(&config.release_host)?;
    std::fs::create_dir_all(output)?;

    let agp_version = version_check::read_agp_version(config)?;
    let ndk_version = version_check::read_ndk_version(config)?;
    let gradle_version = version_check::read_gradle_version(config)?;

    let android_home = resolve_env_path(&["ANDROID_HOME", "ANDROID_SDK_ROOT"])
        .ok_or_else(|| Error::Other("ANDROID_HOME / ANDROID_SDK_ROOT not set".into()))?;
    let gradle_home = resolve_env_path(&["GRADLE_USER_HOME"]).unwrap_or_else(|| {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("/"))
            .join(".gradle")
    });

    if !android_home.join("ndk").join(&ndk_version).exists() {
        return Err(Error::Other(format!(
            "NDK {} not present under {} — install it first",
            ndk_version,
            android_home.display(),
        )));
    }
    if !gradle_home.join("caches").exists() {
        return Err(Error::Other(format!(
            "no Gradle cache at {} — prime it with `./gradlew assembleDebug` first",
            gradle_home.display(),
        )));
    }

    let ndk_tarball = output.join(manifest::ASSET_NDK.filename);
    let sdk_tarball = output.join(manifest::ASSET_SDK.filename);
    let cache_tarball = output.join(manifest::ASSET_GRADLE_CACHE.filename);

    println!("cargo-agdk: packing NDK -> {}", ndk_tarball.display());
    tar_zstd(
        &android_home,
        &[&format!("ndk/{ndk_version}")],
        &ndk_tarball,
    )?;

    println!(
        "cargo-agdk: packing SDK pieces -> {}",
        sdk_tarball.display()
    );
    let sdk_subdirs: Vec<String> = [
        "cmdline-tools",
        "platform-tools",
        "build-tools",
        "platforms",
    ]
    .iter()
    .filter(|sub| android_home.join(sub).exists())
    .map(|s| (*s).to_string())
    .collect();
    if sdk_subdirs.is_empty() {
        return Err(Error::Other(
            "no SDK subdirs (cmdline-tools / build-tools / platforms / platform-tools) found"
                .into(),
        ));
    }
    let sdk_refs: Vec<&str> = sdk_subdirs.iter().map(String::as_str).collect();
    tar_zstd(&android_home, &sdk_refs, &sdk_tarball)?;

    println!(
        "cargo-agdk: packing Gradle cache -> {}",
        cache_tarball.display(),
    );
    let cache_subdirs: Vec<String> = ["caches", "wrapper"]
        .iter()
        .filter(|sub| gradle_home.join(sub).exists())
        .map(|s| (*s).to_string())
        .collect();
    let cache_refs: Vec<&str> = cache_subdirs.iter().map(String::as_str).collect();
    tar_zstd(&gradle_home, &cache_refs, &cache_tarball)?;

    let ndk_sha = sha256_file(&ndk_tarball)?;
    let sdk_sha = sha256_file(&sdk_tarball)?;
    let gc_sha = sha256_file(&cache_tarball)?;

    let release_tag = derive_release_tag(
        &agp_version,
        &ndk_version,
        &gradle_version,
        &ndk_sha,
        &sdk_sha,
        &gc_sha,
    );

    let lock = Lock {
        release_tag: release_tag.clone(),
        agp_version,
        ndk_version,
        gradle_version,
        ndk_sha256: ndk_sha,
        sdk_sha256: sdk_sha,
        gradle_cache_sha256: gc_sha,
    };

    let prior = Lock::load(config).ok();
    if prior.as_ref() == Some(&lock) {
        println!(
            "cargo-agdk: lockfile already current (release_tag={})",
            lock.release_tag,
        );
    } else {
        lock.save(config)?;
        println!(
            "cargo-agdk: wrote {} (release_tag={})",
            config.lockfile_abs().display(),
            lock.release_tag,
        );
    }

    println!();
    for (asset, path) in [
        (&manifest::ASSET_NDK, &ndk_tarball),
        (&manifest::ASSET_SDK, &sdk_tarball),
        (&manifest::ASSET_GRADLE_CACHE, &cache_tarball),
    ] {
        let size_mib = std::fs::metadata(path)?.len() / (1024 * 1024);
        println!(
            "  {:14} sha256 = {}  ({} MiB)",
            asset.name,
            lock.sha256_for(asset.name)
                .expect("manifest asset name has a sha256 mapping"),
            size_mib,
        );
    }

    Ok(())
}

/// agdk.toml placeholders (anything containing `TBD`) block publish so
/// a consumer never accidentally tries to upload to a non-existent
/// repo. The verify-side error from missing assets would be confusing.
fn refuse_placeholder_host(host: &str) -> Result<()> {
    if host.contains("TBD") {
        return Err(Error::PlaceholderReleaseHost(host.into()));
    }
    Ok(())
}

/// `android-toolchain-agp{AGP}-ndk{NDK_MAJOR}-gradle{GRADLE}-{SHA8}`.
/// SHA8 is the first 8 hex chars of
/// `sha256(ndk_sha256 || sdk_sha256 || gradle_cache_sha256)`.
fn derive_release_tag(
    agp: &str,
    ndk: &str,
    gradle: &str,
    ndk_sha: &str,
    sdk_sha: &str,
    cache_sha: &str,
) -> String {
    let ndk_major = ndk.split('.').next().unwrap_or(ndk);
    let mut hasher = Sha256::new();
    hasher.update(ndk_sha.as_bytes());
    hasher.update(sdk_sha.as_bytes());
    hasher.update(cache_sha.as_bytes());
    let combined = format!("{:x}", hasher.finalize());
    let sha8 = &combined[..8];
    format!("android-toolchain-agp{agp}-ndk{ndk_major}-gradle{gradle}-{sha8}")
}

fn resolve_env_path(keys: &[&str]) -> Option<PathBuf> {
    for k in keys {
        if let Some(v) = std::env::var_os(k) {
            return Some(PathBuf::from(v));
        }
    }
    None
}

/// Shell out to `tar --zstd` rather than building the tar+zstd
/// stream ourselves: the publish role runs on a Linux/macOS host
/// where GNU/BSD tar is universal, and shelling out avoids a 1 GiB-
/// class write pipeline through the Rust `tar` + `zstd` crates'
/// allocators.
fn tar_zstd(working_dir: &Path, subdirs: &[&str], dest: &Path) -> Result<()> {
    let mut cmd = Command::new("tar");
    cmd.arg("--zstd")
        .arg("-cf")
        .arg(dest)
        .arg("-C")
        .arg(working_dir);
    for s in subdirs {
        cmd.arg(s);
    }
    let status = cmd.status()?;
    if !status.success() {
        return Err(Error::Other(format!(
            "tar failed (status {}) producing {}",
            status.code().unwrap_or(-1),
            dest.display(),
        )));
    }
    Ok(())
}

fn sha256_file(path: &Path) -> Result<String> {
    let mut f = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derive_release_tag_is_deterministic() {
        let t1 = derive_release_tag("8.4.0", "27.2.12479018", "8.6", "a", "b", "c");
        let t2 = derive_release_tag("8.4.0", "27.2.12479018", "8.6", "a", "b", "c");
        assert_eq!(t1, t2);
    }

    #[test]
    fn derive_release_tag_includes_pins_and_sha8() {
        let tag = derive_release_tag("8.4.0", "27.2.12479018", "8.6", "a", "b", "c");
        assert!(tag.starts_with("android-toolchain-agp8.4.0-ndk27-gradle8.6-"));
        let sha = tag.rsplit('-').next().unwrap();
        assert_eq!(sha.len(), 8);
        assert!(sha.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn derive_release_tag_changes_with_content() {
        let t1 = derive_release_tag("8.4.0", "27.2.12479018", "8.6", "a", "b", "c");
        let t2 = derive_release_tag("8.4.0", "27.2.12479018", "8.6", "a", "b", "d");
        assert_ne!(t1, t2);
    }

    #[test]
    fn derive_release_tag_changes_with_pins() {
        let t1 = derive_release_tag("8.4.0", "27.2.12479018", "8.6", "a", "b", "c");
        let t2 = derive_release_tag("8.5.0", "27.2.12479018", "8.6", "a", "b", "c");
        assert_ne!(t1, t2);
    }

    #[test]
    fn derive_release_tag_takes_ndk_major_only() {
        let tag = derive_release_tag("8.4.0", "28.0.99999", "8.6", "a", "b", "c");
        assert!(tag.contains("-ndk28-"));
        assert!(!tag.contains("-ndk28.0.99999-"));
    }

    #[test]
    fn refuses_tbd_release_host() {
        assert!(matches!(
            refuse_placeholder_host("acme/TBD-toolchain"),
            Err(Error::PlaceholderReleaseHost(_))
        ));
        assert!(refuse_placeholder_host("acme/toolchain").is_ok());
    }
}
