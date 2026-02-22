# Performance Specification

## Overview

This document describes the performance characteristics, known bottlenecks, and scaling considerations for the `vorpal-artifacts` project. It reflects what actually exists in the codebase as of the current state -- not aspirational goals.

The project is a Rust build-time configuration tool that defines ~45 software artifacts for the Vorpal build system. Its "runtime" is the `vorpal build` invocation, which compiles the Rust binary and then hands off artifact build instructions to the Vorpal engine. Performance considerations are split between two phases: (1) the Rust compilation of the artifact definitions, and (2) the execution of artifact builds by the Vorpal runtime.

---

## Artifact Definition Phase (Rust Compilation)

### Compile-Time Characteristics

- **Dependency footprint**: The project has a moderate dependency tree (~2,630 lines in `Cargo.lock`). Key dependencies are `anyhow`, `indoc`, `tokio` (with `rt-multi-thread` feature), and `vorpal-sdk` (from Git).
- **Git dependency**: `vorpal-sdk` is sourced from a Git branch (`main`), which means every fresh `cargo build` must fetch the latest commit. This adds network latency to cold builds and makes builds non-reproducible unless `Cargo.lock` is committed (it is).
- **Protobuf code generation**: The `vorpal-sdk` dependency triggers `prost` protobuf code generation at build time (visible in `target/debug/build/vorpal-sdk-*/out/*.rs`). This adds compile-time cost for generated `.rs` files covering agent, artifact, archive, context, and worker protobuf schemas.
- **Binary size**: A single binary (`vorpal`) is produced. All 45+ artifact modules are compiled into this binary regardless of which artifacts are actually needed at runtime.

### No Compile-Time Optimizations Present

- No `[profile.release]` optimizations are configured in `Cargo.toml` (e.g., `lto`, `codegen-units`, `opt-level`).
- No feature flags exist to conditionally compile subsets of artifacts.
- No workspace structure -- the project is a single crate.

---

## Artifact Build Phase (Vorpal Runtime)

### Sequential Execution Model

The main entry point (`src/vorpal.rs`) builds all artifacts **sequentially** using `await` chains. Each `.build(context).await?` call completes before the next begins. There is no parallelism at the artifact definition level within this codebase.

```
let libevent = Libevent::new().build(context).await?;
let libgpg_error = LibgpgError::new().build(context).await?;
// ... every artifact awaits in sequence
```

Whether the underlying Vorpal runtime (`context.run().await`) parallelizes the actual builds is outside the scope of this project. This project's responsibility is defining the build graph; execution is delegated to the Vorpal engine.

### Dependency Graph Impact

The dependency graph creates a critical path that constrains parallelism even if the Vorpal engine supports it:

- **Longest chain**: `libgpg_error` -> `libassuan` / `libgcrypt` / `libksba` -> `gpg` (3 levels deep, with 5 dependencies converging)
- **Shared dependencies**: `ncurses` is a dependency of `tmux`, `zsh`, `nnn`, and `readline` (which itself feeds `nnn`). Building `ncurses` is on the critical path for 4+ artifacts.
- **Independent artifacts**: The majority of artifacts (~30 out of 45) have no dependencies on other artifacts in this project. They download pre-built binaries and could theoretically build in parallel.

### Artifact Build Categories and Performance Profiles

Artifacts fall into three distinct performance profiles:

#### 1. Pre-built Binary Downloads (Fast)

~30 artifacts download pre-built binaries and extract them. Build time is dominated by network download speed.

Examples: `jj`, `bat`, `fd`, `ripgrep`, `kubectl`, `helm`, `terraform`, `starship`, `k9s`, `lazygit`, `argocd`, `bottom`, `cue`, `direnv`, `doppler`, `fluxcd`, `golangci-lint`, `jq`, `just`, `kn`, `kubeseal`, `lima`, `neovim`, `yq`, `beads`, `openjdk`.

Typical build script pattern:
```bash
mkdir -pv "$VORPAL_OUTPUT/bin"
cp ./source/<name>/<binary> "$VORPAL_OUTPUT/bin/<binary>"
chmod +x "$VORPAL_OUTPUT/bin/<binary>"
```

#### 2. Source Compilations with `configure && make` (Slow)

~10 artifacts compile from source using autotools. These are the slowest artifacts.

Examples: `ncurses`, `libevent`, `libgpg_error`, `libassuan`, `libgcrypt`, `libksba`, `npth`, `gpg`, `readline`, `pkg_config`, `nginx`, `tmux`, `nnn`, `zsh`, `sqlite3`.

**Notable**: None of these use `make -j` for parallel compilation. All use bare `make`, which defaults to single-threaded compilation. This is a significant performance gap for source-built artifacts.

#### 3. Go Source Builds (Medium)

3 artifacts use the `vorpal-sdk` Go builder: `crane`, `skopeo`, `umoci`. Build time depends on Go compilation and Go module download speeds.

#### 4. Special Cases

- `awscli2`: Platform-specific extraction (Linux installer vs macOS `.pkg` expansion).
- `openapi-generator-cli`: Downloads a JAR file and wraps it with a shell script; depends on `openjdk`.

---

## CI Performance Characteristics

### Changed-Artifact Detection

The CI workflow (`.github/workflows/vorpal.yaml`) uses `script/detect-changed-artifacts.sh` to avoid building unchanged artifacts. This is the project's primary CI performance optimization.

- Detection works by comparing `git diff --name-only --diff-filter=d` between base and head SHAs.
- Only files matching `src/artifact/*.rs` trigger rebuilds.
- Filename-to-artifact conversion uses `snake_case` -> `kebab-case` mapping.
- **Limitation**: Changes to shared code (`src/lib.rs`, `src/artifact.rs`, `src/vorpal.rs`, `Cargo.toml`) do not trigger any artifact rebuilds, even though they could affect all artifacts.

### CI Matrix Strategy

- The `build-dev` job runs on 4 runners in parallel: `macos-latest`, `macos-latest-large`, `ubuntu-latest`, `ubuntu-latest-arm64`. This covers all 4 target platforms simultaneously.
- The `build` job uses a matrix of `artifact x runner`, with `fail-fast: false` so a failure in one artifact does not block others.
- The `build` job depends on `build-dev` completing first.

### Registry-Based Caching

The CI workflow configures an S3-backed Vorpal registry (`altf4llc-vorpal-registry`). This means previously built artifacts are cached in S3 and reused when their definition has not changed. The caching mechanism is provided by the Vorpal engine, not by this project's code.

---

## Caching Strategy

### What Exists

- **Vorpal registry (S3)**: Artifact outputs are cached in an S3 bucket. The Vorpal engine handles cache key computation (likely based on source hash + build script content). This is the primary caching layer.
- **Vorpal.lock**: A lockfile (`Vorpal.lock`) records resolved artifact configurations. It is generated by `vorpal build` and committed to the repository. CI uploads it as a GitHub Actions artifact per platform.
- **Cargo dependency caching**: No explicit Cargo cache configuration exists in CI (no `actions/cache` for `target/` or `~/.cargo`).
- **Git fetch-depth 0**: The `build-changes` job fetches full history for accurate diff detection. The `build-dev` and `build` jobs use default (shallow) checkout.

### What Does Not Exist

- No application-level caching within the Rust code.
- No download caching for artifact sources (e.g., no local cache of downloaded tarballs between runs). Source caching is presumably handled by the Vorpal engine.
- No incremental build support within individual artifact definitions.
- No Cargo build caching in CI (e.g., `sccache`, `actions/cache` for `target/`).

---

## Concurrency and Parallelism

### Within This Codebase

- **Tokio runtime**: The project uses `tokio` with `rt-multi-thread`, but the main function executes all artifact builds sequentially via `.await` chains. The multi-threaded runtime is required by the Vorpal SDK, not utilized for parallelism within this project.
- **No `tokio::spawn`**: No tasks are spawned concurrently. No `join!` or `JoinSet` usage.
- **No parallel iteration**: No `rayon` or similar parallel iterator usage.
- **`&mut ConfigContext`**: The mutable reference to `ConfigContext` prevents parallel artifact definition -- only one artifact can register itself with the context at a time. This is a fundamental design constraint from the Vorpal SDK.

### In CI

- CI achieves parallelism through GitHub Actions matrix strategy (4 platforms x N artifacts).
- Each matrix cell runs independently with its own Vorpal instance.

---

## Known Bottlenecks

1. **Sequential artifact registration**: All 45+ artifacts are registered sequentially due to `&mut ConfigContext`. Even independent artifacts cannot be registered concurrently.

2. **Single-threaded `make`**: Source-built artifacts use bare `make` without `-j` flags. On multi-core CI runners, this wastes available CPU capacity during compilation of `ncurses`, `gpg`, `nginx`, `tmux`, `readline`, etc.

3. **GPG dependency chain depth**: The `gpg` artifact has the deepest dependency chain (5 library dependencies, each requiring source compilation). This creates the longest critical path in the build graph.

4. **No conditional compilation**: All 45+ artifact definitions are compiled into every binary, even when building a single artifact. The Vorpal engine selects which to build at runtime, but compile time includes all definitions.

5. **Git-sourced SDK dependency**: `vorpal-sdk` from Git branch `main` requires network access for every clean build. No pinned tag or crate registry version is used.

6. **No Cargo build caching in CI**: Each CI run compiles the Rust binary from scratch (modulo any runner-level caching from the `setup-vorpal-action`).

---

## Benchmarking and Profiling

### What Exists

- **No benchmarking infrastructure**: No `cargo bench`, no criterion benchmarks, no performance tests.
- **No timing instrumentation**: No elapsed-time logging, no build duration tracking within artifact definitions.
- **No profiling configuration**: No `perf`, `flamegraph`, or profiling-related tooling configured.

### What CI Provides

- GitHub Actions provides job-level timing in the workflow UI.
- Individual artifact build times are visible in the Vorpal build output (provided by the Vorpal engine, not this project).

---

## Scaling Considerations

### Current Scale

- ~45 artifacts defined across 47 source files.
- 4 target platforms (Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux).
- Single `vorpal.rs` entry point that imports and builds all artifacts.

### Scaling Concerns

- **Linear growth in `vorpal.rs`**: Every new artifact adds import lines and build calls to `vorpal.rs`. At 45 artifacts this is manageable; at 200+ it would become unwieldy.
- **Compile time growth**: Each new artifact module adds to the compile time of the single binary. No mechanism exists to compile subsets.
- **CI matrix explosion**: Each new artifact adds a row to the CI matrix (artifact x 4 platforms). With 45 artifacts and 4 platforms, a full build is 180 CI jobs.
- **Dependency fan-out**: Shared dependencies like `ncurses` create coupling. A version bump to `ncurses` would require rebuilding `tmux`, `zsh`, `nnn`, and `readline` (and transitively `nnn` again through `readline`).

### What Would Help at Scale

- Feature flags or workspace splits to compile artifact subsets.
- Parallel artifact registration (would require Vorpal SDK changes to `ConfigContext`).
- `make -j$(nproc)` for source-built artifacts.
- Cargo build caching (`sccache` or `actions/cache`) in CI.
- Download caching for artifact source tarballs.

---

## Summary

The project has minimal performance engineering within its own codebase. Performance is primarily determined by:

1. The Vorpal engine's caching and build execution strategy (outside this project).
2. CI workflow design (changed-artifact detection, matrix parallelism, S3 registry caching).
3. Individual artifact build characteristics (download vs. source compilation).

The main performance opportunities within this project's control are: adding `-j$(nproc)` to `make` invocations in source-built artifacts, adding Cargo build caching to CI, and potentially restructuring the codebase for selective compilation if the artifact count grows significantly.
