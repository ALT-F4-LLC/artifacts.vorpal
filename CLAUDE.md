# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Rust project that defines and builds software artifacts (packages/tools) for the **Vorpal** build system. Each artifact is a self-contained definition that downloads, builds, and installs a specific tool or library across four target platforms: `Aarch64Darwin`, `Aarch64Linux`, `X8664Darwin`, `X8664Linux`.

## Build and Test Commands

- **Build**: `cargo build` (or use Vorpal: `vorpal build <artifact-name>`)
- **Check**: `cargo check`
- **Run tests**: `./script/test-detect-changed-artifacts.sh`
- **Build in Lima VM** (Linux on macOS): `make lima` then `make lima-vorpal VORPAL_ARTIFACT=<name>`

There are no Rust unit tests; testing covers the CI artifact detection scripts.

## Architecture

### Artifact Builder Pattern

Every artifact in `src/artifact/` follows the same structure:

1. A struct (optionally holding dependency references) with `new()` constructor
2. Optional `with_*()` builder methods for injecting dependency artifact keys
3. An `async fn build(self, context: &mut ConfigContext) -> Result<String>` that:
   - Resolves platform-specific source URLs via `context.get_system()` match
   - Creates an `ArtifactSource` for download
   - Writes a shell build script (typically extract + copy to `$VORPAL_OUTPUT/bin`)
   - Returns the artifact key string

**Simple artifact** (pre-built binary download): `src/artifact/jj.rs`
**Complex artifact** (source build with dependencies): `src/artifact/gpg.rs`

### Dependency Graph

Artifacts can depend on other artifacts. Dependencies are built first in `src/vorpal.rs` and injected via builder methods. Key dependency chains:

- `gpg` depends on `libassuan`, `libgcrypt`, `libgpg_error`, `libksba`, `npth`
- `tmux` depends on `libevent`, `ncurses`
- `readline` depends on `ncurses`
- `nnn` depends on `ncurses`, `pkg_config`, `readline`

### Entry Points

- `src/vorpal.rs` — Binary entry point. Builds all artifacts in dependency order, then creates the `dev` ProjectEnvironment.
- `src/lib.rs` — Exports `ProjectEnvironment` (bundles Lima, Protoc, Rust toolchain into a dev environment) and `DEFAULT_SYSTEMS`.
- `src/artifact.rs` — Module declarations for all artifact modules.

### Adding a New Artifact

1. Create `src/artifact/<name>.rs` implementing the builder pattern
2. Add `pub mod <name>;` to `src/artifact.rs`
3. Import and call `.build(context)` in `src/vorpal.rs` (respecting dependency order)
4. CI automatically detects the new file via `script/detect-changed-artifacts.sh`

### CI Artifact Detection

`script/detect-changed-artifacts.sh` auto-discovers artifacts from `src/artifact/*.rs` filenames, converts `snake_case.rs` to `kebab-case` names, and uses `git diff --diff-filter=d` to determine which artifacts changed between commits. Only changed artifacts are built in CI.

## Issue Tracking

See `AGENTS.md` for full instructions on using **Linear** for issue tracking via MCP tools. Key points:

- All issue management uses Linear MCP tools (`list_issues`, `create_issue`, `update_issue`, etc.)
- Issues are scoped to a project matching the repository name, under the "Agents" team
- Issue titles must follow the format: `[<branch>] <description>` (e.g., `[main] Feature: add new artifact`)
- Every issue must have exactly one label: **Bug**, **Feature**, or **Improvement**
- Session completion requires closing all finished issues in Linear (`state="Done"`) with completion summary comments
