# Testing Specification

## Overview

This document describes the current state of testing in the `vorpal-artifacts` project. The
project is a Rust-based Vorpal artifact registry that defines build configurations for 55+
software packages (CLI tools, libraries, development environments) across four target systems
(aarch64-darwin, aarch64-linux, x86_64-darwin, x86_64-linux).

Testing in this project is minimal. There are **no Rust unit tests, no integration test
harnesses, no `[dev-dependencies]` in `Cargo.toml`, and no `#[cfg(test)]` or `#[test]`
attributes** anywhere in the Rust source code. The only tests that exist are shell-based
regression tests for the CI helper script `detect-changed-artifacts.sh`.

---

## What Currently Exists

### Shell Script Tests

**Location:** `script/test-detect-changed-artifacts.sh`

The sole test file in the project. It is a hand-rolled bash test harness that validates the
`script/detect-changed-artifacts.sh` script, which is responsible for detecting which artifacts
need rebuilding based on git diffs between commits.

**Test count:** 8 assertions across 6 test functions.

**Test functions:**

| Test | What It Verifies |
|---|---|
| `test_list_returns_artifacts` | `--list` flag returns a non-zero count of artifacts |
| `test_all_returns_json` | `--all` flag returns valid JSON array (validated with `jq`) |
| `test_empty_diff_returns_empty` | Comparing `HEAD` to `HEAD` returns empty array `[]` |
| `test_non_artifact_changes_dont_trigger_full_rebuild` | Changes to non-artifact files (e.g., `src/vorpal.rs`, `Cargo.lock`, workflow files) do NOT trigger a full rebuild -- only the specific changed artifact modules are included (regression test) |
| `test_deleted_files_excluded` | Deleted artifact `.rs` files are excluded from the build list (regression test) |
| `test_no_hardcoded_artifacts` | Verifies that removed mechanisms (`CORE_FILES`, `is_core_file`, `DEPENDENTS`) do not reappear in the script (regression test) |

**Test runner:** Direct bash execution (`./test-detect-changed-artifacts.sh`). No test framework.
Uses colored PASS/FAIL output and exits with code 1 if any test fails.

**Dependencies:** Requires `jq` and a git repository with commit history (some tests reference
specific commit SHAs like `5fc8933e` and `98beb7a`).

**Note:** These tests are NOT integrated into the CI workflow (`vorpal.yaml`). They must be run
manually.

### CI Build Verification

**Location:** `.github/workflows/vorpal.yaml`

The CI workflow provides a form of implicit integration testing through successful artifact builds.
It does NOT run any explicit test commands (`cargo test`, `cargo check`, `cargo clippy`, etc.).

**CI structure:**

1. **`build-changes` job:** Detects which artifacts changed between commits using
   `detect-changed-artifacts.sh`.
2. **`build-dev` job:** Builds the `dev` project environment on all four runners (macos-latest,
   macos-latest-large, ubuntu-latest, ubuntu-latest-arm64). Uses Vorpal's own build system with
   an S3 registry backend.
3. **`build` job:** Builds each changed artifact individually on all four runners using a matrix
   strategy with `fail-fast: false`.

The CI validates that artifact definitions compile and that the Vorpal build system can execute
them, but this is build verification, not testing.

### In-Script Assertions

A small number of artifact build scripts contain inline bash assertions using `test -f` to verify
expected files exist after extraction:

- `src/artifact/awscli2.rs` (lines 55-56): Verifies `aws` and `aws_completer` executables exist
  after `pkgutil` extraction on macOS.

These are build-time sanity checks embedded in shell scripts, not structured tests.

---

## Test Pyramid Breakdown

| Level | Count | Tools/Framework | Notes |
|---|---|---|---|
| **Unit tests** | 0 | None | No `#[test]` attributes, no `#[cfg(test)]` modules, no `[dev-dependencies]` |
| **Integration tests** | 0 | None | No `tests/` directory, no integration test crate |
| **Shell script tests** | 6 functions (8 assertions) | Hand-rolled bash | Only for `detect-changed-artifacts.sh`; not in CI |
| **E2E / Build verification** | Implicit via CI | GitHub Actions + Vorpal | Verifies artifacts build on 4 platforms |

The test pyramid is effectively **inverted** -- the only structured testing is at the script
level, and the primary validation mechanism is full CI builds (the most expensive and slowest
form of verification).

---

## Test Infrastructure

### Test Runners

- **Shell tests:** Direct bash execution, no framework.
- **Rust tests:** `cargo test` is available (Cargo.toml is properly configured) but never invoked.
  No CI step runs it.

### Coverage Tools

None. No coverage tooling is configured or referenced anywhere in the project.

### Mocking and Fixtures

None. The Rust source has no test utilities, mock implementations, or test fixtures. The shell
test file uses real git history (specific commit SHAs) as test fixtures, which makes them
coupled to the repository's commit history.

### Test Utilities

None beyond the `pass()` and `fail()` helper functions in
`script/test-detect-changed-artifacts.sh`.

---

## How to Run Tests

### Shell Script Tests

```bash
./script/test-detect-changed-artifacts.sh
```

Requires: `bash`, `jq`, full git history (tests reference specific SHAs).

### Rust Tests (No Tests Exist)

```bash
cargo test
```

This will compile and report 0 tests. It can still be useful as a compilation check.

### Compilation Check

```bash
cargo check
cargo build
```

Currently the only way to validate that Rust source code is correct.

---

## Gaps and Missing Pieces

### Critical Gaps

1. **No Rust unit tests.** The 55+ artifact modules, the `ProjectEnvironment` struct, and
   `lib.rs` have zero test coverage. Logic like system matching (`match context.get_system()`),
   URL construction, and dependency wiring in complex artifacts (e.g., `gpg.rs` with 5
   dependencies) is entirely untested.

2. **No `cargo test` in CI.** Even though `cargo test` would catch compilation errors and run
   any future tests, it is not part of the CI pipeline.

3. **No `cargo clippy` or `cargo fmt --check` in CI.** The Vorpal lock file shows clippy and
   rustfmt are distributed as part of the Rust toolchain, but they are not used in any
   automated pipeline.

4. **Shell tests not in CI.** The existing shell regression tests are not executed by the GitHub
   Actions workflow.

### Moderate Gaps

5. **No integration tests against the Vorpal SDK.** The project depends heavily on `vorpal-sdk`
   (from a git branch dependency), but there are no tests that verify the interface contract
   between this project and the SDK.

6. **No validation of artifact source URLs.** Source URLs are hardcoded strings. There is no
   automated verification that URLs resolve or that digests (in `Vorpal.lock`) match expected
   values outside of a full build.

7. **No property-based or fuzz testing.** Given the repetitive structure of artifact modules
   (version strings, URL templates, system matching), property-based testing could catch
   pattern-level bugs across all 55+ artifacts.

### Minor Gaps

8. **Shell test fixtures use hardcoded commit SHAs.** Tests like
   `test_non_artifact_changes_dont_trigger_full_rebuild` depend on specific commits
   (`5fc8933e5f5ddb18b00d2d307d676e4c503814c7`). These will break if the repository is rebased,
   shallow-cloned, or if history is rewritten.

9. **No test documentation.** There is no guidance for contributors on what tests to write when
   adding a new artifact module.

---

## Recommendations for Test Strategy

These are observations about what would be reasonable to add, ordered by impact relative to effort.

### High Value, Low Effort

- **Add `cargo test` and `cargo clippy` steps to CI.** Even with zero tests, this catches
  compilation errors and common Rust anti-patterns automatically.
- **Add shell script tests to CI.** Add a job that runs
  `./script/test-detect-changed-artifacts.sh` (requires `jq` and full git history).

### High Value, Medium Effort

- **Add unit tests for artifact URL construction and system matching.** Each artifact's `build()`
  method performs system-dependent branching and string formatting. These are pure logic that
  can be tested without the Vorpal SDK context by extracting the logic into testable functions.
- **Add a compilation test for each artifact module.** A basic test that constructs each struct
  and verifies it implements the expected interface.

### Medium Value, Higher Effort

- **Add integration tests using a mock `ConfigContext`.** Test that `build()` methods produce
  the expected Vorpal artifact configurations without actually running builds.
- **Add URL liveness checks as a scheduled CI job.** Verify that artifact source URLs still
  resolve (HTTP HEAD requests), catching upstream breakage before build time.
