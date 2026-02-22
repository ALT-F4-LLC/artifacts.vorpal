# Review Strategy

This document defines the code review strategy for the `artifacts.vorpal` project based on the
actual codebase structure, change patterns, and risk profile observed in the repository.

---

## Project Review Profile

This is a **Vorpal artifact definitions** repository -- a Rust project that declares build
definitions for ~55 software packages using the Vorpal SDK. The codebase is primarily authored
by a single contributor (64 of 67 commits) with automated dependency updates from Renovate.

**Key characteristics that shape review strategy:**

- The project is a build configuration layer, not a runtime application
- Changes are overwhelmingly artifact additions or version bumps
- The Vorpal SDK (external dependency) handles all build execution
- Cross-platform correctness (4 targets: aarch64-darwin, aarch64-linux, x86_64-darwin, x86_64-linux) is the primary concern
- There is no application logic, no user-facing API, and no persistent state

---

## Review Dimensions by Priority

Review effort should be allocated according to this priority ordering, which reflects the
actual risk profile of this project.

### 1. Cross-Platform Correctness (Highest Priority)

The single most important review dimension. Every artifact must build on all four target
systems. Common failure modes to watch for:

- **System-specific match arms**: Verify all four `ArtifactSystem` variants are handled
  (`Aarch64Darwin`, `Aarch64Linux`, `X8664Darwin`, `X8664Linux`). Missing a variant produces
  a runtime error.
- **Download URL correctness**: Platform-specific download URLs must use the correct
  architecture naming conventions for each upstream project (e.g., `arm64` vs `aarch64`,
  `amd64` vs `x86_64`, `darwin` vs `macos`).
- **Build flags**: Compiler flags, configure options, and cmake variables may need to differ
  across platforms. Example: `--disable-x86asm` for ffmpeg, `nproc` vs `sysctl -n hw.ncpu`
  for parallel builds.
- **Library linking**: `LDFLAGS`, `CPPFLAGS`, `rpath` settings, and `PKG_CONFIG_PATH` must
  reference correct paths per platform.

**Review checklist for this dimension:**
- [ ] All four system variants handled (or explicit `anyhow!` error for unsupported)
- [ ] URLs resolve correctly for each platform
- [ ] Build scripts use cross-platform compatible commands
- [ ] `nproc`/`sysctl` fallback pattern used where applicable

### 2. Dependency Graph Integrity

Many artifacts depend on other artifacts in this repository (e.g., `gpg` depends on
`libassuan`, `libgcrypt`, `libgpg_error`, `libksba`, `npth`). Incorrect dependency wiring
is a high-risk failure mode.

**Review checklist for this dimension:**
- [ ] Dependencies are correctly wired via the builder pattern (`with_*` methods)
- [ ] `get_env_key()` is used for referencing dependency paths in build scripts
- [ ] Circular dependencies are not introduced
- [ ] Dependency versions are compatible with the artifact being built
- [ ] Transitive dependencies are properly propagated (e.g., `libgpg_error` flowing through
  to both `libassuan` and `libgcrypt`)

### 3. Build Script Correctness

Build scripts are embedded as Rust string literals (using `formatdoc!`) and executed by the
Vorpal runtime. Errors in these scripts only surface at build time, not at compile time.

**Review checklist for this dimension:**
- [ ] Shell scripts follow `set -euo pipefail` semantics (inherited from Vorpal runtime)
- [ ] `$VORPAL_OUTPUT` is used as the install prefix
- [ ] Source directory paths match the expected archive structure
- [ ] `configure` flags, cmake variables, and make targets are correct
- [ ] Environment variables (`PATH`, `PKG_CONFIG_PATH`, `CPPFLAGS`, `LDFLAGS`) include all
  required dependency paths
- [ ] String interpolation in `formatdoc!` correctly substitutes variables

### 4. Version Management

Version changes are frequent and carry moderate risk.

**Review checklist for this dimension:**
- [ ] Version string is updated in artifact definition
- [ ] Source URL structure hasn't changed between versions (upstream projects sometimes
  change release URL patterns)
- [ ] Aliases include the versioned format (`name:version`)
- [ ] `Vorpal.lock` is updated with correct digests for all platforms
- [ ] Upstream changelog checked for breaking build system changes

### 5. Code Quality and Pattern Consistency

The codebase follows well-established patterns. Deviations should be intentional.

**Review checklist for this dimension:**
- [ ] Struct implements `Default` derive (for simple artifacts) or manual `new()` constructor
- [ ] Builder pattern used consistently for artifacts with dependencies (`with_*` methods
  returning `Self`)
- [ ] `build()` method is `async` and returns `Result<String>`
- [ ] Module declared in `src/artifact.rs` and imported in `src/vorpal.rs`
- [ ] Follows the established naming convention: `snake_case` module name, `PascalCase`
  struct name, hyphenated artifact name

### 6. Security (Lower Priority for This Project)

Security review is lower priority because this project defines build configurations, not
runtime code. However, some concerns remain:

- **Source URL integrity**: Artifacts download from external URLs. Verify URLs point to
  official release channels (GitHub releases, official project domains).
- **Build script injection**: `formatdoc!` string interpolation could theoretically inject
  shell commands if variables contain malicious content, but in practice all interpolated
  values come from hardcoded constants or the Vorpal SDK.
- **Dependency pinning**: Source digests in `Vorpal.lock` provide integrity verification.
  Verify lock file updates match expected changes.

---

## Change Categories and Review Effort

### Trivial Changes (1-2 minutes)

- `Vorpal.lock` updates with no corresponding source changes (lock regeneration)
- `Cargo.lock` updates from Renovate
- Lockfile-only commits (`chore(vorpal): update lock file`)

**Review approach**: Verify the lock update is expected. Approve quickly.

### Small Changes: New Pre-Built Binary Artifact (5-10 minutes)

Artifacts that download pre-built binaries (e.g., `argocd`, `bat`, `kubectl`, `helm`).
These follow a simple pattern: download platform-specific binary, copy to `$VORPAL_OUTPUT/bin`.

**Review approach**: Verify URL correctness for all 4 platforms, correct permissions (`chmod +x`),
and pattern adherence. These are low-risk.

**Reference pattern**: `src/artifact/argocd.rs` (simplest form), `src/artifact/bat.rs`
(with archive extraction).

### Medium Changes: New Source-Build Artifact (15-30 minutes)

Artifacts that compile from source (e.g., `ffmpeg`, `zlib`, `ncurses`). These involve
configure/make/cmake scripts with platform-specific flags.

**Review approach**: Full review of build script correctness, cross-platform flags, and
install paths. Test that configure options are valid for the specified version. Check for
missing dependencies.

**Reference pattern**: `src/artifact/ffmpeg.rs` (simple source build),
`src/artifact/cmake.rs` (platform-specific install paths).

### Large Changes: Artifact with Dependencies (30-45 minutes)

Artifacts with inter-artifact dependencies using the builder pattern (e.g., `gpg`, `ttyd`,
`nnn`, `libwebsockets`). These are the highest-risk changes.

**Review approach**: Full review across all dimensions. Particular attention to dependency
wiring, environment variable propagation, and transitive dependency correctness. Verify
the dependency chain builds correctly on all platforms.

**Reference patterns**: `src/artifact/gpg.rs` (5 dependencies, deepest chain),
`src/artifact/ttyd.rs` (platform-conditional build: pre-built on Linux, source on Darwin),
`src/artifact/libwebsockets.rs` (cmake-based with multiple dependency paths).

### Infrastructure Changes (15-30 minutes)

Changes to CI workflow (`.github/workflows/vorpal.yaml`), scripts (`script/`), or
core files (`src/lib.rs`, `src/vorpal.rs`, `Cargo.toml`).

**Review approach**: These affect all artifacts. Review with higher scrutiny. Changes to
`detect-changed-artifacts.sh` should be verified against its test suite. Changes to
`src/lib.rs` (ProjectEnvironment) affect the development environment for all users.

---

## High-Risk Areas

These areas of the codebase carry disproportionate risk and warrant extra review attention:

| Area | Risk | Reason |
|---|---|---|
| `src/lib.rs` (ProjectEnvironment) | High | Defines the dev environment; affects all contributors |
| `.github/workflows/vorpal.yaml` | High | CI pipeline; broken workflow blocks all builds |
| `script/detect-changed-artifacts.sh` | High | Determines which artifacts CI builds; incorrect detection wastes CI or misses builds |
| Artifacts with dependencies (gpg, ttyd, nnn, tmux, neovim, zsh) | High | Complex dependency chains; failures cascade |
| `src/artifact/file.rs` | Medium | Utility artifact used by others; interface changes break dependents |
| `Vorpal.toml` | Medium | Project-level build configuration; incorrect settings affect all builds |

---

## Artifacts by Complexity Tier

Understanding which artifacts are simple vs complex helps calibrate review effort.

### Simple (pre-built binary, no dependencies)
argocd, bat, bottom, crane, cue, direnv, doppler, fd, helm, jq, just, k9s, kn, kubectl,
kubeseal, lazygit, lima, ripgrep, starship, terraform, yq

### Moderate (source build, no inter-artifact dependencies)
beads, cmake, ffmpeg, jj, libevent, ncurses, nginx, npth, openjdk, pkg_config, sqlite3,
vhs, zlib

### Moderate (pre-built + runtime dependency)
openapi_generator_cli (depends on openjdk)

### Complex (source build with inter-artifact dependencies)
gpg (5 deps: libassuan, libgcrypt, libgpg_error, libksba, npth),
ttyd (5 deps: cmake, json_c, libuv, libwebsockets, mbedtls; platform-conditional build),
libwebsockets (3 deps: cmake, libuv, mbedtls),
nnn (3 deps: ncurses, pkg_config, readline),
tmux (depends on libevent, ncurses),
neovim (depends on cmake),
zsh (depends on ncurses),
libassuan (depends on libgpg_error),
libgcrypt (depends on libgpg_error),
libksba (depends on libgpg_error),
readline (depends on ncurses),
libuv (depends on cmake),
mbedtls (depends on cmake),
json_c (depends on cmake),
awscli2 (source install with platform-specific steps)

---

## Existing Review Infrastructure

### What Exists

- **CI pipeline**: GitHub Actions workflow builds artifacts on all 4 platform runners
  (macos-latest, macos-latest-large, ubuntu-latest, ubuntu-latest-arm64). CI runs on both
  pull requests and pushes to main.
- **Changed artifact detection**: `script/detect-changed-artifacts.sh` identifies which
  artifacts were modified, enabling incremental CI builds.
- **Regression tests for CI scripts**: `script/test-detect-changed-artifacts.sh` provides
  regression tests for the change detection logic.
- **Renovate**: Automated dependency update bot configured with recommended defaults.
- **Vorpal.lock**: Content-addressable source integrity verification via SHA-256 digests.
- **S3 registry backend**: Built artifacts are stored in an S3 registry
  (`altf4llc-vorpal-registry`), providing a cache layer.

### What Does Not Exist

- **No PR template**: No `.github/PULL_REQUEST_TEMPLATE.md` exists.
- **No CODEOWNERS file**: No `.github/CODEOWNERS` for automatic review assignment.
- **No CONTRIBUTING guide**: No contribution guidelines documented.
- **No branch protection rules** (not verified, but no evidence of required reviews in workflow).
- **No automated linting**: No `cargo clippy` or `cargo fmt --check` in CI.
- **No unit tests**: The Rust code has no `#[test]` modules. Build correctness is verified
  entirely through CI builds on all platforms.
- **No integration test suite**: No automated verification that built artifacts actually work
  (e.g., running `argocd version` after build).

---

## Commit Message Convention

The project uses conventional commit format with scope, based on observed history:

```
type(scope): description
```

Common types observed: `feat`, `fix`, `chore`, `refactor`, `patch`, `docs`
Common scopes observed: `artifact`, `vorpal`, `ci`, `lock`, `spec`, `lib`, specific artifact names

---

## Recommended Review Workflow

1. **Categorize the change** using the categories above to set time budget.
2. **Check cross-platform coverage first** -- this is the most common failure mode.
3. **For dependency changes**, trace the full dependency chain.
4. **For build scripts**, verify against upstream documentation for the specified version.
5. **For infrastructure changes**, verify against the test suite and consider blast radius.
6. **Approve with confidence** for simple pre-built binary artifacts that follow established
   patterns. These are low-risk and highly formulaic.
7. **Request split** if a PR mixes artifact additions with infrastructure changes.
