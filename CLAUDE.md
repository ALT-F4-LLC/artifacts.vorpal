# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Vorpal Artifacts repository that defines pre-compiled binaries and development tools using the Vorpal build system. Each artifact is a Rust struct implementing a builder pattern to download, compile, and package software for cross-platform distribution.

## Build Commands

```bash
# Build the development environment
vorpal build 'dev'

# Build a specific artifact
vorpal build 'bat'
vorpal build 'gpg'
```

The project requires the Vorpal CLI installed via `setup-vorpal-action` or directly from the ALT-F4-LLC/vorpal repository.

## Architecture

### Artifact Builder Pattern

Each artifact in `src/artifact/` follows this structure:

1. **Simple artifacts** (no dependencies): Implement `new()` and `build()` methods
   - Example: `Bat`, `Jq`, `Kubectl` - download pre-built binaries

2. **Complex artifacts** (with dependencies): Add `with_*()` builder methods for each dependency
   - Example: `Gpg` requires `libassuan`, `libgcrypt`, `libgpg_error`, `libksba`, `npth`
   - Dependencies are injected via builder pattern and their paths accessed via `get_env_key()`

### Dependency Graph

Dependencies are wired in `src/vorpal.rs`. Artifacts that need dependencies must be built after their dependencies, and the dependency keys are passed via `with_*()` methods:

```rust
let ncurses = Ncurses::new().build(context).await?;
let readline = Readline::new()
    .with_ncurses(&ncurses)
    .build(context)
    .await?;
```

### Target Systems

All artifacts support four platforms defined in `DEFAULT_SYSTEMS`:
- `Aarch64Darwin` (Apple Silicon macOS)
- `Aarch64Linux` (ARM Linux)
- `X8664Darwin` (Intel macOS)
- `X8664Linux` (x86_64 Linux)

### Build Steps

Artifacts use `step::shell()` for build scripts. The `$VORPAL_OUTPUT` environment variable is the staging directory for the final artifact. Dependencies are accessed via `get_env_key(&artifact_key)` which resolves to the artifact's installation path.

## Adding New Artifacts

1. Create `src/artifact/<name>.rs` following the pattern in existing artifacts
2. Export the module in `src/artifact.rs`
3. Add the artifact build call in `src/vorpal.rs` (respecting dependency order)
4. CI automatically detects changed artifacts via `script/detect-changed-artifacts.sh`

## Issue Tracking

This project uses `bd` (beads) for issue tracking. See AGENTS.md for workflow details.
