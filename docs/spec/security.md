# Security Specification

This document describes the security model, trust boundaries, secret management, and threat
considerations for the `vorpal-artifacts` project as they actually exist in the codebase today.

---

## 1. Project Security Profile

This project is an **artifact definition repository** -- it declares how software packages are
downloaded and built via the Vorpal build system. It does not serve user-facing traffic, handle
user authentication, or store sensitive user data at runtime. Its primary security concerns are:

- **Supply chain integrity**: Ensuring downloaded artifacts are genuine and untampered.
- **CI/CD credential management**: Protecting cloud credentials used during automated builds.
- **Build-time execution safety**: Shell scripts run during artifact builds must not introduce
  vulnerabilities.

---

## 2. Trust Boundaries

### 2.1 Upstream Source Origins

Every artifact downloads source code or pre-built binaries from external URLs. The project trusts
the following categories of upstream sources:

| Source Category | Examples | Count |
|---|---|---|
| GitHub Releases | `github.com/*/releases/download/*` | ~30 artifacts |
| Official project sites | `gnupg.org/ftp/gcrypt/*`, `dl.k8s.io/*`, `releases.hashicorp.com/*` | ~5 artifacts |
| Package mirrors | `ftpmirror.gnu.org/*`, `invisible-mirror.net/*`, `downloads.sourceforge.net/*` | ~3 artifacts |
| Cloud vendor CDNs | `awscli.amazonaws.com/*`, `download.java.net/*` | ~2 artifacts |
| Package repositories | `repo1.maven.org/*` | ~1 artifact |

**Current state**: All source URLs use HTTPS. No HTTP (plaintext) URLs exist in the codebase.
This provides transport-layer encryption and basic server authentication via TLS certificates.

### 2.2 Vorpal SDK

The project depends on `vorpal-sdk` (sourced from `https://github.com/ALT-F4-LLC/vorpal.git`,
`main` branch) for all build orchestration, including `ArtifactSource`, `ConfigContext`,
`step::shell`, and the `Artifact` builder. The SDK is an implicit trust boundary -- this project
trusts whatever download, extraction, sandboxing, and caching behavior the SDK implements.

### 2.3 CI/CD Environment

GitHub Actions runners (`ubuntu-latest`, `macos-latest`, and their architecture variants) are
trusted execution environments. The `ALT-F4-LLC/setup-vorpal-action@main` action is referenced
by branch (`main`), not by commit SHA or tag, which means changes to that action are implicitly
trusted.

### 2.4 Renovate Bot

Renovate (`config:recommended`) is configured to automatically propose dependency updates via
pull requests. This introduces a trust relationship with the Renovate service for dependency
version bumps in `Cargo.toml`.

---

## 3. Source Integrity Verification

### 3.1 Vorpal.lock Digest Mechanism

The `Vorpal.lock` file contains SHA-256 digests for every source artifact, per platform. Example:

```toml
[[sources]]
name = "argocd"
path = "https://github.com/argoproj/argo-cd/releases/download/v3.2.3/argocd-darwin-amd64"
digest = "1ef724533580a0011ff20f172119065214470aa7fca80eb10c9e11a26489f6cc"
platform = "x86_64-darwin"
```

There are approximately 301 digest entries in `Vorpal.lock`, covering all artifacts across all
four target platforms. This lockfile is checked into version control, providing a verifiable
record of expected source content.

**How verification works**: The Vorpal SDK (not this repository's code) is responsible for
comparing downloaded content against these digests. The artifact definitions in `src/artifact/*.rs`
do **not** contain inline hashes -- they only specify URLs. The lockfile is generated and enforced
at the SDK/runtime level.

**Implication**: Digest verification depends entirely on the Vorpal SDK's implementation.
This repository cannot independently verify that digests are checked before build scripts execute.

### 3.2 What Is NOT Present

- **No GPG signature verification** of downloaded sources (despite building GPG itself as an
  artifact).
- **No checksum URLs** (e.g., `.sha256` sidecar files) are fetched or compared.
- **No SLSA provenance** or Sigstore verification for any artifact.
- **No Software Bill of Materials (SBOM)** generation.
- **No pinned commit SHAs** for source archives fetched from GitHub archive URLs (e.g.,
  `archive/refs/tags/v*.tar.gz`). Tag mutability means these could theoretically change.

---

## 4. Secret Management

### 4.1 CI/CD Secrets

The GitHub Actions workflow (`.github/workflows/vorpal.yaml`) uses the following secrets and
variables:

| Name | Type | Purpose | Scope |
|---|---|---|---|
| `AWS_ACCESS_KEY_ID` | `secrets.*` | S3 registry authentication | CI only |
| `AWS_SECRET_ACCESS_KEY` | `secrets.*` | S3 registry authentication | CI only |
| `AWS_DEFAULT_REGION` | `vars.*` | S3 registry region | CI only |

These credentials are passed as environment variables to the `ALT-F4-LLC/setup-vorpal-action`
step and are used to authenticate with an S3 bucket (`altf4llc-vorpal-registry`) serving as the
Vorpal artifact registry backend.

**Exposure surface**: The secrets are scoped to the CI workflow and are not referenced anywhere
in the Rust source code. They are available to the `vorpal build` command at runtime within CI,
which means any artifact's build script technically executes in an environment where these
variables exist in the process tree.

### 4.2 Local Development

- **No `.env` files** exist in the repository.
- **No credential files** (`.json`, `.pem`, `.key`) are present.
- **`.gitignore`** only excludes the `/target` directory. There is no explicit exclusion for
  common secret file patterns (`.env`, `*.pem`, `*.key`, `credentials.*`).
- The `Vorpal.toml` source includes (`src`, `Cargo.toml`, `Cargo.lock`) are narrowly scoped,
  which limits what gets packaged, but the `.gitignore` gap means secrets could accidentally be
  committed.

### 4.3 Runtime Secrets

The project itself does not handle runtime secrets. However, some of the artifacts it builds are
secret management tools:

- **Doppler CLI** (`src/artifact/doppler.rs`) -- a secrets management tool
- **GPG** (`src/artifact/gpg.rs`) -- cryptographic key management
- **Kubeseal** (`src/artifact/kubeseal.rs`) -- Kubernetes secret encryption
- **AWS CLI** (`src/artifact/awscli2.rs`) -- cloud credential management

These tools handle secrets at the user level after installation, not during the build process
defined here.

---

## 5. Build Script Security

### 5.1 Shell Script Execution Model

Every artifact defines a shell build script (via `step::shell()`) that runs during `vorpal build`.
These scripts perform operations like:

- Extracting archives (`tar`, `unzip`, `pkgutil`)
- Copying binaries to `$VORPAL_OUTPUT`
- Running `./configure && make && make install` for source builds
- Setting `chmod +x` on executables

### 5.2 Script Injection Risk

Build scripts are constructed using Rust's `formatdoc!` macro with string interpolation. The
interpolated values come from:

1. **Hardcoded version strings** (e.g., `source_version = "0.37.0"`) -- safe
2. **Platform match results** (e.g., `"aarch64-apple-darwin"`) -- safe
3. **Artifact key references** via `get_env_key()` -- values are Vorpal-internal identifiers

There is **no user input** flowing into build scripts. All values are compile-time constants or
SDK-generated identifiers. The injection risk is minimal given the current architecture.

### 5.3 Build Environment Isolation

Build isolation is delegated to the Vorpal SDK. The `step::shell()` function signature accepts
artifact dependencies and environment variables, suggesting the SDK provides some form of
sandboxing (potentially using `bubblewrap`, given `lima.sh` installs it). The exact isolation
guarantees are defined by the SDK, not this repository.

The Lima VM script (`script/lima.sh`) installs `bubblewrap` and configures an AppArmor profile
for it on Linux, which suggests Vorpal uses `bwrap` for build sandboxing on Linux targets.

---

## 6. Dependency Security

### 6.1 Rust Dependencies

Direct dependencies from `Cargo.toml`:

| Dependency | Version | Security Relevance |
|---|---|---|
| `anyhow` | `1` | Error handling; no security surface |
| `indoc` | `2` | String formatting macro; compile-time only |
| `tokio` | `1` (rt-multi-thread) | Async runtime; well-audited |
| `vorpal-sdk` | Git branch `main` | **Critical**: Handles all download, verification, and sandboxing |

**Concern**: `vorpal-sdk` is pinned to a **branch** (`main`), not a specific commit or version.
This means any push to the `vorpal` repository's `main` branch will be picked up on the next
`cargo update`. The `Cargo.lock` file pins the actual resolved commit, providing reproducibility
for a given lock state, but the `Cargo.toml` declaration allows unreviewed SDK changes to flow in.

### 6.2 Transitive Dependencies

The `Cargo.lock` contains the full set of transitive dependencies. These are standard Rust
ecosystem crates (tokio, tonic/gRPC stack, prost for protobuf, etc.). No known security concerns
beyond standard supply chain risk.

---

## 7. CI/CD Security

### 7.1 GitHub Actions Workflow

**Strengths**:
- Secrets are stored in GitHub's encrypted secrets store, not in code.
- The workflow uses `actions/checkout@v6` (version-tagged, not branch-pinned).
- `actions/upload-artifact@v6` is used for build outputs.
- Matrix builds across 4 runner types provide cross-platform verification.

**Concerns**:
- `ALT-F4-LLC/setup-vorpal-action@main` is pinned to `main` branch, not a commit SHA. A
  compromise of this action repository would affect all CI builds.
- The `vorpal build '${{ matrix.artifact }}'` step interpolates the artifact name from the matrix.
  The artifact names are derived from the `detect-changed-artifacts.sh` script output. Since this
  script reads filenames from `src/artifact/*.rs` and converts them to kebab-case, a maliciously
  named artifact file (e.g., one containing shell metacharacters) could theoretically inject into
  the `vorpal build` command. However, Rust module names are restricted to `[a-z0-9_]`, making
  this practically infeasible.
- The `detect-changed-artifacts.sh` script uses `set -euo pipefail` and properly quotes
  variables, reducing shell injection risk.

### 7.2 Artifact Detection Script Security

The `script/detect-changed-artifacts.sh` script:
- Uses `set -euo pipefail` for strict error handling.
- Reads only from `src/artifact/*.rs` filenames (controlled by the repository).
- Uses `git diff --diff-filter=d` to exclude deleted files (preventing stale artifact references).
- Converts filenames via `tr '_' '-'` (safe transformation).

---

## 8. Identified Gaps and Recommendations

### 8.1 High Priority

| Gap | Description | Risk |
|---|---|---|
| **No `.gitignore` for secrets** | Common secret file patterns (`.env`, `*.pem`, `*.key`) are not excluded | Accidental secret commit |
| **Branch-pinned SDK** | `vorpal-sdk` in `Cargo.toml` tracks `main` branch | Unreviewed dependency changes |
| **Branch-pinned CI action** | `setup-vorpal-action@main` is not SHA-pinned | CI supply chain risk |

### 8.2 Medium Priority

| Gap | Description | Risk |
|---|---|---|
| **No signature verification** | Downloaded sources are not GPG-verified | Compromised mirrors |
| **No SBOM generation** | No software bill of materials for built artifacts | Compliance, auditability |
| **Opaque SDK trust** | Digest verification behavior is not visible in this repo | Unknown verification gaps |

### 8.3 Low Priority

| Gap | Description | Risk |
|---|---|---|
| **No Dependabot/audit CI step** | No `cargo audit` in the CI pipeline | Known vulnerability detection |
| **Tag mutability** | GitHub archive URLs use tags that could theoretically be force-pushed | Source tampering (mitigated by lockfile digests) |

---

## 9. Security-Sensitive Artifacts

The following artifacts have elevated security relevance because they handle cryptographic
operations, secrets, or privileged access in the environments where they are deployed:

| Artifact | Why It Matters |
|---|---|
| `gpg` + libraries (`libgcrypt`, `libgpg_error`, `libassuan`, `libksba`, `npth`) | Cryptographic operations, key management |
| `kubeseal` | Encrypts Kubernetes secrets |
| `doppler` | Secrets management platform CLI |
| `awscli2` | Cloud infrastructure access with IAM credentials |
| `terraform` | Infrastructure-as-code with cloud provider credentials |
| `kubectl` | Kubernetes cluster administration |
| `helm` | Kubernetes package manager with cluster access |
| `argocd` | GitOps continuous delivery with cluster access |
| `fluxcd` | GitOps toolkit with cluster access |

These artifacts are downloaded as pre-built binaries (except GPG and its libraries, which are
compiled from source). Their integrity depends entirely on the HTTPS transport security and the
Vorpal lockfile digest verification.
