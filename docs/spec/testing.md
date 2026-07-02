# Testing Specification

## Overview

This document describes the current state of testing in the `vorpal-artifacts` project. The
project is a Rust-based Vorpal artifact registry that defines build configurations for 55+
software packages (CLI tools, libraries, development environments) across four target systems
(aarch64-darwin, aarch64-linux, x86_64-darwin, x86_64-linux).

Testing in this project is minimal. There are **no Rust unit tests, no integration test
harnesses, no `[dev-dependencies]` in `Cargo.toml`, no `#[cfg(test)]` or `#[test]`
attributes, and no shell script tests** anywhere in the project.

---

## What Currently Exists

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
| **Shell script tests** | 0 | None | No shell test files exist |
| **E2E / Build verification** | Implicit via CI | GitHub Actions + Vorpal | Verifies artifacts build on 4 platforms |

The test pyramid is effectively **empty** -- the only validation mechanism is full CI builds
(the most expensive and slowest form of verification).

---

## Test Infrastructure

### Test Runners

- **Rust tests:** `cargo test` is available (Cargo.toml is properly configured) but never invoked.
  No CI step runs it.

### Coverage Tools

None. No coverage tooling is configured or referenced anywhere in the project.

### Mocking and Fixtures

None. The Rust source has no test utilities, mock implementations, or test fixtures.

### Test Utilities

None.

---

## How to Run Tests

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

4. **No shell script tests.** There is no regression coverage for `detect-changed-artifacts.sh`
   or any other script in `script/`.

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

8. **No test documentation.** There is no guidance for contributors on what tests to write when
   adding a new artifact module.

---

## Recommendations for Test Strategy

These are observations about what would be reasonable to add, ordered by impact relative to effort.

### High Value, Low Effort

- **Add `cargo test` and `cargo clippy` steps to CI.** Even with zero tests, this catches
  compilation errors and common Rust anti-patterns automatically.

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
