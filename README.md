# cargo-agdk

Reproducible AGDK Android APK builds in constrained-egress environments
— vendors the NDK + SDK + warm Gradle cache as content-addressed
GitHub release assets, no Google-host access required.

## Install

    cargo install --locked cargo-agdk

## Setup

Create `agdk.toml` at your workspace root. A minimum example:

```toml
release-host = "<your-org>/<your-toolchain-host-repo>"

[[target]]
name    = "main"
package = "my-game"
flavor  = "main"
cdylib  = "my_game_lib"
```

See `agdk.toml.example` for all configurable paths (lockfile,
android-project, pin file paths) — every path used by cargo-agdk has
a default but can be overridden.

## Usage

    # Publish role (full-egress host, once per pin bump):
    cargo agdk package --output ./toolchain-out
    # ...followed by `gh release create / upload` to the configured
    # release-host's GitHub releases.

    # Verify role (constrained-egress consumer, every change):
    cargo agdk verify main

## License

Dual-licensed under MIT and Apache-2.0.
