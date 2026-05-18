use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use cargo_agdk::config::Config;

#[derive(Parser)]
#[command(
    name = "cargo-agdk",
    bin_name = "cargo agdk",
    about = "Reproducible AGDK Android APK builds in constrained-egress environments.",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Cmd,

    /// Path to agdk.toml. Defaults to walking up from cwd.
    #[arg(long, global = true, value_name = "PATH")]
    config: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Download, sha256-verify, and extract the toolchain bundle
    /// (as identified by the configured lockfile) into the local cache.
    Install,
    /// Build the debug APK for a target: cargo ndk + Gradle
    /// `assemble<Flavor>Debug` in offline mode, then assert the APK
    /// exists. Auto-installs the toolchain on first run.
    Verify {
        /// Target name — matches a `[[target]]` entry's `name` in
        /// agdk.toml.
        target: String,
    },
    /// Publish-role: pack the three release assets, derive the
    /// content-addressed `release_tag`, and write the lockfile. Run
    /// on a full-egress host with `$ANDROID_HOME` and a warm
    /// `$GRADLE_USER_HOME` in place.
    Package {
        /// Output directory for the tarballs.
        #[arg(long, default_value = "./toolchain-out")]
        output: PathBuf,
    },
    /// Print config + lockfile + cache state.
    Info,
    /// Remove the per-release-tag cache directory.
    Clean,
}

fn main() -> ExitCode {
    // When invoked as `cargo agdk <args>`, cargo passes "agdk" as the
    // first positional argument. Strip it before clap sees argv so
    // both `cargo agdk verify main` and `cargo-agdk verify main`
    // parse identically.
    let mut args: Vec<std::ffi::OsString> = std::env::args_os().collect();
    if args.get(1).map(|s| s == "agdk").unwrap_or(false) {
        args.remove(1);
    }
    let cli = Cli::parse_from(args);

    let result = (|| -> cargo_agdk::error::Result<()> {
        let config = Config::load(cli.config.as_deref())?;
        match cli.command {
            Cmd::Install => cargo_agdk::install::run(&config),
            Cmd::Verify { target } => cargo_agdk::verify::run(&config, &target),
            Cmd::Package { output } => cargo_agdk::package::run(&config, &output),
            Cmd::Info => cargo_agdk::info::run(&config),
            Cmd::Clean => cargo_agdk::clean::run(&config),
        }
    })();

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("cargo-agdk: {e}");
            ExitCode::FAILURE
        }
    }
}
