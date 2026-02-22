# Review Strategy

This document describes the review strategy for the `artifacts.vorpal` project. It identifies which
review dimensions to prioritize, areas of high risk, common pitfalls, and what matters most during
code review for this specific codebase.

---

## Project Context

This is a Rust project that defines ~45 software artifact build definitions for the Vorpal build
system. The codebase is primarily single-contributor (one human author, plus Renovate bot for
dependency updates). Changes are overwhelmingly one of two types: adding a new artifact or updating
an existing artifact's version/build script.

The project has no Rust unit tests. The only test coverage is a shell-based regression test suite
for the CI artifact detection script. The Rust code compiles to a binary that is executed by Vorpal
at build time, so "testing" happens implicitly when artifacts are built in CI across four
platform/architecture combinations.

---

## Review Dimension Priorities

The six standard review dimensions are ranked here by their relevance to this specific project.

### 1. Security (HIGH)

**Why this ranks highest:** Every artifact definition downloads binaries or source code from
external URLs, extracts archives, and runs shell scripts. This is a software supply chain system.
A compromised URL, a missing integrity check, or a malicious build script could affect every
downstream consumer of these artifacts.

**What to look for:**
- Source URLs must point to official release channels (GitHub Releases, official project sites)
- Version pinning must be explicit (no `latest` tags, no floating references)
- Shell scripts embedded in `formatdoc!` blocks must not introduce injection vectors through
  string interpolation of untrusted input
- `chmod` and file permission operations should follow least privilege
- No credentials, tokens, or secrets in artifact definitions

**Current gaps:**
- No cryptographic hash verification of downloaded sources (no SHA256 checksums)
- No signature verification for any downloaded artifacts
- Source URLs are HTTP/HTTPS strings with no integrity pinning beyond the version in the URL path

### 2. Architecture (HIGH)

**Why this ranks high:** The builder pattern and dependency graph are the structural backbone of
the project. Breaking the pattern or miswiring dependencies causes cascading build failures across
all four platforms.

**What to look for:**
- New artifacts must follow the established builder pattern: struct with `new()`, optional
  `with_*()` methods, `async fn build(self, context: &mut ConfigContext) -> Result<String>`
- Dependency injection via `with_*()` must match what `src/vorpal.rs` passes in
- Dependencies must be built before dependents in `src/vorpal.rs` (topological ordering)
- The `file.rs` module is a utility, not a standard artifact -- it is excluded from CI detection
  via `EXCLUDED_FILES` in `detect-changed-artifacts.sh`
- All four platform systems (`Aarch64Darwin`, `Aarch64Linux`, `X8664Darwin`, `X8664Linux`) must
  be handled in every artifact's `match context.get_system()` block

### 3. Operations (MEDIUM)

**Why this ranks medium:** The CI pipeline (`vorpal.yaml`) is tightly coupled to the artifact
detection script. Changes to CI infrastructure, the detection script, or the naming convention
can silently break the build pipeline.

**What to look for:**
- Changes to `script/detect-changed-artifacts.sh` must pass the regression test suite
- Filename-to-artifact-name conversion (`snake_case.rs` to `kebab-case`) must be preserved
- The `EXCLUDED_FILES` array in the detection script must stay in sync with utility modules
- CI workflow changes should be tested against both PR and push event types
- S3 registry backend credentials (`AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`) must remain in
  GitHub Secrets, never hardcoded

### 4. Code Quality (MEDIUM)

**Why this ranks medium:** The highly repetitive nature of artifact definitions means consistency
is more important than cleverness. Deviations from the pattern create maintenance burden.

**What to look for:**
- Consistent use of `formatdoc!` for shell scripts
- Consistent struct/impl layout matching existing artifacts
- Source version variables named consistently (`source_version` or `version`)
- Build scripts follow the `mkdir -pv "$VORPAL_OUTPUT"` / extract / copy-or-build / install pattern
- Error handling uses `anyhow::anyhow!("Unsupported system for <name> artifact")` for unsupported
  platforms
- Dependency fallback pattern (the `match self.<dep> { Some(val) => val, None => &build() }`)
  is used consistently for optional dependencies

### 5. Performance (LOW)

**Why this ranks low:** This is a build configuration project, not a runtime system. Performance
concerns are limited to CI pipeline efficiency, which is already addressed by the changed-artifact
detection mechanism that avoids rebuilding unchanged artifacts.

**What to look for:**
- Avoid unnecessary dependencies between artifacts that would force sequential builds
- Shell build scripts should not do redundant work (e.g., downloading the same source twice)
- The `build-dev` job runs on all four runners before individual artifact builds; changes that
  increase dev environment build time affect every CI run

### 6. Testing (LOW -- but a known gap)

**Why this ranks low:** There are no Rust unit tests and the project explicitly states this. The
only testing is the shell script regression suite for artifact detection. Artifacts are validated
implicitly by whether they build successfully in CI across all four platforms.

**What to look for:**
- Changes to `detect-changed-artifacts.sh` should include corresponding test cases in
  `test-detect-changed-artifacts.sh`
- If a new utility module (like `file.rs`) is added that should be excluded from detection, it
  must be added to both `EXCLUDED_FILES` and tested
- Regression tests should use real commit SHAs from the repository history

---

## High-Risk Areas

These are the areas of the codebase where bugs have the highest blast radius or are hardest to
detect during review.

### 1. Dependency Ordering in `src/vorpal.rs`

**Risk: Build failure across all platforms.**
The `main()` function in `vorpal.rs` builds artifacts in a specific order. Dependencies must be
built before the artifacts that consume them. Getting this wrong causes runtime failures that may
only surface on specific platforms.

**Review checklist:**
- [ ] Is the new artifact placed after all its dependencies in `vorpal.rs`?
- [ ] Are `with_*()` calls passing the correct variable names?
- [ ] Does the artifact actually use the injected dependency in its build script?

### 2. Shell Scripts in `formatdoc!` Blocks

**Risk: Silent build failures, platform-specific breakage.**
Every artifact embeds a shell script via Rust's `formatdoc!` macro. These scripts run inside the
Vorpal sandbox and failures may not be obvious from the Rust compilation alone.

**Review checklist:**
- [ ] Does the script use `set -euo pipefail` semantics (inherited from Vorpal)?
- [ ] Are paths properly quoted (especially `$VORPAL_OUTPUT`)?
- [ ] Do `pushd` calls match the expected archive extraction layout?
- [ ] Are configure/make flags correct for the target platform?
- [ ] Do `CPPFLAGS`, `LDFLAGS`, and `rpath` entries reference all required dependencies?

### 3. Platform-Specific URL and Build Logic

**Risk: One or more of four platforms broken silently.**
Each artifact must provide correct source URLs and build instructions for all four platform
variants. A typo in a platform-specific URL will only fail when that platform combination runs
in CI.

**Review checklist:**
- [ ] Are all four systems covered in the `match context.get_system()` block?
- [ ] Do platform-specific URLs use the correct arch/OS naming convention for the upstream project?
- [ ] Is the wildcard `_` arm returning an error (not silently succeeding)?

### 4. CI Artifact Detection Script

**Risk: Missing builds or unnecessary full rebuilds.**
The detection script determines which artifacts CI builds. Bugs here either skip necessary builds
(allowing broken artifacts to ship) or trigger full rebuilds (wasting CI resources).

**Review checklist:**
- [ ] Does the script handle deleted files correctly (`--diff-filter=d`)?
- [ ] Is the `EXCLUDED_FILES` array up to date?
- [ ] Does the `snake_case` to `kebab-case` conversion work for the new artifact name?

---

## Common Pitfalls

These are mistakes that have occurred or are likely to occur based on the codebase's history.

| Pitfall | Example | How to Catch |
|---|---|---|
| Wrong archive directory name in build script | `pushd ./source/name/project-v1.0` when the archive extracts to `project-1.0` (no `v` prefix) | Verify against upstream release archive structure |
| Missing platform in match block | Handling only Darwin variants and forgetting Linux | Confirm all four `ArtifactSystem` variants are present |
| Dependency declared in struct but not wired in `vorpal.rs` | Adding `with_ncurses` method but not calling it in main | Cross-reference `vorpal.rs` with the artifact's `with_*()` methods |
| Incorrect version tag format | SQLite uses `3510200` for version `3.51.2`; some projects use `v` prefix, others don't | Check upstream release page for exact tag/URL format |
| Stale `Cargo.lock` after SDK update | `vorpal-sdk` is pinned to a git branch; lock file may not reflect latest | Run `cargo check` after SDK-related changes |
| Non-artifact file included in CI detection | Adding a utility `.rs` file to `src/artifact/` without adding it to `EXCLUDED_FILES` | Check if the new file is a standalone artifact or a utility |

---

## Review Process

### Current State

- **No formal review process exists.** The project is primarily single-contributor with direct
  pushes to `main`.
- **No PR template.** No `.github/PULL_REQUEST_TEMPLATE.md` exists.
- **No CODEOWNERS file.** No required reviewers are configured.
- **No branch protection rules** are visible in the repository configuration files.
- **No CONTRIBUTING guide.** No contribution guidelines exist.
- **Renovate bot** handles dependency updates automatically with recommended config.
- **Conventional Commits** are used informally (e.g., `feat(artifact):`, `fix(ci):`, `chore(deps):`)
  but there is no enforcing mechanism.

### Recommended Review Focus by Change Type

| Change Type | Effort | Primary Focus |
|---|---|---|
| New simple artifact (binary download) | Quick (5-10 min) | URL correctness, all 4 platforms covered, builder pattern followed |
| New complex artifact (source build) | Medium (15-30 min) | Dependency wiring, build script correctness, linker flags, `vorpal.rs` ordering |
| Artifact version bump | Quick (5 min) | URL still valid, archive structure unchanged, version variables consistent |
| Dependency library change (ncurses, libgpg_error, etc.) | Medium (15-30 min) | All downstream dependents still build, `rpath` and `LDFLAGS` updated |
| CI/detection script change | Medium (15 min) | Regression tests pass, edge cases (deleted files, new utilities) handled |
| `vorpal-sdk` dependency update | Medium (10-15 min) | API compatibility, `Cargo.lock` updated, `cargo check` passes |
| `lib.rs` or `ProjectEnvironment` changes | High (20-30 min) | Dev environment still functional, Lima/Protoc/Rust toolchain integration intact |

---

## Automated Checks

### Currently in Place

- **`cargo build` / `cargo check`**: Validates Rust compilation. Catches type errors, missing
  imports, and syntax issues. Does NOT validate embedded shell scripts.
- **CI matrix build**: Builds changed artifacts on 4 runners (macOS arm64, macOS x86_64, Ubuntu
  arm64, Ubuntu x86_64). This is the primary validation that artifacts actually work.
- **`detect-changed-artifacts.sh`**: Limits CI scope to only changed artifacts.
- **Renovate bot**: Automated dependency update PRs for GitHub Actions.

### Not Currently in Place

- **No linting** (no `clippy` in CI)
- **No formatting enforcement** (no `rustfmt` check in CI)
- **No shell script linting** (no `shellcheck` for embedded scripts or standalone scripts)
- **No security scanning** (no `cargo audit`, no dependency vulnerability checks)
- **No hash verification** of downloaded artifact sources
- **No PR checks or required reviews** before merge

---

## Artifact-Specific Review Notes

### Utility Modules (Non-Artifact Files)

`src/artifact/file.rs` is a utility module, not a standalone artifact. It is excluded from CI
detection via the `EXCLUDED_FILES` array. Any new utility module added to `src/artifact/` must
also be added to this array to prevent CI from attempting to build it as a standalone artifact.

### Artifacts with Dependencies

These artifacts require extra scrutiny because changes to their dependencies can break them:

| Artifact | Dependencies | Risk |
|---|---|---|
| `gpg` | `libassuan`, `libgcrypt`, `libgpg_error`, `libksba`, `npth` | Highest -- 5 dependencies, complex configure flags |
| `nnn` | `ncurses`, `pkg_config`, `readline` | High -- 3 dependencies, readline itself depends on ncurses |
| `tmux` | `libevent`, `ncurses` | Medium -- 2 dependencies |
| `readline` | `ncurses` | Medium -- transitive dependency for `nnn` |
| `zsh` | `ncurses` | Medium -- 1 dependency |
| `openapi-generator-cli` | `openjdk` | Medium -- Java runtime dependency |
| `libassuan` | `libgpg_error` | Medium -- GPG chain |
| `libgcrypt` | `libgpg_error` | Medium -- GPG chain |
| `libksba` | `libgpg_error` | Medium -- GPG chain |

### The GPG Dependency Chain

The GPG artifact chain (`libgpg_error` -> `libassuan`/`libgcrypt`/`libksba` -> `gpg`) is the
most complex dependency graph in the project. A version bump to `libgpg_error` requires
verification that all four downstream artifacts still build correctly. The CI detection script
does NOT automatically rebuild dependents when a dependency changes -- each artifact file must be
individually modified to trigger a rebuild.

---

## Key Takeaway

For this project, the most valuable review time is spent on:

1. **Verifying source URLs and version strings** -- these are the most common source of breakage
   and cannot be caught by the Rust compiler
2. **Checking dependency wiring** -- ensuring `vorpal.rs` ordering and `with_*()` calls are correct
3. **Validating platform coverage** -- confirming all four architecture/OS combinations are handled
4. **Reviewing embedded shell scripts** -- the Rust compiler cannot validate these; they fail only
   at build time in CI
