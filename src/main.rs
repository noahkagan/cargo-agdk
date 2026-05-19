use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

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
}

#[derive(Subcommand)]
enum Cmd {
    /// Build the debug APK for a cargo package: cargo ndk +
    /// `./gradlew --offline assemble<Flavor>Debug` + APK existence
    /// check. Auto-installs the toolchain bundle (~1 GiB) on first
    /// run for the current pin tuple.
    Verify {
        /// Cargo package name. Also used as the kebab-case lookup
        /// for the Gradle product flavor (kebab → camel — e.g.
        /// `hello-world` → `helloWorld`).
        package: String,
        /// Path to the Android Gradle project root, relative to
        /// the workspace root or absolute. Defaults to `android`.
        #[arg(long, default_value = "android")]
        android_project: PathBuf,
    },
    /// Print resolved pins + cached bundle state.
    Info {
        #[arg(long, default_value = "android")]
        android_project: PathBuf,
    },
    /// Remove the cache directory for the current pin tuple.
    Clean {
        #[arg(long, default_value = "android")]
        android_project: PathBuf,
    },
    /// Maintainer-only: build a bundle for the given pin tuple and
    /// upload it to cargo-agdk's GitHub releases. Requires
    /// `$ANDROID_HOME` with the NDK, `gradle` + `gh` on PATH, and
    /// full network egress.
    Publish {
        #[arg(long)]
        agp: String,
        #[arg(long)]
        ndk: String,
        #[arg(long)]
        gradle: String,
        /// Output directory for the tarballs + manifest before
        /// upload.
        #[arg(long, default_value = "./toolchain-out")]
        output: PathBuf,
        /// Use an existing Gradle project for cache priming instead
        /// of the vendored stock sample. Pointed at a real consumer's
        /// `android/` directory when the stock cache wouldn't cover
        /// the consumer's deps.
        #[arg(long)]
        project: Option<PathBuf>,
        /// Tar + write the manifest but skip the final `gh release`
        /// step. Useful for iterating.
        #[arg(long)]
        skip_upload: bool,
    },
}

fn main() -> ExitCode {
    // `cargo agdk <args>` invocation passes "agdk" as argv[1].
    // Strip it so `cargo-agdk verify ...` and `cargo agdk verify ...`
    // parse identically.
    let mut args: Vec<std::ffi::OsString> = std::env::args_os().collect();
    if args.get(1).map(|s| s == "agdk").unwrap_or(false) {
        args.remove(1);
    }
    let cli = Cli::parse_from(args);

    let result = run(cli);
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("cargo-agdk: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> cargo_agdk::error::Result<()> {
    match cli.command {
        Cmd::Verify {
            package,
            android_project,
        } => {
            let cwd = std::env::current_dir()?;
            let workspace = cargo_agdk::verify::find_workspace_root(&cwd)?;
            let project = absolute(&android_project, &workspace);
            cargo_agdk::verify::run(&package, &project, &workspace)
        }
        Cmd::Info { android_project } => {
            let cwd = std::env::current_dir()?;
            let workspace = cargo_agdk::verify::find_workspace_root(&cwd)?;
            let project = absolute(&android_project, &workspace);
            print_info(&project)
        }
        Cmd::Clean { android_project } => {
            let cwd = std::env::current_dir()?;
            let workspace = cargo_agdk::verify::find_workspace_root(&cwd)?;
            let project = absolute(&android_project, &workspace);
            clean(&project)
        }
        Cmd::Publish {
            agp,
            ndk,
            gradle,
            output,
            project,
            skip_upload,
        } => {
            let pins = cargo_agdk::pins::Pins { agp, ndk, gradle };
            cargo_agdk::publish::run(cargo_agdk::publish::PublishOptions {
                pins,
                output,
                project,
                skip_upload,
            })
        }
    }
}

fn absolute(p: &std::path::Path, workspace: &std::path::Path) -> PathBuf {
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        workspace.join(p)
    }
}

fn print_info(android_project: &std::path::Path) -> cargo_agdk::error::Result<()> {
    let pins = cargo_agdk::pins::read(android_project)?;
    let tag = cargo_agdk::release::tag(&pins);
    let root = cargo_agdk::cache::root(&pins)?;
    let installed = root.join(".installed").exists();

    println!("cargo-agdk {}", env!("CARGO_PKG_VERSION"));
    println!();
    println!("Project");
    println!("  android-project : {}", android_project.display());
    println!();
    println!("Pins (read from project)");
    println!("  AGP    : {}", pins.agp);
    println!("  NDK    : {}", pins.ndk);
    println!("  Gradle : {}", pins.gradle);
    println!();
    println!("Bundle");
    println!("  tag     : {tag}");
    println!(
        "  release : https://github.com/{}/releases/tag/{}",
        cargo_agdk::release::HOST,
        tag
    );
    println!();
    println!("Cache");
    println!("  root      : {}", root.display());
    println!(
        "  installed : {}",
        if installed {
            "yes"
        } else {
            "no (run `cargo agdk verify <pkg>`)"
        }
    );
    Ok(())
}

fn clean(android_project: &std::path::Path) -> cargo_agdk::error::Result<()> {
    let pins = cargo_agdk::pins::read(android_project)?;
    let root = cargo_agdk::cache::root(&pins)?;
    if root.exists() {
        std::fs::remove_dir_all(&root)?;
        println!("cargo-agdk: removed {}", root.display());
    } else {
        println!("cargo-agdk: nothing to clean at {}", root.display());
    }
    Ok(())
}
