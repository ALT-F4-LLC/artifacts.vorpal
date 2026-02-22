# Performance Specification

This document describes the performance characteristics, bottlenecks, optimization strategies, and
scaling considerations for the `vorpal-artifacts` project as they exist today.

---

## 1. Project Nature and Performance Profile

`vorpal-artifacts` is a **build-time artifact definition system**, not a runtime service. It defines
~55 software artifacts (binaries, libraries, and tools) as Rust structs that produce build
instructions for the Vorpal build system. The performance-critical operations are:

- **Configuration phase**: Registering all artifact definitions with the Vorpal SDK context
- **Build execution phase**: Downloading sources, compiling, and packaging artifacts (handled by
  the Vorpal SDK/runtime, not this codebase directly)
- **CI pipeline throughput**: Building changed artifacts across 4 platform runners

The codebase itself is a Rust binary (`src/vorpal.rs`) that runs an async Tokio main function.
It does not serve HTTP traffic, manage databases, or handle user-facing load. Performance
characteristics are dominated by build orchestration and I/O.

---

## 2. Build-Time Performance Characteristics

### 2.1 Sequential Artifact Registration

The main entry point (`src/vorpal.rs:21-88`) registers all ~55 artifacts **sequentially** using
`await` on each `.build(context)` call. Each artifact registration is awaited one at a time:

```rust
Argocd::new().build(context).await?;
Awscli2::new().build(context).await?;
Bat::new().build(context).await?;
// ... ~52 more sequential calls
```

This is the **configuration phase** only -- it registers artifact definitions with the Vorpal SDK
context, not the actual build execution. The Vorpal SDK's `context.run().await` at the end handles
actual build orchestration. Whether the SDK itself parallelizes builds is outside this codebase's
control.

**Current state**: No parallel registration of independent artifacts. All 55 artifacts are awaited
sequentially even when they have no dependencies on each other.

**Impact**: For the configuration phase, the overhead is likely minimal (network-free, in-memory
struct registration). The sequential pattern matters more if the SDK performs any I/O during
`.build(context)` registration (e.g., source digest verification).

### 2.2 Dependency Chains and Build Graph Depth

Several artifacts form dependency chains that increase total build time:

| Chain | Depth | Components |
|---|---|---|
| **ttyd** | 5 | cmake -> mbedtls, libuv, json-c, libwebsockets -> ttyd |
| **gpg** | 3 | libgpg-error -> libassuan, libgcrypt, libksba, npth -> gpg |
| **nnn** | 3 | ncurses -> readline, pkg-config -> nnn |
| **tmux** | 2 | ncurses, libevent -> tmux |
| **zsh** | 2 | ncurses -> zsh |

The deepest chain is **ttyd** with 5 levels of transitive dependencies. The `libwebsockets`
artifact itself depends on cmake, libuv, and mbedtls. These chains are the **critical path** for
full rebuilds.

### 2.3 Dependency Deduplication via Builder Pattern

Artifacts with dependencies use an optional builder pattern (e.g., `.with_ncurses(ncurses)`) that
allows callers to inject pre-built dependency references. When dependencies are not injected,
artifacts rebuild them internally:

```rust
// In gpg.rs -- if libgpg_error not provided, it gets built fresh
let libgpg_error = match self.libgpg_error {
    Some(val) => val,
    None => &libgpg_error::LibgpgError::new().build(context).await?,
};
```

The current `src/vorpal.rs` does **not** use this injection pattern. Each artifact is built with
`::new()` defaults, meaning shared dependencies (like `ncurses`, `cmake`) may be registered
multiple times. Whether the Vorpal SDK deduplicates identical artifact registrations is not
visible from this codebase.

**Gap**: No explicit dependency sharing in the top-level orchestration. Artifacts like `ncurses`
are depended upon by tmux, nnn, zsh, and readline, but each path potentially re-registers it.

---

## 3. Artifact Build Strategies and Their Performance Costs

### 3.1 Pre-built Binary Downloads (Fast Path)

Approximately 30 of the ~55 artifacts download pre-built binaries. These involve:
- A single HTTP download
- Extract/copy to `$VORPAL_OUTPUT`
- Set executable permissions

Examples: argocd, awscli2, bat, beads, bottom, crane, cue, direnv, doppler, fd, fluxcd,
golangci-lint, helm, jj, jq, just, k9s, kn, kubectl, kubeseal, lazygit, lima, neovim,
openjdk, ripgrep, starship, terraform, vhs, yq.

**Performance profile**: Network-bound. Build time is dominated by download speed and archive
extraction. Typically seconds to low minutes per artifact.

### 3.2 Source Compilation (Slow Path)

Approximately 20 artifacts compile from source using `./configure && make` or CMake patterns.
These are the most time-intensive:

**Autotools-based** (configure/make): ffmpeg, gpg, libevent, libgpg-error, libassuan, libgcrypt,
libksba, ncurses, nginx, nnn, npth, pkg-config, readline, sqlite3, tmux, zsh, zlib.

**CMake-based**: json-c, libuv, libwebsockets, mbedtls, ttyd (on macOS).

**Performance profile**: CPU-bound during compilation, I/O-bound during configure steps.

### 3.3 Parallel Make Usage

Only 6 of the ~20 source-compiled artifacts use `make -j` for parallel compilation:

| Artifact | Parallel Make |
|---|---|
| ffmpeg | `make -j$(nproc 2>/dev/null \|\| sysctl -n hw.ncpu)` |
| json-c | `make -j$(nproc 2>/dev/null \|\| sysctl -n hw.ncpu) install` |
| libuv | `make -j$(nproc 2>/dev/null \|\| sysctl -n hw.ncpu) install` |
| libwebsockets | `make -j$(nproc 2>/dev/null \|\| sysctl -n hw.ncpu) install` |
| mbedtls | `make -j$(nproc 2>/dev/null \|\| sysctl -n hw.ncpu) install` |
| zlib | `make -j$(nproc 2>/dev/null \|\| sysctl -n hw.ncpu) install` |

**Gap**: The remaining ~14 source-compiled artifacts use single-threaded `make` without `-j`.
This includes nginx, gpg, ncurses, readline, libevent, sqlite3, tmux, nnn, zsh, npth,
libgpg-error, libassuan, libgcrypt, libksba, and pkg-config. Adding `-j$(nproc)` to these
would reduce their build times on multi-core machines.

### 3.4 Platform-Conditional Build Strategies

Some artifacts (ttyd, awscli2) use different strategies per platform. For example, ttyd downloads
pre-built binaries on Linux but compiles from source on macOS. This means macOS CI runners
experience longer build times for these artifacts.

---

## 4. Caching Strategy

### 4.1 Vorpal SDK Content-Addressable Caching

The Vorpal build system uses content-addressable artifact storage. The `Vorpal.lock` file
(2,689 lines) records source digests per platform:

```toml
[[sources]]
name = "argocd"
path = "https://github.com/argoproj/argo-cd/releases/download/v3.2.3/argocd-darwin-arm64"
digest = "bd4b2683005fe932123093d357e8bb2c38048e0371cd27156ebb8a83de8a9bd5"
platform = "aarch64-darwin"
```

Each source entry has a SHA-256 digest and platform specifier. This enables the Vorpal runtime
to skip re-downloading unchanged sources. The caching and rebuild-avoidance logic lives in the
Vorpal SDK/runtime, not in this codebase.

### 4.2 S3 Registry Backend

The CI workflow (`vorpal.yaml`) configures an S3 registry backend:

```yaml
registry-backend: s3
registry-backend-s3-bucket: altf4llc-vorpal-registry
```

Built artifacts are stored in S3 (`altf4llc-vorpal-registry`), enabling cross-build caching.
If an artifact with the same content hash already exists in the registry, the Vorpal runtime
can skip rebuilding it. This is the primary caching mechanism.

### 4.3 No Application-Level Caching

This codebase contains no application-level caching (no in-memory caches, no disk caches, no
Redis/Memcached). All caching is delegated to the Vorpal SDK and its S3 registry backend.

---

## 5. CI/CD Pipeline Performance

### 5.1 Matrix Build Strategy

The CI pipeline (`vorpal.yaml`) uses a matrix strategy across 4 runners:

| Runner | Architecture | OS |
|---|---|---|
| `macos-latest` | ARM64 | macOS |
| `macos-latest-large` | x86_64 | macOS |
| `ubuntu-latest` | x86_64 | Linux |
| `ubuntu-latest-arm64` | ARM64 | Linux |

Builds run in parallel across all 4 runners for both the `build-dev` and `build` jobs.

### 5.2 Changed Artifact Detection

The `detect-changed-artifacts.sh` script implements incremental builds by comparing git diffs
between commits:

- Scans `src/artifact/*.rs` for changed files
- Maps filename changes to artifact names (underscore to hyphen conversion)
- Outputs a JSON array of changed artifacts for the CI matrix

**Current limitation**: The detection is file-level only. It detects that `src/artifact/gpg.rs`
changed but does **not** transitively detect that artifacts depending on gpg's dependencies
should also rebuild. For example, changing `ncurses.rs` would only trigger a rebuild of `ncurses`,
not `tmux`, `nnn`, `zsh`, or `readline` which depend on it.

### 5.3 Pipeline Structure

```
build-changes (detect changed artifacts)
     |
build-dev (build "dev" environment on all 4 runners, in parallel)
     |
build (build each changed artifact x each runner, matrix, fail-fast: false)
```

The `build` job uses `fail-fast: false`, so a failure on one runner/artifact combination does not
cancel other matrix entries. This maximizes build throughput at the cost of consuming runner
minutes on potentially broken builds.

### 5.4 Lima VM Resources

Local cross-platform testing uses Lima VMs with configurable resources:

```makefile
LIMA_CPUS := 8
LIMA_DISK := 100    # GB
LIMA_MEMORY := 8    # GB
```

These defaults provide reasonable resources for from-source compilation. The rsync sync command
excludes `.git` and `target` directories to minimize transfer overhead.

---

## 6. Rust Compilation Performance

### 6.1 Cargo Build Configuration

The `Cargo.toml` has no custom build profiles. There are no `[profile.release]` or
`[profile.dev]` overrides for:

- `opt-level`
- `lto` (link-time optimization)
- `codegen-units`
- `strip`
- `debug`

The project compiles with default Cargo profile settings.

### 6.2 Dependencies

The project has 4 direct dependencies:

| Dependency | Role |
|---|---|
| `anyhow` | Error handling |
| `indoc` | Multi-line string formatting |
| `tokio` (rt-multi-thread) | Async runtime |
| `vorpal-sdk` (git dep) | Build system SDK |

The `Cargo.lock` is committed, ensuring reproducible builds. The `tokio` dependency uses the
`rt-multi-thread` feature, enabling the multi-threaded async runtime even though the current
code uses a single sequential task flow.

---

## 7. Known Bottlenecks

### 7.1 Source Compilation Without Parallel Make

14 source-compiled artifacts use `make` without `-j` flags. On an 8-core machine, this means
compilation uses ~12.5% of available CPU capacity. The most impactful targets for adding parallel
make are the ones most frequently rebuilt or with the longest compile times: nginx, gpg, ncurses.

### 7.2 Sequential Top-Level Registration

All 55 artifacts are registered sequentially. While this may not matter if registration is
purely in-memory, it becomes a bottleneck if the SDK performs I/O during registration (e.g.,
verifying source digests against the lock file).

### 7.3 Missing Transitive Dependency Tracking in CI

The `detect-changed-artifacts.sh` script does not track transitive dependencies. A change to a
foundational library like `ncurses` will not trigger rebuilds of dependent artifacts (tmux, nnn,
zsh, readline, nnn) in CI. This is a correctness gap rather than a performance gap, but it
means that full rebuilds are occasionally needed, which is a performance concern.

### 7.4 Duplicate Dependency Registration

Without explicit dependency sharing in `src/vorpal.rs`, shared libraries like `ncurses` and
`cmake` may be registered multiple times through different dependency paths. Whether this results
in duplicate work depends on the Vorpal SDK's deduplication behavior.

### 7.5 Platform-Specific Build Asymmetry

macOS builds are slower for some artifacts (e.g., ttyd) because they compile from source while
Linux uses pre-built binaries. This creates asymmetric CI pipeline durations.

---

## 8. Benchmarking

### 8.1 Current State

There is **no benchmarking infrastructure** in this project. There are:

- No Rust benchmarks (`#[bench]` or criterion)
- No build time tracking or reporting
- No artifact size monitoring
- No CI timing dashboards

### 8.2 Relevant Metrics to Track

If benchmarking were added, the most useful metrics would be:

- **Total CI pipeline duration** per platform
- **Individual artifact build time** per platform
- **Source download time** (network latency to upstream mirrors)
- **Artifact output size** (to catch regressions from accidental debug builds)
- **Lock file drift** (frequency of Vorpal.lock changes, indicating upstream version churn)

---

## 9. Rootfs Slimming Performance

The `script/linux-vorpal-slim.sh` script addresses artifact **output size** performance by
reducing a Linux rootfs from ~2.9GB to ~600-700MB. It removes:

- GCC compiler infrastructure (~1.2GB)
- Python/Perl runtimes (~338MB)
- Static libraries (~252MB)
- Locale data (~222MB + 76MB translations)
- Documentation (~51MB)
- i18n encodings (~43MB)
- Headers (~34MB)
- Sanitizer libraries (~31MB)

The script supports dry-run mode, section-selective execution, and optional aggressive mode
(binary stripping). This is a post-build optimization step, not a build-time performance tool.

---

## 10. Scaling Considerations

### 10.1 Artifact Count Growth

The project currently defines ~55 artifacts. As more artifacts are added:

- Sequential registration time grows linearly
- CI matrix size grows (artifacts x runners), increasing total runner minutes
- Dependency graph complexity increases, making transitive rebuild detection more important
- The `detect-changed-artifacts.sh` script's file-level detection becomes increasingly
  insufficient

### 10.2 Cross-Platform Coverage

All 4 platform targets (aarch64-darwin, aarch64-linux, x86_64-darwin, x86_64-linux) are built
for every artifact. The CI matrix is `artifacts x 4 runners`. Adding a 5th platform would
increase CI cost by 25%.

### 10.3 S3 Registry Scaling

The S3 registry stores all built artifacts. Storage costs scale with:
- Number of artifacts x number of platforms x number of versions retained
- No visible garbage collection or version retention policy in this codebase

---

## 11. Gaps and Recommendations Summary

| Area | Current State | Gap |
|---|---|---|
| Parallel make | 6 of 20 source builds | 14 builds use single-threaded make |
| Artifact registration | Sequential | Could parallelize independent artifacts |
| Dependency sharing | Builder pattern exists but unused at top level | Shared deps may be registered multiple times |
| Transitive CI rebuilds | File-level only | Does not rebuild dependents |
| Benchmarking | None | No build time or artifact size tracking |
| Cargo profiles | Default | No release optimizations configured |
| Build caching | Vorpal SDK + S3 | No visibility into cache hit rates |
| Rootfs slimming | Script exists | Not integrated into CI pipeline |

This document reflects what exists in the codebase as of the current state. Recommendations are
identified as gaps, not prescriptions -- the right time to address each depends on whether it
is actually causing problems in practice.
