# Code Quality Specification

This document describes the coding standards, naming conventions, error handling patterns, design
patterns, and project-specific style decisions as they actually exist in the codebase.

## Language and Edition

- **Language**: Rust (edition 2021)
- **Shell scripts**: Bash (`#!/usr/bin/env bash`)
- **Build system**: Cargo (standard Rust toolchain)
- **CI/CD**: GitHub Actions YAML

## Formatter and Linter Configuration

### What Exists

- **No `rustfmt.toml` or `.rustfmt.toml`**: The project relies on Rust's default `rustfmt`
  settings. All code in the repository appears to follow standard `rustfmt` formatting.
- **No Clippy configuration**: No `.clippy.toml` or `clippy.toml` exists. There is no evidence
  of Clippy being run in CI or locally.
- **No `.editorconfig`**: No editor configuration file exists.
- **No pre-commit hooks**: No `.pre-commit-config.yaml` or git hooks are configured.

### What Does NOT Exist (Gaps)

- No automated formatting enforcement in CI (no `cargo fmt -- --check` step).
- No automated linting in CI (no `cargo clippy` step).
- No `cargo check` step in CI beyond what `vorpal build` implies.
- The CI workflow (`vorpal.yaml`) only runs `vorpal build` commands -- it does not run any
  Rust-specific quality gates.

## Naming Conventions

### Rust Modules and Files

- **Module files**: `snake_case.rs` (e.g., `libgpg_error.rs`, `openapi_generator_cli.rs`,
  `pkg_config.rs`).
- **Module declarations in `src/artifact.rs`**: Flat list of `pub mod <name>;` in alphabetical
  order. No nested modules.
- **Artifact names at runtime**: kebab-case strings (e.g., `"libgpg-error"`, `"openapi-generator-cli"`).
  The CI script `detect-changed-artifacts.sh` converts `snake_case` filenames to `kebab-case`
  artifact names via `tr '_' '-'`.

### Struct Naming

- **Artifact structs**: PascalCase matching the module name (e.g., `LibgpgError` in
  `libgpg_error.rs`, `OpenapiGeneratorCli` in `openapi_generator_cli.rs`, `Jj` in `jj.rs`).
- **Every artifact struct derives `Default`**: `#[derive(Default)]` is applied to all 47 artifact
  structs without exception.

### Variable Naming

- `name`: The artifact's runtime name string (kebab-case, e.g., `"libgpg-error"`).
- `version` or `source_version`: The upstream version string.
- `source_system` or `source_path`: Platform-specific URL components.
- `source`: The `ArtifactSource` object.
- `step_script` or `script`: The shell script string for the build step.
- `steps`: The `Vec` of build steps.
- `systems`: The `Vec<ArtifactSystem>` of target platforms.
- Dependency variables: Named after the dependency artifact (e.g., `ncurses`, `libevent`,
  `libgpg_error`), holding `&str` artifact key references.

### Method Naming

- `new()`: Constructor, always returns `Self`. Takes no arguments for simple artifacts, takes
  parameters for utility structs like `File`.
- `with_<dependency>(mut self, <dependency>: &'a str) -> Self`: Builder method for injecting
  dependency artifact keys.
- `build(self, context: &mut ConfigContext) -> Result<String>`: Async method that performs the
  artifact registration and returns the artifact key.

## Design Patterns

### Builder Pattern (Primary Pattern)

Every artifact follows the same builder pattern. There are three structural tiers:

**Tier 1 -- Simple artifact (no dependencies):**
Struct is a unit struct, `new()` returns `Self`, `build()` does all the work.
Examples: `Bat`, `Jj`, `Lima`, `Ncurses`, `Sqlite3`, `LibgpgError`.

```rust
#[derive(Default)]
pub struct ArtifactName;

impl ArtifactName {
    pub fn new() -> Self { Self }
    pub async fn build(self, context: &mut ConfigContext) -> Result<String> { ... }
}
```

**Tier 2 -- Artifact with dependencies:**
Struct has `Option<&'a str>` fields for each dependency. `with_*()` methods inject them.
`build()` resolves uninjected dependencies by building them inline.
Examples: `Tmux`, `Zsh`, `Readline`, `Nnn`, `Gpg`, `OpenapiGeneratorCli`.

```rust
#[derive(Default)]
pub struct ArtifactName<'a> {
    dependency: Option<&'a str>,
}

impl<'a> ArtifactName<'a> {
    pub fn new() -> Self { Self { dependency: None } }
    pub fn with_dependency(mut self, dep: &'a str) -> Self { self.dependency = Some(dep); self }
    pub async fn build(self, context: &mut ConfigContext) -> Result<String> { ... }
}
```

**Tier 3 -- Language-specific builder (SDK-provided):**
Uses `vorpal_sdk::artifact::language::go::Go` builder instead of manual `step::shell`.
Examples: `Crane`, `Skopeo`, `Umoci`.

```rust
Go::new(name, systems)
    .with_alias(...)
    .with_build_directory(...)
    .with_build_path(...)
    .with_source(source)
    .build(context)
    .await
```

**Tier 4 -- Utility struct (non-artifact):**
The `File` struct in `file.rs` is a utility builder for creating file-based artifacts. It takes
`content`, `name`, and `systems` as constructor parameters. It does NOT derive `Default` and
does NOT follow the `new()` + `with_*()` pattern.

### Dependency Resolution Pattern

When an artifact has dependencies, each dependency follows a consistent resolution pattern:

```rust
let dep = match self.dep {
    Some(val) => val,
    None => &DepStruct::new().build(context).await?,
};
```

This allows dependencies to be either:
1. **Injected** from the orchestrator (`src/vorpal.rs`) for shared/deduplication.
2. **Built inline** as a fallback when the artifact is used standalone.

### Orchestration Pattern

`src/vorpal.rs` serves as the dependency-aware orchestrator:
1. Shared dependencies are built first and stored as `let` bindings.
2. Artifacts that depend on shared dependencies use `with_*()` to inject them.
3. Independent artifacts call `.build(context)` directly without injection.
4. The final `ProjectEnvironment` is built after all artifacts.

### Platform Dispatch Pattern

Platform-specific values are resolved via `match context.get_system()`:

```rust
let source_system = match context.get_system() {
    Aarch64Darwin => "...",
    Aarch64Linux => "...",
    X8664Darwin => "...",
    X8664Linux => "...",
    _ => return Err(anyhow::anyhow!("Unsupported system for <name> artifact")),
};
```

All four platforms (`Aarch64Darwin`, `Aarch64Linux`, `X8664Darwin`, `X8664Linux`) are always
listed. The wildcard arm returns an `anyhow` error.

Some artifacts (e.g., `awscli2`) use more complex platform dispatch where the entire
`(source_path, step_script)` tuple is matched per-platform.

## Error Handling

### Patterns in Use

- **`anyhow::Result<T>`**: Used exclusively throughout the project. No custom error types.
- **`?` operator**: The primary error propagation mechanism. Used on all `await?` calls and
  `context.get_system()` results.
- **`anyhow::anyhow!("...")`**: Used for unsupported platform errors in the wildcard match arm.
  Message format: `"Unsupported system for <name> artifact"`.
- **No `unwrap()` or `expect()` calls**: All fallible operations use `?`.
- **No `panic!()` calls**: The codebase does not use panics.

### Error Propagation Chain

Errors propagate upward through the `Result` type:
- Artifact `build()` returns `Result<String>`.
- `main()` in `vorpal.rs` returns `Result<()>`.
- The `#[tokio::main]` attribute handles top-level errors.

### Gaps

- Error messages are minimal. The `anyhow::anyhow!` messages do not include contextual
  information like which system was actually encountered.
- No `.context()` or `.with_context()` usage from `anyhow` to add layers of error context.

## Import Organization

Imports follow a consistent ordering pattern (enforced by `rustfmt` defaults):

1. `crate::` imports (local dependencies)
2. `anyhow::Result`
3. `indoc::formatdoc`
4. `vorpal_sdk::` imports

Within the `vorpal_sdk` import block, items are grouped as:
- `api::artifact::ArtifactSystem::{...}` (always listing all four platforms)
- `artifact::{step, Artifact, ArtifactSource}` (or `get_env_key` when needed)
- `context::ConfigContext`

## Code Comments

### What Exists

- **Minimal inline comments**: Only `src/vorpal.rs` and `src/lib.rs` contain comments, using
  section headers like `// Artifacts`, `// Dependencies`, `// Environment variables`,
  `// Artifact`, `// Development Environment`.
- **No doc comments (`///`)**: Zero doc comments exist anywhere in the source code.
- **Shell script comments**: The `detect-changed-artifacts.sh` and
  `test-detect-changed-artifacts.sh` files have descriptive header comments and inline
  documentation.

### What Does NOT Exist

- No `//!` module-level documentation.
- No `///` function/struct documentation.
- No `#[doc = "..."]` attributes.
- No `README.md` content beyond a title (the file contains only `# vorpal-artifacts`).

## Module Organization

```
src/
  artifact.rs          -- Module declarations (flat pub mod list, alphabetical)
  artifact/
    <name>.rs          -- One file per artifact (47 artifacts + 1 utility)
    file.rs            -- Utility struct (not an artifact itself, excluded from CI detection)
  lib.rs               -- Library root: exports ProjectEnvironment, DEFAULT_SYSTEMS
  vorpal.rs            -- Binary entry point: orchestrates all artifact builds
```

- Each artifact is a single file containing a single public struct and its `impl` block.
- No artifact file exceeds ~160 lines. Most are 40-90 lines.
- No sub-modules within `artifact/`. All artifacts are siblings.
- The `file.rs` module is explicitly excluded from CI artifact detection via the `EXCLUDED_FILES`
  array in `detect-changed-artifacts.sh`.

## Shell Script Conventions

### Embedded Build Scripts (in Rust)

- Built using `indoc::formatdoc!` macro for heredoc-style string interpolation.
- Shell variables use `$VORPAL_OUTPUT` (provided by the Vorpal runtime).
- Template variables use Rust's `{variable}` format syntax within `formatdoc!`.
- Dependency paths are resolved via `get_env_key()` for environment variable interpolation.
- Common patterns: `mkdir -pv "$VORPAL_OUTPUT/bin"`, `pushd ./source/{name}/...`,
  `chmod +x "$VORPAL_OUTPUT/bin/..."`.

### Standalone Scripts (`script/`)

- All start with `#!/usr/bin/env bash` and `set -euo pipefail`.
- Functions use `snake_case` naming.
- Variables use `UPPER_SNAKE_CASE` for constants and `lower_snake_case` for locals.
- `detect-changed-artifacts.sh` has a `main()` entry point pattern.
- Test script uses colored output (ANSI escape codes) and pass/fail counting.

## Dependency Management

### Rust Dependencies (Cargo.toml)

- **`anyhow = "1"`**: Error handling (permissive version).
- **`indoc = { version = "2" }`**: String formatting for shell scripts.
- **`tokio = { features = ["rt-multi-thread"], version = "1" }`**: Async runtime.
- **`vorpal-sdk`**: Git dependency pinned to `main` branch of `ALT-F4-LLC/vorpal.git`.

### Automated Updates

- **Renovate** is configured (`.github/renovate.json`) with the `config:recommended` preset.
  This covers GitHub Actions version bumps. Cargo dependency updates depend on Renovate's
  Rust manager being enabled by default.

## Consistency and Uniformity

The codebase is notable for its high degree of structural consistency:

- All 47 artifacts follow the same builder pattern with minor tier variations.
- All artifacts derive `Default`.
- All artifacts target the same four platforms.
- All artifacts use the same `Artifact::new(name, steps, systems)` terminal call.
- All artifacts use `.with_aliases(vec![format!("{name}:{version}")])` for versioned aliases.
- All artifacts use `.with_sources(vec![source])` for source registration.
- Import blocks are formatted identically across files.
- The only exceptions are the three Go-based artifacts (`crane`, `skopeo`, `umoci`) which use
  the SDK's `Go` builder, and the `file.rs` utility which serves a different purpose.

## Known Gaps and Areas for Improvement

1. **No CI quality gates**: No `cargo fmt`, `cargo clippy`, or `cargo check` steps in the CI
   pipeline.
2. **No documentation**: Zero doc comments in Rust code. README is a stub.
3. **No rustfmt configuration**: Relying on defaults is fine, but there is no enforcement.
4. **No Clippy configuration**: No linting is performed or configured.
5. **Minimal error context**: Errors do not include contextual information beyond the immediate
   failure message.
6. **No tests in Rust code**: Only shell script tests exist (`test-detect-changed-artifacts.sh`).
   No `#[test]` modules in any Rust source file.
7. **Git dependency for vorpal-sdk**: Pinned to `main` branch, which means builds are not
   reproducible across time. A pinned commit hash or published crate version would be more
   deterministic.
