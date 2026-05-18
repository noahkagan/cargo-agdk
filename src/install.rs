use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::cache;
use crate::config::Config;
use crate::error::{Error, Result};
use crate::lock::Lock;
use crate::manifest::{self, Asset};

/// Download + sha-verify + extract every asset in `manifest::ASSETS`
/// into the cache root. Idempotent — the install marker short-circuits
/// repeat runs.
pub fn run(config: &Config) -> Result<()> {
    let lock = Lock::load(config)?;
    run_with_lock(config, &lock)
}

pub fn run_with_lock(config: &Config, lock: &Lock) -> Result<()> {
    let root = cache::cache_root(&lock.release_tag)?;
    if marker_path(&root).exists() {
        println!(
            "cargo-agdk: toolchain already installed at {}",
            root.display(),
        );
        return Ok(());
    }
    std::fs::create_dir_all(&root)?;

    for asset in manifest::ASSETS {
        download_and_extract(config, lock, asset, &root)?;
    }

    write_marker(&root, &lock.release_tag)?;
    println!("cargo-agdk: toolchain installed at {}", root.display());
    Ok(())
}

/// Auto-install hook for `verify`. Returns `Ok(())` if the cache is
/// already complete; otherwise drives a full install.
pub fn ensure_installed(config: &Config, lock: &Lock) -> Result<()> {
    if is_installed(lock)? {
        return Ok(());
    }
    run_with_lock(config, lock)
}

/// True iff the per-release-tag cache root carries the install marker.
pub fn is_installed(lock: &Lock) -> Result<bool> {
    Ok(marker_path(&cache::cache_root(&lock.release_tag)?).exists())
}

fn marker_path(root: &Path) -> PathBuf {
    root.join(".installed")
}

fn write_marker(root: &Path, release_tag: &str) -> Result<()> {
    std::fs::write(marker_path(root), release_tag)?;
    Ok(())
}

fn download_and_extract(config: &Config, lock: &Lock, asset: &Asset, root: &Path) -> Result<()> {
    let url = manifest::asset_url(asset, &lock.release_tag, &config.release_host);
    println!("cargo-agdk: fetching {}", url);

    let resp = ureq::get(&url).call().map_err(|e| Error::Network {
        url: url.clone(),
        source: Box::new(e),
    })?;
    let mut reader = resp.into_reader();

    // Buffer to a temp file under the cache root (same filesystem so
    // we don't pay a cross-fs copy on cleanup). ~1 GiB on disk is
    // cheaper than buffering in memory and keeps memory pressure
    // bounded for low-RAM hosts that might run this.
    let tmp = root.join(format!(".download.{}", asset.name));
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
        asset.name,
        total / (1024 * 1024),
    );

    let actual = format!("{:x}", hasher.finalize());
    let expected = lock
        .sha256_for(asset.name)
        .expect("manifest asset name has a sha256 mapping");
    if actual != expected {
        let _ = std::fs::remove_file(&tmp);
        return Err(Error::ChecksumMismatch {
            asset: asset.name.into(),
            expected: expected.to_string(),
            actual,
        });
    }

    let dest = root.join(asset.extract_into);
    std::fs::create_dir_all(&dest)?;
    println!(
        "cargo-agdk: extracting {} into {}",
        asset.name,
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
