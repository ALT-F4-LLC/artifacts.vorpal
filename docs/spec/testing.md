# Testing Specification

This document describes the current testing strategy, tooling, and coverage for the
`vorpal-artifacts` project. It reflects what actually exists in the codebase as of the latest
review.

---

## Overview

The project has a minimal testing footprint. There are **no Rust unit tests, integration tests,
or end-to-end tests** in the Rust source code. The only automated tests are **bash-based
regression tests** for the CI artifact detection script. Artifact correctness is validated through
**build-time verification** in CI, not through a traditional test suite.

This is consistent with the project's nature: it is a collection of declarative artifact
definitions (build recipes), not a library or application with complex business logic. The
"tests" are the builds themselves -- if an artifact builds and produces a valid binary, it works.

---

## Test Pyramid

### Unit Tests

**Status: None exist.**

There are zero `#[test]` functions, zero `#[cfg(test)]` modules, and zero test files anywhere in
`src/`. No Rust assertion macros (`assert!`, `assert_eq!`, `assert_ne!`) are used in the source
code.

This is acknowledged in `CLAUDE.md`:

> There are no Rust unit tests; testing covers the CI artifact detection scripts.

**What could be unit tested (if desired):**
- The `filename_to_artifact()` conversion logic in `detect-changed-artifacts.sh` (currently tested
  indirectly through bash regression tests)
- Builder pattern validation (e.g., that `Gpg::new()` without required dependencies produces a
  meaningful error)
- Platform system matching logic (the `context.get_system()` match arms)

**Why they likely don't exist:**
- Artifact definitions are declarative -- they are essentially configuration, not logic
- The `vorpal-sdk` handles the complex build orchestration; this project just composes it
- Build correctness is verified by actually running `vorpal build <artifact>` in CI

### Integration Tests

**Status: None exist.**

There are no Rust integration tests (no `tests/` directory). There is no test infrastructure
for validating artifact dependency resolution, source URL availability, or build script
correctness in isolation.

### End-to-End Tests

**Status: Implicit via CI builds.**

The CI pipeline (`.github/workflows/vorpal.yaml`) serves as the de facto end-to-end test suite:

1. **`build-dev` job**: Builds the `dev` `ProjectEnvironment` artifact across all 4 platform
   runners (`macos-latest`, `macos-latest-large`, `ubuntu-latest`, `ubuntu-latest-arm64`). This
   validates that the project compiles and the development environment artifact resolves
   correctly.

2. **`build` job**: Builds each changed artifact individually via `vorpal build '${{ matrix.artifact }}'`
   across all 4 runners using a matrix strategy (`fail-fast: false`). This validates that each
   modified artifact definition produces a successful build on all target platforms.

If an artifact's source URL is broken, its build script has a bug, or its dependencies are
misconfigured, the CI build job fails. This is the primary correctness signal.

### Script Tests

**Status: One test file exists.**

`script/test-detect-changed-artifacts.sh` contains 6 test functions for the CI artifact detection
script:

| Test | Purpose |
|---|---|
| `test_list_returns_artifacts` | Verifies `--list` returns a non-zero count of artifacts |
| `test_all_returns_json` | Verifies `--all` produces a valid JSON array |
| `test_empty_diff_returns_empty` | Verifies `HEAD HEAD` comparison returns `[]` |
| `test_non_artifact_changes_dont_trigger_full_rebuild` | Regression: non-artifact file changes (e.g., `Cargo.lock`, `vorpal.rs`) do not trigger all artifacts to rebuild |
| `test_deleted_files_excluded` | Regression: deleted artifact `.rs` files are excluded from the build list |
| `test_no_hardcoded_artifacts` | Regression: no `CORE_FILES`, `is_core_file`, or `DEPENDENTS` mechanisms in detection script |

**Test runner**: Plain bash with manual pass/fail tracking (colored output, exit code 1 on any
failure). No test framework.

**External dependencies**: Requires `jq` and `git` with access to repository history (uses
specific commit SHAs for regression tests).

**How to run**:
```bash
./script/test-detect-changed-artifacts.sh
```

---

## CI/CD Testing Pipeline

### Workflow: `.github/workflows/vorpal.yaml`

Triggered on: `pull_request` and `push` to `main`.

| Job | Purpose | Runners |
|---|---|---|
| `build-changes` | Detect which artifacts changed between base and head SHA | `ubuntu-latest` |
| `build-dev` | Build the `dev` environment artifact | 4 runners (macOS arm64, macOS x86, Ubuntu x86, Ubuntu arm64) |
| `build` | Build each changed artifact individually | 4 runners x N changed artifacts (matrix) |

Key properties:
- `build` uses `fail-fast: false` -- all artifact/runner combinations are tested even if one fails
- `build` only runs when `build-changes` detects `has_changes == 'true'`
- Artifact detection uses `script/detect-changed-artifacts.sh` with commit SHA comparison
- The Vorpal build system is installed via `ALT-F4-LLC/setup-vorpal-action@main` with an S3
  registry backend

### What CI Validates

- **Compilation**: `vorpal build 'dev'` compiles the Rust project and resolves the dev environment
- **Artifact builds**: Each changed artifact is built on all 4 target platforms
- **Cross-platform**: macOS (arm64 + x86_64) and Ubuntu Linux (arm64 + x86_64) are all tested
- **Dependency resolution**: Artifacts with dependencies (e.g., `gpg`, `tmux`, `nnn`) must
  resolve their full dependency chain

### What CI Does NOT Validate

- **Artifact functionality**: A binary that downloads and extracts successfully is not tested for
  correct operation (e.g., `jj --version` is never run)
- **Version correctness**: Source URLs and version strings are not validated against upstream
  releases
- **Script test suite**: `test-detect-changed-artifacts.sh` is NOT run in CI -- it must be run
  manually
- **Rust linting**: No `cargo clippy`, `cargo fmt --check`, or `cargo test` steps exist in CI
- **Unchanged artifacts**: Only changed artifacts are built; unchanged artifacts could silently
  break if an upstream source URL becomes unavailable

---

## Test Tooling

### In Use

| Tool | Purpose | Location |
|---|---|---|
| Bash test script | Regression tests for artifact detection | `script/test-detect-changed-artifacts.sh` |
| `jq` | JSON validation in test assertions | Required by test script |
| GitHub Actions | CI build-based validation | `.github/workflows/vorpal.yaml` |
| Vorpal build system | Build-time artifact verification | CI workflow |

### Not In Use

| Tool | Notes |
|---|---|
| `cargo test` | No Rust tests exist |
| `cargo clippy` | Not configured; no CI step |
| `cargo fmt` | No `.rustfmt.toml`; no CI step |
| `clippy.toml` | Does not exist |
| Coverage tools (`tarpaulin`, `llvm-cov`) | Not configured |
| Property-based testing (`proptest`, `quickcheck`) | Not used |
| Mocking frameworks (`mockall`, etc.) | Not used |
| Snapshot testing (`insta`) | Not used |

---

## Test Data and Fixtures

The bash test script uses hardcoded git commit SHAs as test fixtures:

- `5fc8933e5f5ddb18b00d2d307d676e4c503814c7` to `98beb7acbc192e0471891fed1942026c7fbc6296`:
  PR #9 squash merge adding `beads` and `jj` artifacts
- `8ac021d` to `02d3340`: Deletion of `linux-vorpal-slim` and `rsync` artifacts

These are **repository-specific** and require the full git history to work (`fetch-depth: 0`
in checkout).

---

## Gaps and Risks

### Critical Gaps

1. **No CI execution of the script test suite**: `test-detect-changed-artifacts.sh` must be run
   manually. There is no CI step that invokes it, meaning regressions in the detection script
   could ship unnoticed.

2. **No Rust-level testing**: Zero `cargo test` coverage means:
   - Builder pattern misuse (e.g., calling `.build()` without required dependencies) is only
     caught at runtime during CI builds
   - Platform match exhaustiveness is verified by the compiler but error messages are not tested
   - Refactoring safety relies entirely on `cargo check` and manual review

3. **No linting in CI**: Without `cargo clippy` or `cargo fmt --check`, code quality issues
   (unused imports, suboptimal patterns, formatting drift) are not caught automatically.

### Moderate Gaps

4. **No artifact functionality validation**: Built artifacts are not smoke-tested. A binary could
   extract successfully but be corrupt, have missing shared libraries, or be the wrong version.

5. **No upstream availability monitoring**: If an upstream source URL returns 404, the artifact
   will only fail when it is next changed and rebuilt in CI. Unchanged artifacts with broken URLs
   are invisible until someone tries to build them.

6. **Hardcoded commit SHAs in tests**: The regression tests depend on specific commits existing in
   the repository. Force-push or history rewrite would break them.

### Low-Priority Gaps

7. **No build performance benchmarking**: There is no tracking of how long artifacts take to
   build, which could help detect regressions in build times.

8. **No lock file validation**: `Vorpal.lock` is uploaded as a CI artifact but not validated or
   compared across runs.

---

## Recommendations for New Changes

When contributing to this project, the following testing expectations apply:

### Adding a New Artifact

- No Rust tests are expected or required
- The artifact will be validated by CI when its `src/artifact/<name>.rs` file is detected as
  changed by `detect-changed-artifacts.sh`
- The CI `build` job will attempt `vorpal build '<name>'` on all 4 runners
- Manual verification: run `cargo check` locally before pushing

### Modifying the Detection Script

- Run `./script/test-detect-changed-artifacts.sh` locally before pushing
- Consider adding a new regression test if fixing a specific bug
- The test script requires `jq` and full git history

### Modifying Core Files

- Changes to `src/lib.rs`, `src/artifact.rs`, or `src/vorpal.rs` do NOT automatically trigger
  rebuilds of all artifacts (this is by design, per the regression test)
- Run `cargo check` locally to verify compilation
- If modifying `ProjectEnvironment`, the `build-dev` CI job will validate it

---

## How to Run Tests

```bash
# Compile check (fast, catches type errors and missing imports)
cargo check

# Script regression tests (requires jq, git history)
./script/test-detect-changed-artifacts.sh

# Full build of a specific artifact (requires Vorpal installed)
vorpal build <artifact-name>

# Full build of the dev environment (requires Vorpal installed)
vorpal build dev
```

There is no single command that runs "all tests" because the project's testing is fragmented
across compilation checks, bash scripts, and CI builds.
