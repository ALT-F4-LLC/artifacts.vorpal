# Code Quality Specification

## Overview

This document describes the coding standards, naming conventions, error handling patterns, design
patterns, and project-specific style decisions observed in the `artifacts.vorpal` codebase. It
reflects what actually exists in the code today.

The project is a Rust binary (`vorpal-artifacts`) that defines build artifact definitions for the
Vorpal build system. It uses the `vorpal-sdk` to declare how software packages are fetched,
compiled, and installed across four target systems: `Aarch64Darwin`, `Aarch64Linux`,
`X8664Darwin`, and `X8664Linux`.

---

## Language and Edition

- **Language**: Rust (edition 2021)
- **Binary target**: `vorpal` (entry point: `src/vorpal.rs`)
- **Library crate**: `vorpal_artifacts` (entry point: `src/lib.rs`)

---

## Project Structure and Module Organization

```
src/
  vorpal.rs          # Binary entry point - instantiates and builds all artifacts
  lib.rs             # Library root - exports artifact module, DEFAULT_SYSTEMS, ProjectEnvironment
  artifact.rs        # Module declaration file - pub mod for every artifact
  artifact/
    <name>.rs        # One file per artifact (56 files total)
    file.rs          # Utility artifact (excluded from CI auto-discovery)
script/
  detect-changed-artifacts.sh       # CI artifact detection script
  test-detect-changed-artifacts.sh  # Regression tests for detection script
  lima.sh                           # Lima VM provisioning script
  linux-vorpal-slim.sh              # Linux slim build script
```

### Module Organization Conventions

- Each artifact gets its own file under `src/artifact/`.
- The `src/artifact.rs` file is a flat list of `pub mod` declarations, alphabetically ordered.
- No sub-directories within `artifact/` -- all artifacts are peers at the same level.
- The `file.rs` module is a utility artifact (generic file creation) and is excluded from CI
  artifact discovery via the `EXCLUDED_FILES` array in `detect-changed-artifacts.sh`.

---

## Naming Conventions

### Rust Code

| Element | Convention | Examples |
|---------|-----------|----------|
| **Module names** | `snake_case` | `golangci_lint`, `libgpg_error`, `json_c`, `pkg_config` |
| **Struct names** | `PascalCase` | `GolangciLint`, `LibgpgError`, `JsonC`, `PkgConfig` |
| **Method names** | `snake_case` | `new()`, `build()`, `with_cmake()`, `with_ncurses()` |
| **Constants** | `SCREAMING_SNAKE_CASE` | `DEFAULT_SYSTEMS` |
| **Local variables** | `snake_case` | `source_version`, `source_system`, `step_script` |
| **Lifetime parameters** | Single lowercase letter | `'a` (used consistently) |

### File-to-Struct Name Mapping

Module filenames use `snake_case` and map to `PascalCase` struct names:
- `golangci_lint.rs` -> `GolangciLint`
- `libgpg_error.rs` -> `LibgpgError`
- `openapi_generator_cli.rs` -> `OpenapiGeneratorCli`
- `json_c.rs` -> `JsonC`
- `pkg_config.rs` -> `PkgConfig`

### Artifact Names (String Identifiers)

Artifact names used in build definitions are `kebab-case` strings:
- `"openapi-generator-cli"`, `"golangci-lint"`, `"pkg-config"`, `"json-c"`

The CI script `detect-changed-artifacts.sh` converts between these by replacing underscores with
hyphens (`filename_to_artifact` function).

### Shell Scripts

- Filenames: `kebab-case` (e.g., `detect-changed-artifacts.sh`)
- Variables: `SCREAMING_SNAKE_CASE` for constants/globals, `snake_case` for locals
- Functions: `snake_case` (e.g., `filename_to_artifact`, `discover_artifacts`)

### Git Commit Messages

The project follows conventional commit format:

```
<type>(<scope>): <description>
```

Types observed: `feat`, `fix`, `chore`, `refactor`, `docs`
Scopes observed: `artifact`, `vorpal`, `ffmpeg`, `lock`, `spec`

---

## Design Patterns

### Builder Pattern (Primary Pattern)

Every artifact follows the same builder-style pattern:

1. A struct is defined with `#[derive(Default)]`.
2. A `new()` constructor returns a default instance.
3. Optional dependency injection via `with_<dep>()` methods (fluent/chaining style).
4. A `build()` method that takes `&mut ConfigContext` and returns `Result<String>`.

**Two structural variants exist:**

#### Simple Artifacts (No Dependencies)

Artifacts with no build-time dependencies on other artifacts use a unit struct:

```rust
#[derive(Default)]
pub struct Zlib;

impl Zlib {
    pub fn new() -> Self { Self }
    pub async fn build(self, context: &mut ConfigContext) -> Result<String> { ... }
}
```

Examples: `Argocd`, `Bat`, `Cmake`, `Ffmpeg`, `Zlib`, `Sqlite3`, and most download-only artifacts.

#### Complex Artifacts (With Dependencies)

Artifacts that depend on other artifacts use a struct with `Option<&'a str>` fields and
`with_*()` methods:

```rust
#[derive(Default)]
pub struct Tmux<'a> {
    libevent: Option<&'a str>,
    ncurses: Option<&'a str>,
}

impl<'a> Tmux<'a> {
    pub fn new() -> Self { Self { libevent: None, ncurses: None } }
    pub fn with_libevent(mut self, libevent: &'a str) -> Self { ... }
    pub fn with_ncurses(mut self, ncurses: &'a str) -> Self { ... }
    pub async fn build(self, context: &mut ConfigContext) -> Result<String> { ... }
}
```

Examples: `Gpg`, `Ttyd`, `Nnn`, `Tmux`, `Libwebsockets`, `Zsh`.

### Lazy Dependency Resolution

When a dependency is not explicitly provided via `with_*()`, the `build()` method constructs and
builds the dependency on the fly:

```rust
let ncurses = match self.ncurses {
    Some(val) => val,
    None => &Ncurses::new().build(context).await?,
};
```

This pattern is used consistently across all complex artifacts. It enables both explicit
dependency injection (when sharing a dependency across multiple artifacts) and automatic
resolution (when used standalone).

### System-Specific Dispatching

Artifacts use `match context.get_system()` to handle platform-specific behavior:

- **URL selection**: Different download URLs per platform.
- **Build script variation**: Different build commands for Darwin vs Linux.
- **Unsupported system handling**: Returns `Err(anyhow::anyhow!("Unsupported system for ... artifact"))`.

Some artifacts combine Darwin variants (`Aarch64Darwin | X8664Darwin`) when behavior is identical
across architectures.

---

## Error Handling

### Approach

- **`anyhow::Result`** is the universal error type. No custom error types exist.
- **`?` operator** is used pervasively for error propagation.
- **`anyhow::anyhow!()` macro** creates ad-hoc errors for unsupported system variants.
- No `unwrap()` or `expect()` calls in the codebase.
- No `panic!()` calls.
- No custom `From` or `Error` implementations.

### Error Messages

Error messages follow a descriptive pattern:
```rust
return Err(anyhow::anyhow!("Unsupported system for <name> artifact"));
```

This is the only error created directly in the codebase; all other errors propagate from the SDK
or standard library via `?`.

### Gaps

- Error messages do not include the actual unsupported system value in most cases, making
  debugging harder when a new system variant is added.
- No structured logging or tracing instrumentation exists in this codebase (logging may exist in
  the SDK).

---

## Code Style

### Formatting

- No `rustfmt.toml` or `.rustfmt.toml` configuration file exists. The project relies on
  `rustfmt` defaults.
- Code is consistently formatted (likely via `cargo fmt`), though no CI step enforces it.
- 4-space indentation (Rust default).

### Linting

- No `clippy.toml` configuration file exists.
- No `#[allow(...)]`, `#[warn(...)]`, or `#[deny(...)]` attributes are used anywhere.
- No evidence of Clippy being run in CI.

### Comments

- **No doc comments** (`///` or `//!`) exist anywhere in the Rust source code.
- **No inline comments** (`//`) exist in the Rust source code.
- The only comments appear in `src/vorpal.rs` as section markers: `// Artifacts` and
  `// Development Environment`.
- Comments in `src/lib.rs` serve as section markers: `// Dependencies`, `// Environment variables`,
  `// Artifact`.
- Shell scripts include header comments explaining purpose and usage, following this format:
  ```bash
  #!/usr/bin/env bash
  #
  # script-name.sh
  #
  # Description of what the script does.
  #
  # Usage:
  #   ./script-name.sh <args>
  ```

### Import Organization

Imports follow a consistent ordering (likely `rustfmt` defaults):

1. `crate::` imports (internal dependencies) first.
2. `anyhow::Result`.
3. `indoc::formatdoc`.
4. `vorpal_sdk::` imports grouped in a single `use` block with nested paths.
5. No blank lines between import groups.

### Shell Scripting Style

- All shell scripts use `#!/usr/bin/env bash`.
- All scripts set `set -euo pipefail` at the top.
- Functions are declared with `function name {` (lima.sh) or `name() {` (detect script) syntax --
  not fully consistent.
- Local variables are declared with `local`.

---

## Dependencies

### Direct Dependencies (Cargo.toml)

| Dependency | Version | Purpose |
|-----------|---------|---------|
| `anyhow` | `1` | Error handling |
| `indoc` | `2` | Multi-line string formatting (`formatdoc!`) |
| `tokio` | `1` (with `rt-multi-thread`) | Async runtime |
| `vorpal-sdk` | Git (main branch) | Core Vorpal build system SDK |

### Dependency Management

- **Renovate** is configured (`.github/renovate.json`) with `config:recommended` for automated
  dependency updates.
- `vorpal-sdk` is pinned to the `main` branch of the upstream repository via Git dependency, not
  a versioned crate.
- `Cargo.lock` is committed to the repository (appropriate for a binary crate).

---

## Build Script Patterns (Embedded Shell)

Artifacts contain embedded shell scripts via the `formatdoc!` macro from `indoc`. These scripts
follow common patterns:

### Standard Preamble

```bash
mkdir -pv "$VORPAL_OUTPUT"
pushd ./source/<name>/<name>-<version>
```

### Autotools Pattern (configure/make)

```bash
./configure --prefix="$VORPAL_OUTPUT" [flags]
make
make install
```

### CMake Pattern

```bash
<cmake>/bin/cmake \
    -DCMAKE_INSTALL_PREFIX="$VORPAL_OUTPUT" \
    -DCMAKE_PREFIX_PATH="<deps>" \
    [options] \
    <source>
make install
```

### Binary Download Pattern

```bash
mkdir -pv "$VORPAL_OUTPUT/bin"
cp <source-binary> "$VORPAL_OUTPUT/bin/<name>"
chmod +x "$VORPAL_OUTPUT/bin/<name>"
```

### Environment Setup Pattern (for source builds)

```bash
export PATH="<dep>/bin:$PATH"
export PKG_CONFIG_PATH="<dep>/lib/pkgconfig"
export CPPFLAGS="-I<dep>/include"
export LDFLAGS="-L<dep>/lib -Wl,-rpath,<dep>/lib"
```

---

## Consistency and Uniformity

### Highly Consistent Patterns

- Every artifact follows the same `new()` / `build()` pattern.
- Every artifact returns `Result<String>`.
- Every artifact defines `systems` as `vec![Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux]`.
- Every artifact uses `.with_aliases(vec![format!("{name}:{version}")])`.
- Every artifact uses `step::shell()` for build steps.
- The module declaration file (`artifact.rs`) is a simple flat list with no logic.

### Minor Inconsistencies

- Some artifacts use `source_version` while others use `version` for the version string variable.
- Some artifacts include `#!/bin/bash` and `set -euo pipefail` in their shell scripts
  (`file.rs`), while most do not (likely handled by the SDK).
- The `file.rs` artifact does not use the `#[derive(Default)]` pattern since it requires
  constructor arguments.
- Shell function declaration syntax varies between scripts (`function name {` vs `name() {`).

---

## Tooling and Editor Configuration

### Present

- `.envrc` -- direnv environment file (contents not accessible, but direnv is used for
  environment setup).
- `.gitignore` -- excludes `/.docket` and `/target`.
- `Vorpal.toml` -- Vorpal project configuration declaring Rust language and source includes.
- `lima.yaml` -- Lima VM configuration for cross-platform development/testing.
- `makefile` -- Lima VM management targets (`lima`, `lima-clean`, `lima-sync`, `lima-vorpal`).
- `.docket/` -- Docket issue tracking database.

### Absent

- No `.editorconfig` file.
- No `rustfmt.toml` or `.rustfmt.toml`.
- No `clippy.toml`.
- No `deny.toml` (cargo-deny).
- No pre-commit hooks configuration.
- No IDE-specific configuration files (`.vscode/`, `.idea/`).

---

## Gaps and Observations

1. **No linting or formatting enforcement in CI.** The GitHub Actions workflow builds artifacts
   but does not run `cargo fmt --check`, `cargo clippy`, or any other quality gate.

2. **No documentation.** Zero doc comments across all Rust source files. No module-level
   documentation. The README is minimal (2 lines).

3. **No tests in Rust code.** The Rust codebase has no unit tests or integration tests. Testing
   exists only as shell script regression tests for the CI detection script.

4. **No structured error context.** Error messages for unsupported systems do not include the
   actual system value, making debugging harder.

5. **No logging or tracing.** The codebase does not instrument any operations with logging.
   Observability depends entirely on the underlying SDK.

6. **SDK dependency on Git branch.** The `vorpal-sdk` dependency points to a Git branch (`main`)
   rather than a versioned release, which means builds are not reproducible across time.

7. **Boilerplate-heavy code.** The artifact definition pattern involves significant repetition
   across 56 files. The `with_*()` / `match self.field` pattern for optional dependencies is
   repeated verbatim in every complex artifact. This is a deliberate trade-off: each artifact is
   self-contained and independently readable, at the cost of DRY.
