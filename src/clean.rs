use crate::cache;
use crate::config::Config;
use crate::error::Result;
use crate::lock::Lock;

pub fn run(config: &Config) -> Result<()> {
    let lock = Lock::load(config)?;
    let root = cache::cache_root(&lock.release_tag)?;
    if root.exists() {
        std::fs::remove_dir_all(&root)?;
        println!("cargo-agdk: removed {}", root.display());
    } else {
        println!("cargo-agdk: nothing to clean at {}", root.display());
    }
    Ok(())
}
