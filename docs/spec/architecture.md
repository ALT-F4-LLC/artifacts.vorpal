# Architecture Specification

> Living document describing the architecture of `vorpal-artifacts` as it exists in the codebase.
> Last updated: 2026-02-21

## System Overview

`vorpal-artifacts` is a Rust project that defines and builds software artifacts (packages and tools) for the **Vorpal** build system. It is not a general-purpose package manager -- it is a declarative artifact catalog where each artifact is a self-contained Rust module that describes how to download, build, and install a specific tool or library across four target platforms.

The project produces a single binary (`vorpal`) that, when executed by the Vorpal build system, registers all artifact definitions and a development environment, then delegates to the Vorpal SDK runtime for actual build orchestration.

## Target Platforms

All artifacts target four platforms, represented by the `ArtifactSystem` enum from the Vorpal SDK:

| Enum Variant     | Architecture | OS     |
|------------------|-------------|--------|
| `Aarch64Darwin`  | ARM64       | macOS  |
| `Aarch64Linux`   | ARM64       | Linux  |
| `X8664Darwin`    | x86_64      | macOS  |
| `X8664Linux`     | x86_64      | Linux  |

These are exported as `DEFAULT_SYSTEMS` in `src/lib.rs` (line 13-14).

## Component Architecture

### Entry Points

The project has two logical entry points:

1. **`src/vorpal.rs`** -- The binary entry point. Contains `#[tokio::main] async fn main()` which:
   - Obtains a `ConfigContext` from the Vorpal SDK via `get_context().await`
   - Builds all 46 artifacts in dependency order
   - Creates the `dev` `ProjectEnvironment`
   - Calls `context.run().await` to hand off to the Vorpal runtime

2. **`src/lib.rs`** -- The library crate root. Exports:
   - `pub mod artifact` -- the artifact module tree
   - `DEFAULT_SYSTEMS` -- the four-platform constant array
   - `ProjectEnvironment` -- a struct that bundles Lima, Protoc, and Rust toolchain into a development environment

### Module Structure

```
src/
  vorpal.rs           # Binary entry point (main)
  lib.rs              # Library root: ProjectEnvironment, DEFAULT_SYSTEMS
  artifact.rs         # Module declarations (47 pub mod statements)
  artifact/
    argocd.rs          # Simple: pre-built binary download
    awscli2.rs         # Simple: platform-specific installer
    bat.rs             # Simple: pre-built binary download
    beads.rs           # Simple: pre-built binary download
    bottom.rs          # Simple: pre-built binary download
    crane.rs           # Simple: pre-built binary download
    cue.rs             # Simple: pre-built binary download
    direnv.rs          # Simple: pre-built binary download
    doppler.rs         # Simple: pre-built binary download
    fd.rs              # Simple: pre-built binary download
    file.rs            # Utility: generic file artifact (not a tool)
    fluxcd.rs          # Simple: pre-built binary download
    golangci_lint.rs   # Simple: pre-built binary download
    gpg.rs             # Complex: source build, 5 library dependencies
    helm.rs            # Simple: pre-built binary download
    jj.rs              # Simple: pre-built binary download
    jq.rs              # Simple: pre-built binary download
    just.rs            # Simple: pre-built binary download
    k9s.rs             # Simple: pre-built binary download
    kn.rs              # Simple: pre-built binary download
    kubectl.rs         # Simple: pre-built binary download
    kubeseal.rs        # Simple: pre-built binary download
    lazygit.rs         # Simple: pre-built binary download
    libassuan.rs       # Library: source build, depends on libgpg_error
    libevent.rs        # Library: source build, no dependencies
    libgcrypt.rs       # Library: source build, depends on libgpg_error
    libgpg_error.rs    # Library: source build, no dependencies
    libksba.rs         # Library: source build, depends on libgpg_error
    lima.rs            # Simple: pre-built binary download
    ncurses.rs         # Library: source build, no dependencies
    neovim.rs          # Simple: pre-built binary download
    nginx.rs           # Source build: configure/make/install
    nnn.rs             # Complex: source build, 3 dependencies
    npth.rs            # Library: source build, no dependencies
    openapi_generator_cli.rs  # Wrapper: JAR with shell script, depends on openjdk
    openjdk.rs         # Simple: pre-built JDK download
    pkg_config.rs      # Source build: autotools
    readline.rs        # Library: source build, depends on ncurses
    ripgrep.rs         # Simple: pre-built binary download
    skopeo.rs          # Language build: uses SDK Go helper
    sqlite3.rs         # Source build: configure/make/install
    starship.rs        # Simple: pre-built binary download
    terraform.rs       # Simple: pre-built binary download
    tmux.rs            # Complex: source build, 2 dependencies
    umoci.rs           # Simple: pre-built binary download
    yq.rs              # Simple: pre-built binary download
    zsh.rs             # Source build: depends on ncurses
```

### Artifact Categories

Artifacts fall into five categories based on their build strategy:

| Category | Count | Pattern | Example |
|----------|-------|---------|---------|
| **Pre-built binary download** | ~27 | Download platform-specific binary, copy to `$VORPAL_OUTPUT/bin` | `jj.rs`, `argocd.rs` |
| **Source build (no deps)** | ~8 | Download tarball, `configure && make && make install` | `ncurses.rs`, `nginx.rs`, `sqlite3.rs` |
| **Source build (with deps)** | ~7 | Source build with dependency injection via builder methods | `gpg.rs`, `tmux.rs`, `nnn.rs` |
| **Language-specific build** | 1 | Uses SDK language helpers (e.g., `Go::new()`) | `skopeo.rs` |
| **Utility** | 2 | Non-tool artifacts: `file.rs` (generic file), `openapi_generator_cli.rs` (JAR wrapper) | `file.rs` |

## Design Patterns

### Artifact Builder Pattern

Every artifact follows a consistent builder pattern. This is the single most important architectural pattern in the project.

**Simple artifact (no dependencies):**

```rust
#[derive(Default)]
pub struct ArtifactName;

impl ArtifactName {
    pub fn new() -> Self { Self }

    pub async fn build(self, context: &mut ConfigContext) -> Result<String> {
        // 1. Define name, version
        // 2. Match context.get_system() for platform-specific URLs
        // 3. Create ArtifactSource
        // 4. Write shell build script
        // 5. Create step via step::shell()
        // 6. Return Artifact::new(name, steps, systems).build(context).await
    }
}
```

**Complex artifact (with dependencies):**

```rust
#[derive(Default)]
pub struct ArtifactName<'a> {
    dep_a: Option<&'a str>,
    dep_b: Option<&'a str>,
}

impl<'a> ArtifactName<'a> {
    pub fn new() -> Self { Self { dep_a: None, dep_b: None } }

    pub fn with_dep_a(mut self, dep_a: &'a str) -> Self {
        self.dep_a = Some(dep_a);
        self
    }

    pub async fn build(self, context: &mut ConfigContext) -> Result<String> {
        // Resolve dependency: use injected value or build inline
        let dep_a = match self.dep_a {
            Some(val) => val,
            None => &DepA::new().build(context).await?,
        };
        // ... rest of build logic using get_env_key(&dep_a) for paths
    }
}
```

Key aspects of the pattern:
- Dependencies are optional `Option<&'a str>` fields holding artifact key strings
- `with_*()` builder methods inject pre-built dependency keys
- If a dependency is not injected, the artifact builds it inline (fallback)
- `get_env_key()` converts artifact keys to environment variable paths for use in shell scripts
- Dependencies are passed to `step::shell()` as the first argument (artifact dependencies list)
- Build scripts use `$VORPAL_OUTPUT` as the installation prefix
- Sources are extracted to `./source/{name}/` by the Vorpal runtime

### Dependency Injection via Artifact Keys

Artifact keys are strings returned by `.build()` that uniquely identify a built artifact. When an artifact depends on another:

1. The dependency is built first, returning its key string
2. The key is passed via `with_*()` to the dependent artifact
3. Inside the build script, `get_env_key(&key)` resolves the key to a filesystem path
4. The path is used in `CPPFLAGS`, `LDFLAGS`, `PATH`, `PKG_CONFIG_PATH`, etc.

This pattern allows the same dependency to be shared across multiple consumers without rebuilding.

### Build Script Convention

All build scripts follow a shell script convention:
- Output goes to `$VORPAL_OUTPUT` (set by the Vorpal runtime)
- Binaries are placed in `$VORPAL_OUTPUT/bin/`
- Source archives are auto-extracted to `./source/{artifact_name}/`
- Scripts use `pushd` to navigate into source directories
- Source builds use standard autotools: `./configure --prefix="$VORPAL_OUTPUT" && make && make install`

## Dependency Graph

### Build-Time Dependency Tree

```
ncurses (standalone)
  |-- readline
  |     |-- nnn (also depends on ncurses, pkg_config)
  |-- tmux (also depends on libevent)
  |-- zsh
  |-- nnn

libevent (standalone)
  |-- tmux

pkg_config (standalone)
  |-- nnn

libgpg_error (standalone)
  |-- libassuan
  |-- libgcrypt
  |-- libksba
  |-- gpg (depends on all five: libassuan, libgcrypt, libgpg_error, libksba, npth)

npth (standalone)
  |-- gpg

openjdk (standalone)
  |-- openapi_generator_cli
```

### Build Order in vorpal.rs

`src/vorpal.rs` enforces dependency order by building artifacts sequentially. Artifacts with dependencies are built after their dependencies. The order is:

1. **Foundation libraries** (lines 24-49): `libevent`, `libgpg_error`, `libassuan`, `libgcrypt`, `libksba`, `ncurses`, `npth`, `openjdk`, `pkg_config`, `readline`
2. **Independent artifacts** (lines 56-129): All simple pre-built binary downloads, plus `gpg`, `nginx`
3. **Dependent artifacts** (lines 113-145): `nnn`, `openapi_generator_cli`, `tmux`, `zsh`
4. **Development environment** (lines 149-151): `ProjectEnvironment::new("dev", ...)`
5. **Runtime handoff** (line 153): `context.run().await`

### Development Environment (ProjectEnvironment)

`ProjectEnvironment` in `src/lib.rs` bundles three SDK-provided tools into a development environment:

- **Lima** -- Linux VM manager (for macOS-to-Linux development)
- **Protoc** -- Protocol Buffers compiler (from `vorpal_sdk::artifact::protoc`)
- **RustToolchain** -- Rust compiler and toolchain (from `vorpal_sdk::artifact::rust_toolchain`)

The environment configures `PATH`, `RUSTUP_HOME`, and `RUSTUP_TOOLCHAIN` environment variables for the Rust toolchain.

## External Dependencies

### Rust Crate Dependencies

From `Cargo.toml`:

| Crate | Version | Purpose |
|-------|---------|---------|
| `anyhow` | 1 | Error handling (`Result<T>` type alias) |
| `indoc` | 2 | Indented multi-line string literals (`formatdoc!` macro) |
| `tokio` | 1 (rt-multi-thread) | Async runtime for the binary entry point |
| `vorpal-sdk` | git (main branch) | Core Vorpal build system SDK |

### Vorpal SDK

The project depends on `vorpal-sdk` from the `ALT-F4-LLC/vorpal` repository (main branch, git dependency). The SDK provides:

- `ConfigContext` -- build context management, platform detection (`get_system()`)
- `get_context()` -- initialization of the config context
- `ArtifactSystem` enum -- platform target identifiers
- `Artifact` -- artifact definition builder with `.with_aliases()`, `.with_sources()`
- `ArtifactSource` -- source file/URL declaration
- `step::shell()` -- shell-based build step creation
- `get_env_key()` -- artifact key to environment path resolution
- `ProjectEnvironment` -- SDK-level environment bundling
- Language helpers: `language::go::Go` for Go source builds
- Built-in artifacts: `Protoc`, `RustToolchain`

The SDK is pinned to the `main` branch via git, meaning SDK changes can break this project without version bump. The `Cargo.lock` file pins the exact commit.

## CI/CD Architecture

### GitHub Actions Workflow (`.github/workflows/vorpal.yaml`)

The CI pipeline has three jobs:

1. **`build-changes`** -- Runs on `ubuntu-latest`. Uses `script/detect-changed-artifacts.sh` to determine which artifacts changed between commits. Outputs a JSON array of artifact names and a `has_changes` boolean.

2. **`build-dev`** -- Runs on a 4-runner matrix (macOS ARM, macOS x86, Ubuntu x86, Ubuntu ARM). Builds the `dev` environment using `vorpal build 'dev'`. Uploads the `Vorpal.lock` file as a GitHub artifact.

3. **`build`** -- Conditional on `has_changes`. Uses a matrix of changed artifacts x 4 runners. Each cell runs `vorpal build '<artifact-name>'`. This means only changed artifacts are built, and each is built on all four platforms.

The workflow uses `ALT-F4-LLC/setup-vorpal-action@main` to install the Vorpal CLI with an S3-backed registry (`altf4llc-vorpal-registry`).

### Artifact Detection Script

`script/detect-changed-artifacts.sh` provides CI-level change detection:

- **Discovery**: Scans `src/artifact/*.rs` files, converts `snake_case.rs` filenames to `kebab-case` artifact names
- **Exclusions**: `file.rs` is excluded (it is a utility, not a buildable artifact)
- **Diff detection**: Uses `git diff --name-only --diff-filter=d` to find changed files between two commits, filtering to only `src/artifact/*.rs` changes
- **Output**: JSON array of changed artifact names, or `[]` if none changed
- **Key design decision**: Only direct artifact file changes trigger builds. Changes to `src/vorpal.rs`, `Cargo.lock`, `Cargo.toml`, or other infrastructure files do NOT trigger artifact rebuilds. This was an intentional simplification -- a previous `CORE_FILES` mechanism that triggered full rebuilds on infrastructure changes was removed.

### Lima VM Support

For building Linux artifacts on macOS, the project includes Lima VM integration:

- **`lima.yaml`** -- Lima VM template using Debian 13 (Trixie) cloud images for both aarch64 and x86_64
- **`script/lima.sh`** -- VM provisioning script that installs build dependencies (bubblewrap, build-essential, docker, vorpal, etc.) and configures AppArmor for bubblewrap
- **`makefile`** -- Convenience targets for `lima`, `lima-clean`, `lima-sync`, `lima-vorpal`, `lima-vorpal-start`

## Build System Integration

### Vorpal.toml

```toml
language = "rust"

[source]
includes = ["src", "Cargo.toml", "Cargo.lock"]
```

This tells the Vorpal build system that this is a Rust project and to include only the `src/` directory and Cargo manifest files as build inputs.

### Vorpal.lock

The lock file (`Vorpal.lock`) tracks source digests (SHA-256 hashes) for every artifact source URL across all platforms. Each entry includes:
- `name` -- artifact name
- `path` -- source URL
- `digest` -- SHA-256 hash of the downloaded source
- `platform` -- target platform string (e.g., `x86_64-darwin`)

This provides reproducible builds by pinning exact source content.

## Architectural Gaps and Observations

### No Unit Tests
There are no Rust unit tests in the project. The only tests are bash-based regression tests for the `detect-changed-artifacts.sh` script.

### No Artifact Versioning Contract
Artifact versions are hardcoded strings inside each module. There is no centralized version manifest, no automated version bumping mechanism, and no way to query what version of an artifact is defined without reading the source code. Renovate is configured (`.github/renovate.json`) but operates on Cargo dependencies, not artifact versions.

### SDK Dependency is Git-Pinned
The `vorpal-sdk` dependency uses a git branch reference (`main`) rather than a versioned crate release. This means the project depends on HEAD of the SDK's main branch at whatever commit Cargo.lock pins, and SDK API changes can break this project.

### No Artifact Validation
There is no mechanism to validate that a defined artifact actually builds successfully beyond running it through CI. There are no smoke tests, health checks, or artifact integrity verification steps defined within the artifact modules themselves.

### Build Script is Untyped
Build scripts are shell strings constructed via `formatdoc!`. There is no type safety or validation of the shell scripts at compile time. Errors in build scripts are only caught at Vorpal build runtime.

### Linear Dependency Resolution
`src/vorpal.rs` resolves dependencies through sequential Rust code ordering. There is no declarative dependency graph, no parallel build orchestration at the artifact registration level, and no cycle detection. The correctness of the build order depends entirely on the developer ordering the calls correctly in `main()`.

### file.rs is Declared but Not Used in vorpal.rs
The `file` module is declared in `src/artifact.rs` but is not imported or used in `src/vorpal.rs`. It exists as a utility for other consumers of the library crate, not as a standalone artifact. The CI script correctly excludes it from artifact discovery.

### No Error Recovery
All artifact builds use `?` propagation with `anyhow::Result`. A failure in any single artifact build aborts the entire `main()` function. There is no partial build support or continue-on-error mechanism.
