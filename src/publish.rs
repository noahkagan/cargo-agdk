//! Maintainer publish path. Builds a bundle for a given (AGP, NDK,
//! Gradle) tuple by priming a vendored stock AGDK sample, tars the
//! result, sha256s each asset, writes a manifest, and uploads to
//! cargo-agdk's own GitHub releases via `gh`.
//!
//! Preconditions on the maintainer's host:
//! - `$ANDROID_HOME` with the requested NDK installed.
//! - `gh` authenticated with write access to `release::HOST`.
//! - Full network egress to `*.google.com` and `services.gradle.org`
//!   (cargo-agdk downloads the requested Gradle distribution itself
//!   — no system gradle needed).

use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use sha2::{Digest, Sha256};

use crate::error::{Error, Result};
use crate::manifest::{AssetKind, Manifest};
use crate::pins::Pins;
use crate::release;

const STOCK_SAMPLE: include_dir::Dir<'static> =
    include_dir::include_dir!("$CARGO_MANIFEST_DIR/stock-sample");

pub struct PublishOptions {
    pub pins: Pins,
    pub output: PathBuf,
    /// If `Some`, use the gradle project at this path instead of the
    /// vendored stock sample. Lets the maintainer prime against a
    /// real consumer's project when the stock cache wouldn't cover
    /// the consumer's deps.
    pub project: Option<PathBuf>,
    /// Skip the final `gh release` step. Useful for `--dry-run`
    /// equivalents while iterating on the publish flow.
    pub skip_upload: bool,
}

pub fn run(opts: PublishOptions) -> Result<()> {
    let pins = &opts.pins;
    require_tools()?;

    let android_home = std::env::var_os("ANDROID_HOME")
        .map(PathBuf::from)
        .ok_or_else(|| Error::Other("ANDROID_HOME not set".into()))?;
    if !android_home.join("ndk").join(&pins.ndk).exists() {
        return Err(Error::Other(format!(
            "NDK {} not present under {} — install it first",
            pins.ndk,
            android_home.display(),
        )));
    }

    // The child gradle process gets `current_dir(project_dir)`, and on
    // Unix `Command::new(p)` resolves `p` in the child's cwd. So every
    // path passed to `Command` (and every dir we change into) has to be
    // absolute — otherwise the gradle launcher (under output/) becomes
    // unfindable from the project dir.
    std::fs::create_dir_all(&opts.output)?;
    let output = opts.output.canonicalize()?;
    let staging = output.join("staging-project");
    if staging.exists() {
        std::fs::remove_dir_all(&staging)?;
    }
    let gradle_user_home = output.join("gradle-user-home");
    if gradle_user_home.exists() {
        std::fs::remove_dir_all(&gradle_user_home)?;
    }
    std::fs::create_dir_all(&gradle_user_home)?;

    let project_dir = match &opts.project {
        Some(p) => {
            println!(
                "publish: priming via consumer-supplied project at {}",
                p.display()
            );
            p.clone()
        }
        None => {
            println!(
                "publish: priming via vendored stock sample at {}",
                staging.display()
            );
            materialize_stock_sample(&staging, pins)?;
            staging.clone()
        }
    };

    let gradle_bin = download_gradle(&pins.gradle, &output)?;

    println!(
        "publish: {} wrapper --gradle-version={}",
        gradle_bin.display(),
        pins.gradle
    );
    let status = Command::new(&gradle_bin)
        .args([
            "wrapper",
            "--gradle-version",
            &pins.gradle,
            "--distribution-type",
            "bin",
        ])
        .current_dir(&project_dir)
        .status()?;
    if !status.success() {
        return Err(Error::Other(format!(
            "bootstrap `gradle wrapper` failed with status {}",
            status.code().unwrap_or(-1),
        )));
    }

    println!("publish: ./gradlew assembleDebug (priming the cache)");
    let status = Command::new(project_dir.join("gradlew"))
        .args(["--no-daemon", "assembleDebug"])
        .env("ANDROID_HOME", &android_home)
        .env("ANDROID_SDK_ROOT", &android_home)
        .env("ANDROID_NDK_HOME", android_home.join("ndk").join(&pins.ndk))
        .env("GRADLE_USER_HOME", &gradle_user_home)
        .current_dir(&project_dir)
        .status()?;
    if !status.success() {
        return Err(Error::GradleFailed(status.code().unwrap_or(-1)));
    }

    let ndk_tarball = output.join(AssetKind::Ndk.filename());
    let sdk_tarball = output.join(AssetKind::Sdk.filename());
    let cache_tarball = output.join(AssetKind::GradleCache.filename());

    println!("publish: packing NDK -> {}", ndk_tarball.display());
    tar_zstd(&android_home, &[&format!("ndk/{}", pins.ndk)], &ndk_tarball)?;

    println!("publish: packing SDK pieces -> {}", sdk_tarball.display());
    let sdk_subdirs: Vec<String> = [
        "cmdline-tools",
        "platform-tools",
        "build-tools",
        "platforms",
    ]
    .iter()
    .filter(|s| android_home.join(s).exists())
    .map(|s| (*s).to_string())
    .collect();
    if sdk_subdirs.is_empty() {
        return Err(Error::Other(
            "no SDK subdirs under $ANDROID_HOME (cmdline-tools / build-tools / platforms / platform-tools)".into(),
        ));
    }
    let sdk_refs: Vec<&str> = sdk_subdirs.iter().map(String::as_str).collect();
    tar_zstd(&android_home, &sdk_refs, &sdk_tarball)?;

    println!(
        "publish: packing Gradle cache -> {}",
        cache_tarball.display()
    );
    let cache_subdirs: Vec<String> = ["caches", "wrapper"]
        .iter()
        .filter(|s| gradle_user_home.join(s).exists())
        .map(|s| (*s).to_string())
        .collect();
    let cache_refs: Vec<&str> = cache_subdirs.iter().map(String::as_str).collect();
    tar_zstd(&gradle_user_home, &cache_refs, &cache_tarball)?;

    let manifest = Manifest {
        agp_version: pins.agp.clone(),
        ndk_version: pins.ndk.clone(),
        gradle_version: pins.gradle.clone(),
        ndk_sha256: sha256_file(&ndk_tarball)?,
        sdk_sha256: sha256_file(&sdk_tarball)?,
        gradle_cache_sha256: sha256_file(&cache_tarball)?,
    };
    let manifest_path = output.join("manifest.toml");
    std::fs::write(&manifest_path, manifest.to_toml())?;
    println!("publish: wrote {}", manifest_path.display());
    println!();
    for &k in crate::manifest::ALL {
        let path = output.join(k.filename());
        let size_mib = std::fs::metadata(&path)?.len() / (1024 * 1024);
        println!(
            "  {:14} sha256 = {}  ({} MiB)",
            k.as_str(),
            manifest.sha256_for(k),
            size_mib,
        );
    }

    if opts.skip_upload {
        println!("publish: --skip-upload set; stopping before gh release.");
        return Ok(());
    }

    upload_release(
        &release::tag(pins),
        pins,
        &[&ndk_tarball, &sdk_tarball, &cache_tarball, &manifest_path],
    )?;
    Ok(())
}

fn require_tools() -> Result<()> {
    let mut missing = Vec::new();
    for cmd in ["gh", "tar", "sha256sum"] {
        let ok = Command::new(cmd)
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if !ok {
            missing.push(cmd);
        }
    }
    if !missing.is_empty() {
        return Err(Error::Other(format!(
            "publish requires tools on PATH: {}",
            missing.join(", "),
        )));
    }
    let status = Command::new("gh")
        .args(["auth", "status"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()?;
    if !status.success() {
        return Err(Error::Other(
            "gh is not authenticated; run `gh auth login`".into(),
        ));
    }
    Ok(())
}

/// Download `gradle-{version}-bin.zip` from services.gradle.org and
/// extract under `output/gradle-{version}/`. Returns the path to the
/// `bin/gradle` binary. Idempotent — re-uses an existing extract if
/// the binary is already in place.
fn download_gradle(version: &str, output: &Path) -> Result<PathBuf> {
    let dist_root = output.join(format!("gradle-{version}"));
    let bin = dist_root.join("bin/gradle");
    if bin.exists() {
        println!("publish: reusing gradle dist at {}", dist_root.display());
        return Ok(bin);
    }

    let url = format!("https://services.gradle.org/distributions/gradle-{version}-bin.zip");
    println!("publish: downloading {url}");
    let resp = ureq::get(&url).call().map_err(|e| Error::Network {
        url: url.clone(),
        source: Box::new(e),
    })?;
    let zip_path = output.join(format!("gradle-{version}-bin.zip"));
    {
        let mut out = File::create(&zip_path)?;
        let mut reader = resp.into_reader();
        let mut chunk = [0u8; 64 * 1024];
        loop {
            let n = reader.read(&mut chunk)?;
            if n == 0 {
                break;
            }
            out.write_all(&chunk[..n])?;
        }
        out.flush()?;
    }

    println!(
        "publish: extracting {} into {}",
        zip_path.display(),
        output.display()
    );
    let file = File::open(&zip_path)?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| Error::Other(format!("opening gradle zip: {e}")))?;
    archive
        .extract(output)
        .map_err(|e| Error::Other(format!("extracting gradle zip: {e}")))?;
    let _ = std::fs::remove_file(&zip_path);

    if !bin.exists() {
        return Err(Error::Other(format!(
            "extracted gradle but {} is missing",
            bin.display(),
        )));
    }
    // zip crate's extract() doesn't preserve Unix permissions; the
    // launcher script lands at 0644 and won't exec.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perm = std::fs::metadata(&bin)?.permissions();
        perm.set_mode(0o755);
        std::fs::set_permissions(&bin, perm)?;
    }
    Ok(bin)
}

/// Drop the vendored sample to `dest` and rewrite the three pin
/// files with the requested versions.
fn materialize_stock_sample(dest: &Path, pins: &Pins) -> Result<()> {
    std::fs::create_dir_all(dest)?;
    STOCK_SAMPLE
        .extract(dest)
        .map_err(|e| Error::Other(format!("extracting stock sample: {e}")))?;
    let libs_versions = dest.join("gradle/libs.versions.toml");
    let text = std::fs::read_to_string(&libs_versions)?;
    let rewritten = text.replace("__AGP_VERSION__", &pins.agp);
    std::fs::write(&libs_versions, rewritten)?;
    std::fs::write(dest.join("ndk.version"), &pins.ndk)?;
    Ok(())
}

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

fn upload_release(tag: &str, pins: &Pins, files: &[&Path]) -> Result<()> {
    let host = release::HOST;
    let exists = Command::new("gh")
        .args(["release", "view", tag, "--repo", host, "--json", "isDraft"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if exists {
        println!("publish: release {tag} already exists on {host}; deleting + recreating");
        let _ = Command::new("gh")
            .args([
                "release",
                "delete",
                tag,
                "--repo",
                host,
                "--yes",
                "--cleanup-tag",
            ])
            .status();
    }

    println!("publish: gh release create {tag} on {host}");
    let notes = format!(
        "Toolchain bundle for AGP {}, NDK {}, Gradle {}. Generated by `cargo agdk publish`.",
        pins.agp, pins.ndk, pins.gradle,
    );
    let mut cmd = Command::new("gh");
    cmd.args([
        "release", "create", tag, "--repo", host, "--title", tag, "--notes", &notes,
    ]);
    for f in files {
        cmd.arg(f);
    }
    let status = cmd.status()?;
    if !status.success() {
        return Err(Error::Other(format!(
            "gh release create failed with status {}",
            status.code().unwrap_or(-1),
        )));
    }
    println!("publish: https://github.com/{}/releases/tag/{}", host, tag,);
    Ok(())
}
