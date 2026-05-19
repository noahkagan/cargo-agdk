//! Consumer hot path. Read pins from a conventional Android project
//! layout, fetch the matching bundle's manifest from cargo-agdk's
//! releases, download + sha-verify + extract assets, then run
//! cargo-ndk + gradlew offline against the consumer's project.

use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use sha2::{Digest, Sha256};

use crate::cache;
use crate::error::{Error, Result};
use crate::manifest::{self, AssetKind, Manifest};
use crate::pins::{self, Pins};
use crate::release;

/// `cargo agdk verify <package>`.
pub fn run(package: &str, android_project: &Path, workspace_root: &Path) -> Result<()> {
    let pins = pins::read(android_project)?;
    ensure_installed(&pins)?;

    let flavor = kebab_to_camel(package);
    let android_home = cache::android_home(&pins)?;
    let gradle_home = cache::gradle_user_home(&pins)?;
    let ndk_home = cache::ndk_home(&pins)?;

    let jnilibs = android_project
        .join("app/src")
        .join(&flavor)
        .join("jniLibs");
    std::fs::create_dir_all(&jnilibs)?;

    println!("cargo-agdk: cargo ndk -t arm64-v8a build --release -p {package}");
    let status = Command::new("cargo")
        .args(["ndk", "-t", "arm64-v8a", "-o"])
        .arg(&jnilibs)
        .args(["build", "--release", "-p", package])
        .env("ANDROID_NDK_HOME", &ndk_home)
        .env("ANDROID_NDK_ROOT", &ndk_home)
        .current_dir(workspace_root)
        .status()?;
    if !status.success() {
        return Err(Error::CargoNdkFailed(status.code().unwrap_or(-1)));
    }

    let cap = capitalize_first(&flavor);
    let task = format!("assemble{cap}Debug");
    println!("cargo-agdk: ./gradlew --offline --no-daemon {task}");
    let gradlew = android_project.join("gradlew");
    let status = Command::new(&gradlew)
        .args(["--offline", "--no-daemon", &task])
        .env("ANDROID_HOME", &android_home)
        .env("ANDROID_SDK_ROOT", &android_home)
        .env("GRADLE_USER_HOME", &gradle_home)
        .env("ANDROID_NDK_HOME", &ndk_home)
        .current_dir(android_project)
        .status()?;
    if !status.success() {
        return Err(Error::GradleFailed(status.code().unwrap_or(-1)));
    }

    let apk = android_project.join(format!(
        "app/build/outputs/apk/{flavor}/debug/app-{flavor}-debug.apk",
    ));
    if !apk.exists() {
        return Err(Error::ApkNotFound(apk));
    }
    println!("cargo-agdk: OK — APK at {}", apk.display());
    Ok(())
}

/// Install the toolchain bundle for `pins` if not already present.
/// Once the per-tuple cache root carries `.installed`, returns
/// immediately — no network — so steady-state verify is fully
/// offline. The manifest is fetched lazily on first install.
pub fn ensure_installed(pins: &Pins) -> Result<()> {
    let root = cache::root(pins)?;
    if root.join(".installed").exists() {
        return Ok(());
    }
    let manifest = fetch_manifest(pins)?;
    std::fs::create_dir_all(&root)?;
    for &kind in manifest::ALL {
        download_and_extract(pins, &manifest, kind, &root)?;
    }
    std::fs::write(root.join(".installed"), release::tag(pins))?;
    println!("cargo-agdk: toolchain installed at {}", root.display());
    Ok(())
}

/// Download the bundle's `manifest.toml` from cargo-agdk's release at
/// the tag derived from the consumer's pins. 404 here is the
/// "no published bundle for this pin tuple" case.
fn fetch_manifest(pins: &Pins) -> Result<Manifest> {
    let url = release::manifest_url(pins);
    println!("cargo-agdk: fetching manifest {url}");
    let resp = match ureq::get(&url).call() {
        Ok(r) => r,
        Err(ureq::Error::Status(404, _)) => {
            return Err(Error::NoBundle {
                agp: pins.agp.clone(),
                ndk: pins.ndk.clone(),
                gradle: pins.gradle.clone(),
                url,
            });
        }
        Err(e) => {
            return Err(Error::Network {
                url,
                source: Box::new(e),
            });
        }
    };
    let text = resp.into_string()?;
    Manifest::from_toml(&text)
}

fn download_and_extract(
    pins: &Pins,
    manifest: &Manifest,
    kind: AssetKind,
    root: &Path,
) -> Result<()> {
    let url = release::asset_url(pins, kind);
    println!("cargo-agdk: fetching {url}");
    let resp = ureq::get(&url).call().map_err(|e| Error::Network {
        url: url.clone(),
        source: Box::new(e),
    })?;
    let mut reader = resp.into_reader();

    let tmp = root.join(format!(".download.{}", kind.as_str()));
    let mut hasher = Sha256::new();
    let total = {
        let mut out = File::create(&tmp)?;
        let mut chunk = [0u8; 64 * 1024];
        let mut total: u64 = 0;
        loop {
            let n = reader.read(&mut chunk)?;
            if n == 0 {
                break;
            }
            hasher.update(&chunk[..n]);
            out.write_all(&chunk[..n])?;
            total += n as u64;
        }
        out.flush()?;
        total
    };
    println!(
        "cargo-agdk: downloaded {} ({} MiB)",
        kind.as_str(),
        total / (1024 * 1024),
    );

    let actual = format!("{:x}", hasher.finalize());
    let expected = manifest.sha256_for(kind);
    if actual != expected {
        let _ = std::fs::remove_file(&tmp);
        return Err(Error::ChecksumMismatch {
            asset: kind.as_str().into(),
            expected: expected.into(),
            actual,
            tag: release::tag(pins),
        });
    }

    let dest = root.join(kind.extract_into());
    std::fs::create_dir_all(&dest)?;
    println!(
        "cargo-agdk: extracting {} into {}",
        kind.as_str(),
        dest.display(),
    );
    let file = File::open(&tmp)?;
    let dec = zstd::stream::read::Decoder::new(file)?;
    let mut ar = tar::Archive::new(dec);
    ar.set_preserve_permissions(true);
    ar.unpack(&dest)?;
    std::fs::remove_file(&tmp)?;
    Ok(())
}

fn kebab_to_camel(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut upper_next = false;
    for c in s.chars() {
        if c == '-' || c == '_' {
            upper_next = true;
        } else if upper_next {
            out.extend(c.to_uppercase());
            upper_next = false;
        } else {
            out.push(c);
        }
    }
    out
}

fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// Locate the workspace root by walking up from `cwd` until a
/// `Cargo.toml` with a top-level `[workspace]` table is found.
pub fn find_workspace_root(start: &Path) -> Result<PathBuf> {
    let mut p = start.to_path_buf();
    loop {
        let candidate = p.join("Cargo.toml");
        if let Ok(text) = std::fs::read_to_string(&candidate) {
            if let Ok(v) = text.parse::<toml::Value>() {
                if v.get("workspace").is_some() {
                    return Ok(p);
                }
            }
        }
        if !p.pop() {
            return Err(Error::Other(format!(
                "no [workspace] Cargo.toml above {}",
                start.display(),
            )));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kebab_to_camel_basic() {
        assert_eq!(kebab_to_camel("hello-world"), "helloWorld");
        assert_eq!(kebab_to_camel("my-game"), "myGame");
        assert_eq!(kebab_to_camel("solo"), "solo");
    }

    #[test]
    fn kebab_to_camel_multiple_segments() {
        assert_eq!(kebab_to_camel("one-two-three"), "oneTwoThree");
    }

    #[test]
    fn capitalize_first_basic() {
        assert_eq!(capitalize_first("arenaZero"), "ArenaZero");
        assert_eq!(capitalize_first(""), "");
    }
}
