use std::process::Command;

use crate::cache;
use crate::config::Config;
use crate::error::{Error, Result};
use crate::install;
use crate::lock::Lock;
use crate::target;
use crate::version_check;

/// `cargo ndk` + `./gradlew --offline assemble<Flavor>Debug` + APK
/// existence check. Auto-installs the toolchain on first run.
pub fn run(config: &Config, target_name: &str) -> Result<()> {
    let lock = Lock::load(config)?;
    version_check::assert_pinned(config, &lock)?;
    let t = target::lookup(config, target_name)?;

    install::ensure_installed(config, &lock)?;

    let android_home = cache::android_home(&lock.release_tag)?;
    let gradle_home = cache::gradle_user_home(&lock.release_tag)?;
    let ndk_home = cache::ndk_home(&lock.release_tag, &lock.ndk_version)?;

    let android_project = config.android_project_abs();
    let jnilibs = android_project
        .join("app/src")
        .join(&t.flavor)
        .join("jniLibs");
    std::fs::create_dir_all(&jnilibs)?;

    println!(
        "cargo-agdk: cargo ndk -t arm64-v8a build --release -p {}",
        t.package,
    );
    let status = Command::new("cargo")
        .arg("ndk")
        .arg("-t")
        .arg("arm64-v8a")
        .arg("-o")
        .arg(&jnilibs)
        .arg("build")
        .arg("--release")
        .arg("-p")
        .arg(&t.package)
        .env("ANDROID_NDK_HOME", &ndk_home)
        .env("ANDROID_NDK_ROOT", &ndk_home)
        .current_dir(&config.workspace_root)
        .status()?;
    if !status.success() {
        return Err(Error::CargoNdkFailed(status.code().unwrap_or(-1)));
    }

    let task = format!("assemble{}Debug", t.flavor_capitalized());
    println!("cargo-agdk: ./gradlew --offline --no-daemon {}", task);
    let gradlew = android_project.join("gradlew");
    let status = Command::new(&gradlew)
        .arg("--offline")
        .arg("--no-daemon")
        .arg(&task)
        .env("ANDROID_HOME", &android_home)
        .env("ANDROID_SDK_ROOT", &android_home)
        .env("GRADLE_USER_HOME", &gradle_home)
        .env("ANDROID_NDK_HOME", &ndk_home)
        .current_dir(&android_project)
        .status()?;
    if !status.success() {
        return Err(Error::GradleFailed(status.code().unwrap_or(-1)));
    }

    let apk = android_project.join(format!(
        "app/build/outputs/apk/{flavor}/debug/app-{flavor}-debug.apk",
        flavor = t.flavor,
    ));
    if !apk.exists() {
        return Err(Error::ApkNotFound(apk));
    }
    println!("cargo-agdk: OK — APK at {}", apk.display());
    Ok(())
}
