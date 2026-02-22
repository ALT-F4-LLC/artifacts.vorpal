# Security Specification

This document describes the security model, trust boundaries, secret management, and threat
considerations for the `artifacts.vorpal` project as they exist today.

## Project Overview

`artifacts.vorpal` is a Rust-based artifact definition repository that uses the Vorpal SDK to
define, build, and publish software packages (CLI tools, libraries, and development environments)
across four target platforms: `aarch64-darwin`, `aarch64-linux`, `x86_64-darwin`, and
`x86_64-linux`. It is a build-time system -- it does not run services, handle user authentication,
or process untrusted user input at runtime.

## Trust Boundaries

### Boundary 1: Upstream Source Downloads

Every artifact definition fetches source code or pre-built binaries from external URLs over HTTPS.
These URLs point to:

- **GitHub Releases** (e.g., argocd, bat, bottom, crane, doppler, fd, fluxcd, helm, jj, jq, k9s,
  kn, kubectl, kubeseal, lazygit, lima, neovim, ripgrep, skopeo, starship, ttyd, umoci, vhs, yq)
- **Official project download servers** (e.g., ffmpeg.org, gnupg.org, ftpmirror.gnu.org,
  awscli.amazonaws.com, releases.hashicorp.com, repo1.maven.org, sqlite.org, invisible-island.net,
  github.com/libevent, github.com/libuv, github.com/warmcat, github.com/Mbed-TLS)

**Current integrity verification:** The `Vorpal.lock` file records a SHA-256 `digest` for every
downloaded source (336 entries across all platforms). The Vorpal SDK is responsible for verifying
these digests at download time. This is the primary supply chain integrity mechanism.

**What is NOT verified at the artifact definition level:**
- GPG/PGP signature verification of upstream sources
- Certificate pinning on download URLs
- SBOM (Software Bill of Materials) generation

All URLs use HTTPS, which provides transport-level authenticity and integrity.

### Boundary 2: Build Script Execution

Each artifact's `build()` method generates a shell script that is executed by the Vorpal SDK's
`step::shell()` function. These scripts run with access to:

- `$VORPAL_OUTPUT` -- the designated output directory for the built artifact
- `./source/<name>/` -- the downloaded and extracted source files
- Environment variables injected via `get_env_key()` for dependency paths
- Standard build tools available on the host or sandbox

**Build isolation is delegated to the Vorpal SDK.** On Linux, the Vorpal build system uses
`bubblewrap` (bwrap) for sandboxed builds. The Lima VM setup script (`script/lima.sh`) installs
bubblewrap and configures an AppArmor profile for it. On macOS, the sandboxing mechanism is
determined by the SDK (not configured in this repository).

**Shell script safety:** Only `src/artifact/file.rs` uses `set -euo pipefail`. Other artifact
build scripts rely on the Vorpal SDK's `step::shell()` to configure the shell execution
environment. The scripts themselves are static templates with compile-time string interpolation --
they do not process untrusted input at build time.

### Boundary 3: CI/CD Pipeline

The GitHub Actions workflow (`.github/workflows/vorpal.yaml`) is the primary automation boundary.

**Secrets used:**
- `secrets.AWS_ACCESS_KEY_ID` -- AWS credential for S3 registry access
- `secrets.AWS_SECRET_ACCESS_KEY` -- AWS credential for S3 registry access
- `vars.AWS_DEFAULT_REGION` -- AWS region (stored as a variable, not a secret)

These are passed as environment variables to the `ALT-F4-LLC/setup-vorpal-action@main` step,
which configures Vorpal with an S3 backend registry at `altf4llc-vorpal-registry`.

**Workflow trigger scope:** The workflow runs on `pull_request` events and `push` to `main`. There
is no restriction on which branches can trigger PR builds.

### Boundary 4: Development Environment

The `ProjectEnvironment` (`src/lib.rs`) configures a development environment with:
- Lima (VM management)
- Protoc (Protocol Buffers compiler)
- Rust toolchain

Environment variables set: `PATH`, `RUSTUP_HOME`, `RUSTUP_TOOLCHAIN`. No secrets or credentials
are injected into the development environment configuration.

## Secret Management

### Secrets in Use

| Secret | Location | Purpose |
|---|---|---|
| `AWS_ACCESS_KEY_ID` | GitHub Actions secrets | Authenticate to S3 artifact registry |
| `AWS_SECRET_ACCESS_KEY` | GitHub Actions secrets | Authenticate to S3 artifact registry |
| `AWS_DEFAULT_REGION` | GitHub Actions variables | Configure AWS region |

### Secret Handling Assessment

- Secrets are stored in GitHub's encrypted secrets store and only exposed as environment variables
  during CI runs. This follows GitHub's recommended pattern.
- No secrets are hardcoded in source code. A search for `secret`, `token`, `password`,
  `credential`, `api_key`, and `auth` across all Rust source files returned no matches related
  to credential handling.
- No `.env` files exist in the repository.
- The `.gitignore` excludes `/.docket` and `/target` but does not explicitly exclude `.env` files
  or other common secret file patterns.
- The `.envrc` file (for direnv) exists but could not be inspected due to permissions. Based on
  the project structure, it likely configures the local Vorpal development environment, not
  secrets.

## Supply Chain Security

### Source Integrity

The lockfile-based integrity model (`Vorpal.lock`) provides:

1. **Reproducible builds**: Every source URL is pinned to a specific version and its SHA-256 digest
   is recorded.
2. **Tamper detection**: If an upstream source changes (e.g., a re-tagged release), the digest
   mismatch will cause the build to fail.
3. **Version pinning**: All artifact versions are hardcoded in their respective Rust source files
   (e.g., `let source_version = "3.2.3"` in `argocd.rs`).

### Dependency Management

**Rust dependencies** are managed via `Cargo.lock`, which pins exact versions. The project has a
minimal dependency footprint:
- `anyhow` -- error handling
- `indoc` -- indented string formatting
- `tokio` -- async runtime
- `vorpal-sdk` -- the core build SDK (pinned to `main` branch of the ALT-F4-LLC/vorpal repository)

**Notable:** The `vorpal-sdk` dependency tracks the `main` branch via Git, not a tagged release.
This means the SDK version used in builds can change without explicit action in this repository.
This is a supply chain risk -- a compromised or breaking change in the SDK's `main` branch would
affect all builds.

**Renovate** is configured (`.github/renovate.json`) with the recommended preset for automated
dependency update PRs. This helps keep dependencies current with security patches.

### Third-Party Actions

The CI workflow uses:
- `actions/checkout@v6` -- official GitHub action, version-pinned by major version
- `ALT-F4-LLC/setup-vorpal-action@main` -- organization-owned action, pinned to `main` branch
- `actions/upload-artifact@v6` -- official GitHub action, version-pinned by major version

**Risk:** Both `ALT-F4-LLC/setup-vorpal-action@main` and the vorpal-sdk Git dependency track
`main` branches rather than tagged releases or commit SHAs. This creates a mutable reference that
could change without notice.

## CI/CD Security

### Workflow Configuration

- **No explicit permissions block**: The workflow does not define `permissions:` at the workflow
  or job level. This means jobs run with the default token permissions for the repository, which
  may be broader than necessary.
- **Matrix injection risk**: The `build` job uses `${{ matrix.artifact }}` in a `run` step
  (`vorpal build '${{ matrix.artifact }}'`). The artifact names are derived from
  `needs.build-changes.outputs.artifacts`, which is computed by `script/detect-changed-artifacts.sh`
  from filenames in `src/artifact/`. Since artifact names are derived from filesystem paths in the
  repository (not from user-controlled PR titles or branch names), the injection risk is low.
- **S3 bucket access**: The `altf4llc-vorpal-registry` S3 bucket is accessible to all CI jobs
  with the same credentials. There is no distinction between read and write access at the
  workflow level.

### Build Artifact Handling

- The workflow uploads `Vorpal.lock` as a GitHub Actions artifact for each platform/architecture
  combination. This file contains digests but no secrets.
- Built artifacts are stored in the S3 registry via the Vorpal backend, not as GitHub Actions
  artifacts.

## Build Script Security

### Shell Script Patterns

The artifact build scripts follow a consistent pattern:
1. Create output directory (`mkdir -pv "$VORPAL_OUTPUT/bin"`)
2. Navigate to extracted source (`pushd ./source/<name>/...`)
3. Build or copy the artifact
4. Install to `$VORPAL_OUTPUT`

**Strengths:**
- Variables are consistently double-quoted (`"$VORPAL_OUTPUT"`) in most scripts, preventing word
  splitting and glob expansion issues.
- No network access during build steps (sources are pre-downloaded).
- Build scripts are static templates -- no dynamic input from users or PRs.

**Weaknesses:**
- Only `file.rs` explicitly sets `set -euo pipefail`. Other scripts rely on the SDK to configure
  shell behavior. If the SDK does not set `set -e`, build errors in intermediate commands could be
  silently ignored.
- Some scripts use `$VORPAL_OUTPUT` without quotes (e.g., `nginx.rs` line 34:
  `--prefix=$VORPAL_OUTPUT`, line 40: `ln -svf $VORPAL_OUTPUT/sbin/nginx $VORPAL_OUTPUT/bin/nginx`;
  `file.rs` line 29: `$VORPAL_OUTPUT/{name}`; `pkg_config.rs` line 33:
  `--prefix=$VORPAL_OUTPUT`). Since `VORPAL_OUTPUT` is set by the SDK and unlikely to contain
  spaces or special characters, this is low-risk but inconsistent.

### Lima VM Setup Script

`script/lima.sh` performs privileged operations:
- Runs `sudo apt-get` for package installation
- Downloads and executes Docker's install script from `https://get.docker.com`
- Downloads and executes the Vorpal install script from GitHub
- Modifies AppArmor profiles with `sudo`

This script is intended for VM provisioning only, not for use in production environments. The use
of `curl -fsSL | sh` for Docker installation is a common but inherently trust-on-first-use
pattern.

### Linux Slimming Script

`script/linux-vorpal-slim.sh` is a rootfs slimming tool that:
- Operates on a specified rootfs path
- Has a protected list of runtime libraries that must never be removed
- Defaults to dry-run mode (`DRY_RUN="yes"`)
- Requires explicit confirmation unless `--no-confirm` is passed

The dry-run default is a good safety measure against accidental data loss.

## Identified Gaps and Recommendations

### Current Gaps

1. **No GPG signature verification**: Upstream source authenticity relies solely on HTTPS transport
   security and SHA-256 digest matching. Sources from projects that provide GPG signatures (GnuPG,
   GNU projects) are not signature-verified.

2. **Mutable dependency references**: Both `vorpal-sdk` (Cargo.toml) and
   `setup-vorpal-action` (workflow) reference `main` branches instead of pinned
   commits or tags.

3. **No workflow permissions scoping**: The GitHub Actions workflow does not restrict token
   permissions, potentially granting broader access than needed.

4. **No `.env` exclusion in `.gitignore`**: While no `.env` files exist, the `.gitignore` does not
   proactively exclude them, increasing the risk of accidental secret commits.

5. **No CODEOWNERS file**: There is no `CODEOWNERS` file to enforce review requirements for
   security-sensitive paths (e.g., `.github/workflows/`, `script/`).

6. **No branch protection documentation**: Whether branch protection rules (required reviews,
   status checks) are configured on `main` is not captured in the repository.

### Low-Priority Observations

- The `${{ matrix.artifact }}` expression in the CI workflow is safe given the current artifact
  name derivation from filesystem paths, but wrapping it in an intermediate environment variable
  would be a defense-in-depth improvement.
- The Renovate configuration uses the default recommended preset. Adding automerge rules for
  patch-level security updates could reduce the window of vulnerability exposure.

## Threat Model Summary

| Threat | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Compromised upstream source | Low | High | SHA-256 digest verification via Vorpal.lock |
| Compromised vorpal-sdk main branch | Low | Critical | None -- tracks mutable branch reference |
| AWS credential leak from CI | Low | High | GitHub encrypted secrets, not hardcoded |
| Malicious PR modifying build scripts | Low | Medium | Code review (no automated enforcement documented) |
| Supply chain attack via GitHub Actions | Low | High | First-party and org-owned actions only |
| Accidental secret commit | Low | Medium | No `.env` files exist; `.gitignore` could be stricter |
