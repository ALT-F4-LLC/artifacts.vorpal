# Operations Specification

> Project: `artifacts.vorpal` (ALT-F4-LLC)
> Last updated: 2026-02-22

This document describes the operational characteristics of the `artifacts.vorpal` project based on
what actually exists in the codebase. It covers CI/CD, deployment, environment management, artifact
registry, and operational tooling.

---

## 1. Project Overview

`artifacts.vorpal` is a Rust-based artifact definition repository that uses the
[Vorpal](https://github.com/ALT-F4-LLC/vorpal) build system to define, build, and publish
software artifacts (CLI tools, libraries, and runtime components) across four target platforms:

- `aarch64-darwin` (macOS ARM)
- `x86_64-darwin` (macOS Intel)
- `aarch64-linux` (Linux ARM)
- `x86_64-linux` (Linux x86)

The project currently defines **55+ artifacts** including tools like `bat`, `ripgrep`, `kubectl`,
`terraform`, `neovim`, `tmux`, `gpg`, `nginx`, `ffmpeg`, and many others. Each artifact is
defined as a Rust module in `src/artifact/` that describes how to fetch, build, and package the
software.

---

## 2. CI/CD Pipeline

### 2.1 Workflow Configuration

**File**: `.github/workflows/vorpal.yaml`

The project uses a single GitHub Actions workflow (`vorpal`) triggered on:

- **Pull requests**: All PRs
- **Pushes to `main`**: Post-merge builds

### 2.2 Pipeline Stages

The pipeline has three jobs executed in sequence:

#### Job 1: `build-changes` (Change Detection)

- **Runner**: `ubuntu-latest`
- **Purpose**: Determines which artifacts have changed between commits using
  `script/detect-changed-artifacts.sh`
- **Logic**:
  - On PRs: compares PR base SHA to head SHA
  - On pushes: compares `github.event.before` to current SHA
  - Falls back to building all artifacts when no base SHA is available (e.g., initial push)
- **Outputs**: `artifacts` (JSON array of changed artifact names) and `has_changes` (boolean)

The change detection script (`script/detect-changed-artifacts.sh`) dynamically discovers artifacts
by scanning `src/artifact/*.rs` files. It only triggers rebuilds for artifacts whose source files
have changed -- changes to shared files like `src/vorpal.rs`, `Cargo.lock`, or workflow files do
**not** trigger a full rebuild of all artifacts.

#### Job 2: `build-dev` (Development Environment)

- **Runners**: Matrix across all four platform runners:
  - `macos-latest` (aarch64-darwin)
  - `macos-latest-large` (x86_64-darwin)
  - `ubuntu-latest` (x86_64-linux)
  - `ubuntu-latest-arm64` (aarch64-linux)
- **Steps**:
  1. Checkout repository
  2. Set up Vorpal via `ALT-F4-LLC/setup-vorpal-action@main` with S3 registry backend
  3. Run `vorpal build 'dev'` to build the development environment
  4. Upload `Vorpal.lock` as a GitHub Actions artifact (named by arch/OS)
- **Purpose**: Validates the development environment builds on all platforms

#### Job 3: `build` (Artifact Builds)

- **Condition**: Only runs if `build-changes` detected changed artifacts (`has_changes == 'true'`)
- **Dependencies**: Requires both `build-dev` and `build-changes` to complete
- **Strategy**: Matrix of changed artifacts x platform runners, with `fail-fast: false`
  (individual artifact failures do not cancel other builds)
- **Steps**: Same Vorpal setup, then `vorpal build '<artifact_name>'` for each changed artifact

### 2.3 Vorpal Setup Action

The pipeline uses `ALT-F4-LLC/setup-vorpal-action@main` with:

- **Version**: `nightly`
- **Registry backend**: S3
- **S3 bucket**: `altf4llc-vorpal-registry`
- **AWS credentials**: Provided via GitHub Secrets (`AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`)
  and Variables (`AWS_DEFAULT_REGION`)

---

## 3. Artifact Registry

### 3.1 Storage Backend

Built artifacts are stored in an **AWS S3 bucket** (`altf4llc-vorpal-registry`). This serves as
the Vorpal artifact registry where built artifacts are cached and retrieved.

### 3.2 Lock File

The `Vorpal.lock` file records the source digest (SHA-256 hash) and download URL for every source
dependency across all platforms. This provides:

- **Reproducibility**: Exact source versions are pinned by content hash
- **Integrity verification**: Downloaded sources are validated against recorded digests
- **Platform awareness**: Each source entry includes a `platform` field

The lock file is uploaded as a CI artifact from `build-dev` for each platform runner.

### 3.3 Source Configuration

`Vorpal.toml` defines the project metadata:

```toml
language = "rust"

[source]
includes = ["src", "Cargo.toml", "Cargo.lock"]
```

This tells Vorpal which files constitute the project source for build purposes.

---

## 4. Development Environment

### 4.1 Local Development

- **direnv**: The project uses `.envrc` for automatic environment variable loading (direnv
  integration)
- **Rust toolchain**: Defined via `vorpal-sdk` dependency; the development environment
  (`ProjectEnvironment`) configures `RUSTUP_HOME`, `RUSTUP_TOOLCHAIN`, and `PATH` to include
  the Rust toolchain binaries

### 4.2 Lima Virtual Machines (Linux Testing on macOS)

The project includes Lima VM support for testing Linux builds from macOS:

**File**: `lima.yaml`

- **Base images**: Debian 13 (Trixie) cloud images for both `amd64` and `aarch64`
- **Mount type**: 9p unsupported, uses default

**File**: `makefile`

Lima management targets:

| Target | Description |
|---|---|
| `lima` | Creates and provisions a Lima VM (installs deps, Vorpal) |
| `lima-clean` | Stops and deletes the Lima VM |
| `lima-sync` | Syncs source code into the VM |
| `lima-vorpal` | Runs `vorpal build` inside the VM |
| `lima-vorpal-start` | Starts Vorpal services inside the VM |

Default VM configuration: 8 CPUs, 8GB RAM, 100GB disk.

**File**: `script/lima.sh`

Provisioning script that installs:

- Build essentials, curl, jq, rsync, wget, unzip, ca-certificates
- Bubblewrap (sandbox runtime)
- Docker
- Vorpal (via upstream install script)
- AppArmor profile for bubblewrap (if AppArmor is detected)

---

## 5. Dependency Management

### 5.1 Rust Dependencies

Managed via `Cargo.toml` and `Cargo.lock`:

- `anyhow` - Error handling
- `indoc` - Indented string formatting for build scripts
- `tokio` - Async runtime (multi-threaded)
- `vorpal-sdk` - Vorpal build system SDK (from Git, `main` branch)

### 5.2 Automated Dependency Updates

**File**: `.github/renovate.json`

Renovate Bot is configured with the `config:recommended` preset. This provides automated PRs for
dependency updates across the project.

### 5.3 Artifact Sources

Each artifact module defines its own upstream source URLs and versions. Sources are fetched as
tarballs or binaries from upstream release pages (GitHub Releases, project websites, etc.) and
their integrity is tracked in `Vorpal.lock` via SHA-256 digests.

---

## 6. Operational Scripts

### 6.1 Change Detection

**File**: `script/detect-changed-artifacts.sh`

- Dynamically discovers artifacts from `src/artifact/*.rs` filenames
- Converts filenames to artifact names (underscores to hyphens)
- Excludes utility files (currently `file.rs`)
- Supports `--all` (JSON), `--list` (plain text), and commit range comparison modes
- Uses `git diff --name-only --diff-filter=d` to detect changes (excludes deleted files)

### 6.2 Linux Rootfs Slimming

**File**: `script/linux-vorpal-slim.sh`

A comprehensive rootfs slimming script that reduces Vorpal Linux installations from ~2.9GB to
~600-700MB. Features:

- 13 configurable removal sections (GCC, dev tools, Python, Perl, static libs, headers,
  sanitizers, docs, locales, i18n, build artifacts, optional cleanup)
- Dry-run mode (default) with size estimation
- Backup creation support
- Protected files list (critical runtime libraries and binaries)
- Post-removal verification of essential files
- Aggressive mode for binary stripping

### 6.3 Change Detection Tests

**File**: `script/test-detect-changed-artifacts.sh`

Regression tests for the change detection script, covering:

- Basic listing and JSON output
- Empty diff handling
- Regression: non-artifact file changes do not trigger full rebuild
- Regression: deleted artifact files are excluded from build list
- Regression: no hardcoded artifact mechanisms (CORE_FILES, DEPENDENTS removed)

---

## 7. Release & Versioning

### 7.1 Project Version

The project is at version `0.1.0-rc.0` (release candidate), as defined in `Cargo.toml`.

### 7.2 Artifact Versioning

Each artifact tracks its own upstream version independently. Artifacts register aliases in the
format `<name>:<version>` (e.g., `bat:0.25.0`, `zlib:1.3.2`). Version bumps are made by updating
the version constant in the corresponding `src/artifact/<name>.rs` file.

### 7.3 Release Process

**No formal release process exists.** There are no:

- Tagged releases
- Changelog generation
- Release branches
- Published binaries or packages

The project operates on a trunk-based development model where `main` is the primary branch and
CI builds artifacts on every push.

---

## 8. Monitoring & Observability

### 8.1 Current State

**There is no monitoring, logging, or observability infrastructure in this project.** Specifically:

- No structured logging framework
- No metrics collection or dashboards
- No distributed tracing
- No alerting
- No health checks
- No error tracking or reporting service

### 8.2 Build Visibility

The only operational visibility comes from:

- **GitHub Actions workflow runs**: Build success/failure per artifact per platform
- **GitHub Actions matrix view**: Shows which specific artifact x platform combinations failed
- **Vorpal.lock diffs**: Show when source versions or digests change

---

## 9. Rollback Procedures

### 9.1 Current State

**No formal rollback procedures exist.** Since this is an artifact definition repository (not a
deployed service), rollback means:

1. **Git revert**: Revert the commit that introduced the problematic artifact change
2. **Re-run CI**: Push the revert to `main` to trigger a rebuild
3. **Registry state**: Built artifacts in the S3 registry are keyed by content hash, so previous
   versions should remain available as long as the registry has not been pruned

### 9.2 Risks

- The Vorpal SDK dependency tracks `main` branch via Git, meaning SDK changes could break builds
  without any change in this repository
- No pinned Vorpal action version (uses `@main`)
- No mechanism to roll back the S3 registry contents independently of Git history

---

## 10. Security Operations

### 10.1 Secrets Management

- **AWS credentials**: Stored as GitHub Secrets (`AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`)
- **AWS region**: Stored as a GitHub Variable (`AWS_DEFAULT_REGION`)
- No other secrets are used in the codebase

### 10.2 Supply Chain

- Source integrity is verified via SHA-256 digests in `Vorpal.lock`
- Renovate Bot provides automated dependency update PRs
- The Vorpal SDK is fetched from Git (`main` branch) without a pinned commit or tag
- The setup action (`ALT-F4-LLC/setup-vorpal-action@main`) is not pinned to a specific version

---

## 11. Infrastructure

### 11.1 Compute

All compute is provided by GitHub Actions runners. No self-hosted runners or additional
infrastructure is used for CI/CD.

| Runner | Platform | Architecture |
|---|---|---|
| `macos-latest` | macOS | aarch64 |
| `macos-latest-large` | macOS | x86_64 |
| `ubuntu-latest` | Linux | x86_64 |
| `ubuntu-latest-arm64` | Linux | aarch64 |

### 11.2 Storage

- **S3 bucket** (`altf4llc-vorpal-registry`): Artifact registry storage
- **GitHub Actions artifacts**: Temporary storage for lock files between jobs

### 11.3 No Additional Infrastructure

There are no:

- Databases
- Container registries (though Docker is installed in Lima VMs)
- Load balancers or CDNs
- DNS or domain management
- Kubernetes clusters or container orchestration

---

## 12. Known Gaps

| Gap | Risk | Impact |
|---|---|---|
| No pinned Vorpal SDK version | High | SDK `main` branch changes can break builds without warning |
| No pinned setup action version | Medium | Action changes could alter build behavior |
| No monitoring or alerting | Low | Relies entirely on checking GitHub Actions manually |
| No formal release process | Medium | No versioned releases, changelogs, or upgrade paths for consumers |
| No rollback automation | Low | Manual git revert + CI re-run required |
| No build notifications | Low | No Slack/email notifications on build failures |
| No artifact retention policy | Medium | S3 registry may grow unbounded without pruning |
| No cost monitoring | Low | S3 storage and GitHub Actions usage are not tracked |
| Shared infrastructure changes (e.g., Vorpal SDK breaking changes) not detected proactively | High | Builds can break due to upstream changes in dependencies not tracked in this repo |
