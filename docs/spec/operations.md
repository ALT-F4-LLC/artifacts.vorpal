# Operations Specification

> Last updated: 2026-02-21

This document describes the operational characteristics, deployment strategy, CI/CD pipeline,
and environment management for the `artifacts.vorpal` project as they actually exist in the
codebase today.

---

## 1. Project Nature

This is a **build-definition** project, not a deployed service. It produces artifact definitions
(Rust code consumed by the Vorpal build system) rather than running workloads. As a result,
traditional operational concerns like uptime, health checks, and production monitoring do not
apply directly. Instead, the operational surface is:

- **CI/CD pipeline** that validates and builds artifacts
- **Artifact registry** (S3-backed) that stores build outputs
- **Local development environments** including Lima VMs for cross-platform testing

---

## 2. CI/CD Pipeline

### 2.1. Workflow

A single GitHub Actions workflow is defined at `.github/workflows/vorpal.yaml` named `vorpal`.

**Trigger events:**
- `pull_request` (all branches)
- `push` to `main`

### 2.2. Jobs

The pipeline consists of three jobs:

#### Job 1: `build-changes`
- **Runner:** `ubuntu-latest`
- **Purpose:** Detects which artifacts have changed between the base and head commits.
- **Mechanism:** Runs `script/detect-changed-artifacts.sh` with commit SHAs derived from the
  event context (PR base SHA or push `before` SHA).
- **Outputs:** `artifacts` (JSON array of changed artifact names) and `has_changes` (boolean).
- **Edge case handling:** When `BASE_SHA` is the null SHA (`000...000`, e.g., initial branch
  push), falls back to `--all` mode, building every artifact.
- **Uses:** `actions/checkout@v6` with `fetch-depth: 0` for full git history.

#### Job 2: `build-dev`
- **Runner:** Matrix of 4 runners:
  - `macos-latest` (ARM64 macOS)
  - `macos-latest-large` (x86_64 macOS)
  - `ubuntu-latest` (x86_64 Linux)
  - `ubuntu-latest-arm64` (ARM64 Linux)
- **Purpose:** Builds the `dev` ProjectEnvironment on all four target platforms.
- **Steps:**
  1. Checkout code
  2. Set up Vorpal via `ALT-F4-LLC/setup-vorpal-action@main` with S3 registry backend
  3. Run `vorpal build 'dev'`
  4. Upload `Vorpal.lock` as a GitHub Actions artifact (named `<arch>-<os>-vorpal-lock`)
- **Runs on:** Every push/PR (no conditional gating)

#### Job 3: `build`
- **Conditional:** Only runs when `build-changes.outputs.has_changes == 'true'`
- **Dependencies:** `build-dev` and `build-changes` must complete first
- **Runner:** Same 4-runner matrix as `build-dev`
- **Strategy:** `fail-fast: false` -- all artifact/platform combinations are attempted even if
  some fail.
- **Matrix:** Artifact names from `build-changes` output crossed with the 4 runners, producing
  up to `N artifacts * 4 platforms` jobs.
- **Steps:**
  1. Checkout code
  2. Set up Vorpal via `ALT-F4-LLC/setup-vorpal-action@main`
  3. Run `vorpal build '<artifact-name>'`

### 2.3. Vorpal Setup Action

All build jobs use `ALT-F4-LLC/setup-vorpal-action@main` with the following configuration:

| Parameter | Value |
|---|---|
| `registry-backend` | `s3` |
| `registry-backend-s3-bucket` | `altf4llc-vorpal-registry` |
| `version` | `nightly` |

This action installs the Vorpal CLI and configures it to use an S3-backed artifact registry.
The `nightly` version is always used, meaning CI depends on the latest Vorpal build.

### 2.4. Required Secrets and Variables

| Name | Type | Purpose |
|---|---|---|
| `AWS_ACCESS_KEY_ID` | Secret | S3 registry authentication |
| `AWS_SECRET_ACCESS_KEY` | Secret | S3 registry authentication |
| `AWS_DEFAULT_REGION` | Variable | S3 bucket region |

These are referenced in the workflow as environment variables passed to the setup action.

---

## 3. Artifact Detection System

### 3.1. Script: `script/detect-changed-artifacts.sh`

This script is the core of the incremental build strategy. It dynamically discovers artifacts
from `src/artifact/*.rs` file names and detects which ones changed between two git commits.

**Key behaviors:**

- **Discovery:** Scans `src/artifact/*.rs`, excludes utility files (currently only `file.rs`),
  and converts `snake_case.rs` filenames to `kebab-case` artifact names.
- **Change detection:** Uses `git diff --name-only --diff-filter=d` to find changed files,
  filtering out deleted files (the `d` filter).
- **Output formats:**
  - `--all`: All artifacts as JSON array
  - `--list`: All artifacts as plain text, one per line
  - `<base_sha> <head_sha>`: Changed artifacts as JSON array
- **Important design decision:** Only changes to `src/artifact/<name>.rs` files trigger builds
  for that artifact. Changes to shared files like `src/vorpal.rs`, `Cargo.toml`, or workflow
  files do NOT trigger full rebuilds. This is intentional -- the CORE_FILES and DEPENDENTS
  mechanisms were deliberately removed (regression tests verify this).

### 3.2. Script: `script/test-detect-changed-artifacts.sh`

Regression test suite for the detection script. Tests cover:

- `--list` returns artifacts
- `--all` returns valid JSON
- Same-commit comparison returns empty array
- Non-artifact file changes do NOT trigger full rebuild (regression test)
- Deleted artifact files are excluded from build list (regression test)
- No hardcoded artifact mechanisms remain (regression test)

---

## 4. Artifact Registry

### 4.1. S3-Backed Registry

Built artifacts are stored in the S3 bucket `altf4llc-vorpal-registry`. This serves as both
a build cache and distribution mechanism. The Vorpal build system handles cache lookups and
uploads transparently.

### 4.2. Lock File

The `Vorpal.lock` file records the exact artifact hashes for a build. The CI pipeline uploads
this as a GitHub Actions artifact for each platform, enabling reproducibility verification
across the 4-runner matrix.

The `Vorpal.toml` configuration specifies the source includes:

```toml
language = "rust"

[source]
includes = ["src", "Cargo.toml", "Cargo.lock"]
```

---

## 5. Target Platforms

All artifacts target four platforms:

| Platform | CI Runner | Architecture |
|---|---|---|
| `Aarch64Darwin` | `macos-latest` | ARM64 macOS |
| `X8664Darwin` | `macos-latest-large` | x86_64 macOS |
| `Aarch64Linux` | `ubuntu-latest-arm64` | ARM64 Linux |
| `X8664Linux` | `ubuntu-latest` | x86_64 Linux |

Each artifact must provide platform-specific source URLs or build logic for all four targets.
The `DEFAULT_SYSTEMS` constant in `src/lib.rs` defines these four systems.

---

## 6. Local Development Environment

### 6.1. Build Commands

| Command | Purpose |
|---|---|
| `cargo build` | Compile the Rust artifact definitions |
| `cargo check` | Type-check without producing binaries |
| `vorpal build <artifact-name>` | Build a specific artifact via Vorpal |
| `vorpal build dev` | Build the full development environment |

### 6.2. Lima VM (Linux on macOS)

The project includes Lima VM support for testing Linux builds on macOS.

**`lima.yaml`** defines a Lima VM using Debian 13 (Trixie) cloud images:
- ARM64 and x86_64 images available
- Home directory mounted (read-only by default)
- `/tmp/lima` mounted writable
- `9p` mount type excluded (unsupported)

**`script/lima.sh`** provides VM provisioning:
- `deps`: Installs build dependencies (bubblewrap, build-essential, curl, jq, rsync, etc.),
  Docker, Vorpal CLI, and configures AppArmor for bubblewrap if needed.
- `sync`: Runs `deps` then rsyncs the project source to `$HOME/source` inside the VM,
  excluding `.git` and `target` directories.

**Usage** (per CLAUDE.md): `make lima` then `make lima-vorpal VORPAL_ARTIFACT=<name>`. Note:
no `Makefile` currently exists in the repository. This appears to be a documentation reference
to a Makefile that has been removed or exists in a parent/sibling project.

### 6.3. Linux Rootfs Slimming

**`script/linux-vorpal-slim.sh`** is a comprehensive rootfs slimming script (v1.0.0) that
reduces a Vorpal Linux installation from ~2.9GB to ~600-700MB. It removes development
toolchains, Python, Perl, static libraries, headers, documentation, and locale data while
preserving runtime essentials. This script supports dry-run mode, backup creation, selective
section execution, and aggressive mode (binary stripping).

---

## 7. Dependency Management

### 7.1. Rust Dependencies

Managed via `Cargo.toml` and `Cargo.lock`. The project has minimal dependencies:

- `anyhow` (error handling)
- `indoc` (indented string formatting)
- `tokio` (async runtime)
- `vorpal-sdk` (from `ALT-F4-LLC/vorpal.git`, `main` branch)

The `vorpal-sdk` dependency is pinned to the `main` branch of the upstream Vorpal repository
via git URL, not a crate version. This means builds track upstream changes.

### 7.2. Renovate

Automated dependency updates are configured via `.github/renovate.json` using the
`config:recommended` preset. This handles Cargo dependency updates and GitHub Actions version
bumps.

---

## 8. Release Process

**There is no formal release process.** The project operates on a continuous-delivery model:

- Pushes to `main` trigger CI builds
- Built artifacts are stored in the S3 registry
- No versioning, tagging, or release branches exist
- The `Cargo.toml` version (`0.1.0-rc.0`) appears to be a placeholder

---

## 9. Monitoring and Observability

**There is no monitoring or observability infrastructure.** This is expected given the project's
nature as a build-definition repository rather than a running service. Operational visibility
comes from:

- **GitHub Actions UI** for build status and logs
- **GitHub Actions artifacts** for lock file inspection
- **S3 bucket** for registry contents (no documented access pattern)

---

## 10. Rollback Procedures

**There is no formal rollback procedure.** Given the project's nature:

- **Code rollback:** Standard `git revert` on `main` would trigger a new CI build
- **Artifact rollback:** The S3 registry retains previous builds, but there is no documented
  mechanism to roll back to a prior artifact version
- **Vorpal CLI rollback:** CI uses `version: nightly`, so there is no pinned Vorpal version
  to roll back to

---

## 11. Identified Gaps

The following operational gaps exist. These are documented for awareness, not as a requirement
to address immediately:

1. **No Makefile:** CLAUDE.md references `make lima` and `make lima-vorpal` commands, but no
   `Makefile` exists in the repository.
2. **Nightly Vorpal dependency:** CI uses `version: nightly` for the Vorpal CLI, meaning builds
   can break from upstream Vorpal changes without any code change in this repository.
3. **Git-branch SDK dependency:** `vorpal-sdk` is pinned to `main` branch via git URL, not a
   versioned release. Breaking changes upstream will propagate silently until `cargo build`
   fails.
4. **No artifact versioning strategy:** Individual artifacts embed version numbers in their
   source code (e.g., `jj` at `0.37.0`), but there is no systematic process for updating them
   beyond manual edits and Renovate.
5. **No build status notifications:** Failed CI builds only surface through the GitHub Actions
   UI. There are no Slack, email, or other notification integrations.
6. **No artifact registry cleanup:** No retention policy or garbage collection mechanism is
   documented for the S3 registry bucket.
7. **No cross-artifact dependency tracking in CI:** If `ncurses` changes, dependent artifacts
   (`tmux`, `nnn`, `readline`, `zsh`) are not automatically rebuilt. The detection script
   intentionally only rebuilds artifacts whose own `.rs` files changed.
