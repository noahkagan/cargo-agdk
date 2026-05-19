use crate::cache;
use crate::config::Config;
use crate::error::Result;
use crate::lock::Lock;
use crate::manifest;

pub fn run(config: &Config) -> Result<()> {
    println!("cargo-agdk {}", env!("CARGO_PKG_VERSION"));
    println!();
    println!("Config:");
    println!("  release-host    : {}", config.release_host);
    println!("  lockfile        : {}", config.lockfile.display());
    println!("  android-project : {}", config.android_project.display());
    println!("  agp-file        : {}", config.agp_file.display());
    println!("  ndk-file        : {}", config.ndk_file.display());
    println!("  gradle-file     : {}", config.gradle_file.display());
    println!("  workspace root  : {}", config.workspace_root.display());
    println!();
    println!("Targets:");
    for t in &config.targets {
        println!(
            "  {:<14} package={} flavor={} cdylib={}",
            t.name, t.package, t.flavor, t.cdylib,
        );
    }
    println!();

    let lockfile_abs = config.abs(&config.lockfile);
    match Lock::load(config) {
        Ok(lock) => {
            println!("Lock ({}):", lockfile_abs.display());
            println!("  release tag  : {}", lock.release_tag);
            println!("  AGP          : {}", lock.agp_version);
            println!("  NDK          : {}", lock.ndk_version);
            println!("  Gradle       : {}", lock.gradle_version);
            println!();
            println!("Assets:");
            for &kind in manifest::ALL {
                println!("  {:14} {}", kind.as_str(), kind.filename());
                println!("                 sha256 = {}", lock.sha256_for(kind));
            }
            println!();
            let root = cache::cache_root(&lock.release_tag)?;
            let installed = root.join(".installed").exists();
            println!("Cache root  : {}", root.display());
            println!("  installed : {}", if installed { "yes" } else { "no" });
        }
        Err(_) => {
            println!("Lock ({}): NOT FOUND or NOT PARSED", lockfile_abs.display());
            println!("  Run `cargo agdk package --output <dir>`");
            println!("  from a full-egress publish host to populate it.");
        }
    }
    Ok(())
}
