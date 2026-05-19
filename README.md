# cargo-agdk

Reproducible AGDK Android APK builds in constrained-egress environments
— fetches a content-addressed toolchain bundle (NDK + SDK + warm Gradle
cache) from this repo's GitHub releases, no Google-host access required.

## Install

    cargo install --git https://github.com/noahkagan/cargo-agdk --locked --tag v0.2.0 cargo-agdk

(Crates.io publish is a follow-up; `cargo install --locked cargo-agdk`
will work once that lands.)

## Consumer usage

Zero configuration. From any cargo workspace with an `android/` Gradle
project alongside cargo packages:

    cargo agdk verify <package>

This reads the pinned AGP / NDK / Gradle versions from your project's
conventional paths:

- `android/gradle/libs.versions.toml` — AGP version
- `android/ndk.version` — NDK version
- `android/gradle/wrapper/gradle-wrapper.properties` — Gradle version

…constructs the matching bundle's release tag, downloads it from this
repo's releases, sha-verifies against the published manifest, and runs
cargo-ndk + `./gradlew --offline assemble<Flavor>Debug`. Flavor is
inferred kebab→camel from the cargo package name.

If no bundle exists for your pin tuple, you get a clear error
pointing at the `publish` subcommand. Either pin to a supported tuple
or open an issue.

`cargo agdk info` prints the resolved pins, the expected release URL,
and the local cache state. `cargo agdk clean` wipes the cache for the
current tuple.

## Maintainer publishing

For each (AGP, NDK, Gradle) tuple cargo-agdk supports, the maintainer
runs (once, on a full-egress host with `$ANDROID_HOME` carrying the
right NDK, `gradle` and `gh` on PATH):

    cargo agdk publish --agp 8.4.0 --ndk 27.2.12479018 --gradle 8.6

The publish flow:

1. Drops the vendored `stock-sample/` AGDK GameActivity project to a
   temp dir.
2. Rewrites the AGP / NDK pins into the sample.
3. `gradle wrapper --gradle-version=X` bootstraps the wrapper.
4. `./gradlew assembleDebug` primes the cache against the requested
   Gradle.
5. Tars NDK, SDK pieces, and the gradle cache into three assets.
6. sha256s each, writes a manifest.toml.
7. `gh release create` on this repo with the four files attached.

If a consumer's deps aren't in the stock cache, the maintainer can
prime against the consumer's own project instead:

    cargo agdk publish --project /path/to/their/android --agp ... --ndk ... --gradle ...

## License

Dual-licensed under MIT and Apache-2.0.
