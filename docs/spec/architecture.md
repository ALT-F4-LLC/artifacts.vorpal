# Architecture Specification

## Overview

`artifacts.vorpal` is a Rust-based artifact definition repository that uses the [Vorpal](https://github.com/ALT-F4-LLC/vorpal) build system SDK (`vorpal-sdk`) to declaratively define, build, and manage software artifacts across multiple platforms. It functions as a **catalog of build recipes** -- each artifact module describes how to fetch, compile, and package a specific tool or library for cross-platform use.

The project produces a single binary (`vorpal`) that, when executed with `vorpal build <artifact>`, registers all artifact definitions with the Vorpal build system and then delegates the actual build orchestration to the Vorpal runtime.

## System Context

```
+---------------------+         +------------------------+
|                     |         |                        |
|  artifacts.vorpal   |-------->|  Vorpal Build System   |
|  (this repo)        |  SDK    |  (vorpal-sdk crate)    |
|                     |         |                        |
+---------------------+         +------------------------+
        |                                |
        | defines                        | orchestrates
        v                                v
+---------------------+         +------------------------+
|                     |         |                        |
|  Artifact Sources   |         |  S3 Registry Backend   |
|  (GitHub releases,  |         |  (altf4llc-vorpal-     |
|   tarballs, zips)   |         |   registry)            |
|                     |         |                        |
+---------------------+         +------------------------+
```

### External Dependencies

- **vorpal-sdk**: The core SDK crate, sourced from `https://github.com/ALT-F4-LLC/vorpal.git` (branch: `main`). Provides `ConfigContext`, `Artifact`, `ArtifactSource`, `ArtifactSystem`, `step::shell`, `get_env_key`, language-specific builders (e.g., `Go`), and the `ProjectEnvironment` abstraction.
- **Vorpal CLI**: The `vorpal` CLI tool (installed separately) that this binary communicates with via gRPC (evidenced by `tonic`/`prost` in the SDK's dependency tree). The Vorpal CLI handles services, workers, and the actual build execution.
- **S3 Registry**: Artifact outputs are stored in an S3 bucket (`altf4llc-vorpal-registry`) using AWS credentials, configured via the `setup-vorpal-action` GitHub Action.

## Project Structure

```
.
├── Cargo.toml              # Rust package manifest (single binary crate)
├── Cargo.lock              # Pinned Rust dependency versions
├── Vorpal.toml             # Vorpal configuration (language=rust, source includes)
├── Vorpal.lock             # Pinned source digests per platform (~2700 lines)
├── src/
│   ├── vorpal.rs           # Binary entry point (main function)
│   ├── lib.rs              # Library root: ProjectEnvironment, DEFAULT_SYSTEMS, re-exports
│   ├── artifact.rs         # Module declarations for all artifact submodules
│   └── artifact/           # One file per artifact definition (~55 artifacts)
│       ├── argocd.rs
│       ├── bat.rs
│       ├── file.rs         # Utility: generic file artifact builder
│       ├── gpg.rs          # Complex: multi-dependency source build
│       ├── ttyd.rs         # Complex: platform-conditional build strategies
│       └── ...
├── script/
│   ├── detect-changed-artifacts.sh      # CI: identifies changed artifacts for selective builds
│   ├── test-detect-changed-artifacts.sh # Tests for the detection script
│   ├── lima.sh                          # Lima VM provisioning for Linux builds on macOS
│   └── linux-vorpal-slim.sh             # Rootfs slimming utility for Vorpal Linux
├── .github/
│   ├── workflows/vorpal.yaml            # CI pipeline: build-changes -> build-dev -> build
│   └── renovate.json                    # Automated dependency updates
├── makefile                             # Lima VM management targets
├── lima.yaml                            # Lima VM template for Linux build environments
└── docs/
    └── spec/                            # Project specifications (this directory)
```

## Core Architectural Components

### 1. Binary Entry Point (`src/vorpal.rs`)

The `main` function is the single entry point. It:

1. Obtains a `ConfigContext` via `vorpal_sdk::context::get_context()` (async, sets up gRPC communication with the Vorpal build system).
2. Sequentially registers all artifact definitions by calling `Artifact::new().build(context)` on each one.
3. Registers a development environment (`ProjectEnvironment::new("dev", ...)`) that includes Lima, Protoc, and Rust toolchain artifacts.
4. Calls `context.run().await` to hand control to the Vorpal runtime for actual build execution.

All artifact registrations happen at startup, before any builds execute. The Vorpal runtime selects which artifacts to build based on CLI arguments.

### 2. Library Root (`src/lib.rs`)

Exports:
- `DEFAULT_SYSTEMS`: A constant array of the four supported `ArtifactSystem` variants.
- `ProjectEnvironment`: A wrapper around the SDK's `ProjectEnvironment` that adds project-specific development dependencies (Lima, Protoc, Rust toolchain) with correct environment variable configuration.
- `pub mod artifact`: Re-exports all artifact submodules.

### 3. Artifact Module (`src/artifact.rs` + `src/artifact/`)

The `artifact.rs` file is a flat module declaration file -- it declares `pub mod` for every artifact submodule. There are currently **55 artifact modules** plus 1 utility module (`file.rs`).

### 4. Vorpal Configuration Files

- **`Vorpal.toml`**: Declares the project language (`rust`) and which files to include as sources (`src`, `Cargo.toml`, `Cargo.lock`). This tells Vorpal how to build this project itself.
- **`Vorpal.lock`**: A TOML lockfile containing pinned source digests for every artifact source URL, per platform. Each entry records `name`, `path` (URL), `includes`, `excludes`, `digest` (SHA-256), and `platform`. This file is ~2700 lines and is regenerated when artifact versions change.

## Artifact Architecture Patterns

### Pattern Categories

Artifacts follow one of three build strategy patterns:

#### A. Pre-built Binary Download (Simplest)

Used for tools that publish pre-built binaries per platform. Examples: `argocd`, `bat`, `terraform`, `kubectl`, `neovim`, `starship`, `beads`.

Structure:
1. Map `ArtifactSystem` to platform-specific download URL suffix.
2. Create an `ArtifactSource` from the URL.
3. Write a shell step that copies the binary into `$VORPAL_OUTPUT/bin/`.
4. Register as `Artifact::new(name, steps, systems).with_aliases(...).with_sources(...)`.

#### B. Source Build (No Dependencies)

Used for tools built from source with `./configure && make && make install` but no artifact dependencies. Examples: `nginx`, `ffmpeg`, `zlib`, `sqlite3`, `readline`.

Structure:
1. Download source tarball.
2. Shell step runs configure/make/install with `--prefix="$VORPAL_OUTPUT"`.
3. No artifact dependencies passed to `step::shell`.

#### C. Source Build with Dependencies (Most Complex)

Used for tools that depend on other artifacts built by this same project. Examples: `gpg` (depends on libgpg-error, libassuan, libgcrypt, libksba, npth), `tmux` (depends on libevent, ncurses), `ttyd` (depends on cmake, json-c, libuv, libwebsockets, mbedtls), `zsh` (depends on ncurses).

Structure:
1. Struct fields hold `Option<&'a str>` references to pre-built dependency artifact hashes.
2. Builder pattern with `with_*` methods allows dependency injection.
3. `build()` method resolves dependencies -- if not provided, builds them inline.
4. Dependencies are referenced via `get_env_key()` which converts artifact hashes to environment variable paths.
5. `step::shell` receives dependency artifact hashes so the Vorpal runtime makes them available during build.

#### D. Language-Specific Builder

Used for Go projects via the SDK's `Go` builder. Examples: `skopeo`, `umoci`.

Structure:
1. Uses `vorpal_sdk::artifact::language::go::Go` instead of raw `Artifact` + `step::shell`.
2. Configures build directory, build path, build flags, and source.
3. The SDK handles Go toolchain setup, compilation, and output packaging.

#### E. Platform-Conditional Strategy (Hybrid)

Some artifacts use different strategies per platform. Example: `ttyd` uses pre-built binaries on Linux but builds from source on macOS. `awscli2` uses different installation methods per platform (zip extraction on Linux, pkg extraction on macOS).

### Common Artifact API

Every artifact module follows this contract:

```rust
pub struct ArtifactName {
    // Optional dependency references
}

impl ArtifactName {
    pub fn new() -> Self { ... }
    // Optional: pub fn with_dependency(mut self, dep: &str) -> Self { ... }
    pub async fn build(self, context: &mut ConfigContext) -> Result<String> { ... }
}
```

The `build` method returns `Result<String>` where the `String` is the artifact's content hash, used to reference it as a dependency in other artifacts.

### Dependency Graph

The artifact dependency tree (inferred from `use crate::artifact::*` imports and `with_*` calls):

```
gpg
├── libgpg-error
├── libassuan -> libgpg-error
├── libgcrypt -> libgpg-error
├── libksba -> libgpg-error
└── npth

tmux
├── libevent
└── ncurses

zsh
└── ncurses

ttyd (macOS only, Linux uses prebuilt)
├── cmake
├── json-c
├── libuv -> cmake
├── libwebsockets -> cmake, libuv, mbedtls
└── mbedtls -> cmake

neovim (standalone, prebuilt)
libevent (standalone, source build)
ncurses (standalone, source build)
```

All other artifacts (~45) are standalone with no inter-artifact dependencies.

## Cross-Platform Support

### Supported Platforms

All artifacts target four platforms, defined by the `ArtifactSystem` enum:

| Enum Variant      | Platform Description    |
|-------------------|------------------------|
| `Aarch64Darwin`   | macOS on Apple Silicon  |
| `Aarch64Linux`    | Linux on ARM64          |
| `X8664Darwin`     | macOS on Intel x86_64   |
| `X8664Linux`      | Linux on x86_64         |

### Platform Resolution

Each artifact's `build` method uses `context.get_system()` to determine the current platform and selects the appropriate:
- Download URL or URL suffix
- Build flags and configure options
- Installation procedure

### Lima VM for Cross-Platform Linux Builds

The project includes Lima VM support for building Linux artifacts on macOS hosts:
- `lima.yaml`: VM template configuration.
- `makefile`: Targets for managing Lima VMs (`lima`, `lima-clean`, `lima-sync`, `lima-vorpal`, `lima-vorpal-start`).
- `script/lima.sh`: Provisions the Lima VM with build dependencies (bubblewrap, build-essential, Docker, Vorpal CLI).

## CI/CD Architecture

### GitHub Actions Workflow (`vorpal.yaml`)

Three-job pipeline:

1. **`build-changes`**: Runs `detect-changed-artifacts.sh` to identify which artifacts were modified between commits. Outputs a JSON array of artifact names and a `has_changes` boolean.

2. **`build-dev`**: Builds the development environment (`vorpal build 'dev'`) on all four runner types (`macos-latest`, `macos-latest-large`, `ubuntu-latest`, `ubuntu-latest-arm64`). Uses `ALT-F4-LLC/setup-vorpal-action` to install Vorpal with S3 registry backend. Uploads `Vorpal.lock` as a build artifact.

3. **`build`**: Conditionally runs only if `has_changes == true`. Uses a matrix strategy to build each changed artifact on all four runners. Each runner executes `vorpal build '<artifact>'`.

### Change Detection (`script/detect-changed-artifacts.sh`)

Dynamically discovers artifacts by scanning `src/artifact/*.rs` (excluding `file.rs`). Compares git diffs between commits to identify which artifact files changed. Converts Rust filenames to artifact names (`_` to `-`). Only detects direct artifact file changes -- modifications to shared files (`lib.rs`, `vorpal.rs`, `Cargo.lock`) do not trigger a full rebuild.

### Automated Dependency Updates

Renovate Bot is configured with `config:recommended` to automatically propose dependency updates via pull requests.

## Build System Integration

### How Artifacts Are Built

1. This project is itself a Vorpal project (defined by `Vorpal.toml`).
2. Running `vorpal build <artifact-name>` compiles this Rust project, executes the resulting binary, which registers all artifact definitions with the Vorpal runtime.
3. The Vorpal runtime identifies the requested artifact by name or alias, resolves its dependency graph, fetches sources (verified against `Vorpal.lock` digests), and executes build steps in sandboxed environments.
4. Build steps are shell scripts executed in an environment where `$VORPAL_OUTPUT` points to the artifact's output directory and `$VORPAL_ARTIFACT_<hash>` environment variables point to dependency artifacts.
5. Built artifacts are cached in the S3 registry, keyed by content hash.

### Source Integrity

The `Vorpal.lock` file provides deterministic, verifiable builds by pinning SHA-256 digests for every source URL per platform. The lockfile format is:

```toml
[[sources]]
name = "artifact-name"
path = "https://download-url"
includes = []
excludes = []
digest = "sha256-hex"
platform = "aarch64-darwin"
```

## Key Architectural Decisions

1. **Single binary, all artifacts registered at startup**: Every artifact is registered regardless of which one is being built. This simplifies the architecture but means adding a new artifact requires modifying both `src/artifact.rs` (module declaration) and `src/vorpal.rs` (build registration).

2. **Inline dependency resolution**: Artifacts with dependencies build their dependencies inline if not explicitly provided. This means running `gpg` alone will also build all five of its dependencies. The Vorpal runtime likely caches results to avoid redundant builds.

3. **Shell-based build steps**: Build logic lives in bash scripts embedded in Rust string literals via `indoc::formatdoc!`. This keeps the build recipes portable but means build logic is not type-checked by Rust.

4. **No centralized version catalog**: Each artifact hardcodes its version, source URL pattern, and platform mappings. There is no shared version manifest or central configuration for artifact versions.

5. **Flat module structure**: All artifacts are siblings in `src/artifact/` with no sub-categorization (e.g., no separation between CLI tools, libraries, development tools). The `artifact.rs` module file is a flat list of `pub mod` declarations.

6. **Content-addressable artifact references**: Artifact `build()` methods return content hash strings. Dependencies reference each other by these hashes, enabling the Vorpal runtime to provide deterministic, cached builds.

## Gaps and Observations

- **No tests in Rust**: There are no unit or integration tests for the artifact definitions themselves. The only tests are bash-based tests for the `detect-changed-artifacts.sh` script.
- **No documentation per artifact**: Individual artifact modules have no doc comments explaining versioning policy, known issues, or platform-specific caveats.
- **Sequential artifact registration**: All artifacts are registered sequentially in `main()`. While this is the registration phase (not the build phase), it could theoretically be parallelized since registrations are independent.
- **Dependency deduplication is implicit**: When multiple artifacts share a dependency (e.g., `tmux` and `zsh` both depend on `ncurses`), deduplication relies on the Vorpal runtime's caching behavior. There is no explicit mechanism in this codebase to prevent redundant builds.
- **`file.rs` is a utility, not an artifact**: It provides a generic `File` builder for creating simple file artifacts from string content. It is excluded from the artifact discovery script and is not registered in `vorpal.rs`.
- **Source lock entries for artifacts not in `src/artifact/`**: The `Vorpal.lock` contains entries for artifacts like `bash`, `tar`, `texinfo`, `unzip`, `util-linux`, `xz` that do not have corresponding `src/artifact/*.rs` files. These are likely SDK-internal bootstrap artifacts needed by the Vorpal build environment.
