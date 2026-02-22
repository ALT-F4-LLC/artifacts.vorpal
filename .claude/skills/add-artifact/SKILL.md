---
name: add-artifact
description: Add one or more new artifacts (tools/libraries) to the Vorpal artifacts repository. Use when asked to add, create, or implement a new artifact.
argument-hint: <artifact-name> [artifact-name...]
---

# Add Artifact Skill

You are adding one or more new artifacts to the Vorpal artifacts repository. This repository builds cross-platform tools and libraries for 4 target systems: `Aarch64Darwin`, `Aarch64Linux`, `X8664Darwin`, `X8664Linux`.

## Step 1: Parse & Pre-check

Parse `$ARGUMENTS` into a list of artifact names (space-separated).

**Naming conventions** for each artifact name:
- `snake_name`: Replace hyphens with underscores (e.g., `golangci-lint` → `golangci_lint`)
- `PascalName`: Capitalize each segment after splitting on hyphens/underscores (e.g., `golangci-lint` → `GolangciLint`, `awscli2` → `Awscli2`)
- `artifact_name`: Keep original hyphens for the `name` variable in Rust code (e.g., `golangci-lint` stays `"golangci-lint"`)

**Pre-check**: For each artifact, check if `src/artifact/{snake_name}.rs` already exists. If it does, skip it and inform the user.

## Step 2: Research Official Source

For each artifact, research the official project:

1. Search for the official project website or GitHub repository
2. Find the **latest stable version** (not pre-release, not nightly)
3. Find download URLs for all 4 target platforms:
   - `aarch64-apple-darwin` (macOS ARM)
   - `aarch64-unknown-linux-gnu` or `aarch64-linux` (Linux ARM)
   - `x86_64-apple-darwin` (macOS x86)
   - `x86_64-unknown-linux-gnu` or `x86_64-linux` (Linux x86)

**STRICT RULE**: ONLY use official sources — GitHub releases from the project's own repository, or official project websites (e.g., `ffmpeg.org`, `sqlite.org`, `gnupg.org`). NEVER use third-party mirrors, package managers, or unofficial builds.

## Step 3: Build Strategy Decision

Choose a build pattern based on what's available:

### Pattern A — Pre-built binary (preferred)
Use when official pre-built binaries exist for **all 4 platforms**. This is the simplest and most reliable approach.

**Reference files**: `src/artifact/bat.rs`, `src/artifact/kubectl.rs`

### Pattern B — Source compilation
Use when **no pre-built binaries** are available and the project must be compiled from source.

**Reference file**: `src/artifact/ffmpeg.rs`

### Pattern C — Source with dependencies
Use when building from source **requires other libraries** that must be built first. Uses the builder pattern with `Option<&'a str>` fields for each dependency.

**Reference files**: `src/artifact/tmux.rs`, `src/artifact/gpg.rs`

### Pattern D — Mixed (pre-built + source)
Use when pre-built binaries exist for **some** platforms but not others. Uses per-system match arms with different strategies.

**Reference file**: `src/artifact/ttyd.rs`

## Step 4: Write the Artifact File

Create `src/artifact/{snake_name}.rs` using the appropriate pattern below. **Read the reference files listed in Step 3 to understand the exact code structure before writing.**

### Template A: Pre-built Binary

```rust
use anyhow::Result;
use indoc::formatdoc;
use vorpal_sdk::{
    api::artifact::ArtifactSystem::{Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux},
    artifact::{step, Artifact, ArtifactSource},
    context::ConfigContext,
};

#[derive(Default)]
pub struct PascalName;

impl PascalName {
    pub fn new() -> Self {
        Self
    }

    pub async fn build(self, context: &mut ConfigContext) -> Result<String> {
        let name = "artifact-name";
        let version = "X.Y.Z";

        let source_system = match context.get_system() {
            Aarch64Darwin => "aarch64-apple-darwin",
            Aarch64Linux => "aarch64-unknown-linux-gnu",
            X8664Darwin => "x86_64-apple-darwin",
            X8664Linux => "x86_64-unknown-linux-musl",
            _ => return Err(anyhow::anyhow!("Unsupported system for {name} artifact")),
        };

        let source_path = format!(
            "https://github.com/OWNER/REPO/releases/download/vVERSION/{name}-v{version}-{source_system}.tar.gz"
        );

        let source = ArtifactSource::new(name, &source_path).build();

        let step_script = formatdoc! {"
            mkdir -pv \"$VORPAL_OUTPUT/bin\"
            pushd ./source/{name}/EXTRACTED_DIR_NAME
            cp {name} \"$VORPAL_OUTPUT/bin/{name}\"
            chmod +x \"$VORPAL_OUTPUT/bin/{name}\"",
        };

        let steps = vec![step::shell(context, vec![], vec![], step_script, vec![]).await?];

        let systems = vec![Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux];

        Artifact::new(name, steps, systems)
            .with_aliases(vec![format!("{name}:{version}")])
            .with_sources(vec![source])
            .build(context)
            .await
    }
}
```

**Notes for Pattern A:**
- Adjust `source_system` match arms to match the project's naming convention for platforms
- Some projects use `(os, arch)` tuples instead of combined strings — see `src/artifact/kubectl.rs` for this variant
- Adjust the `source_path` URL format to match the project's release URL pattern
- Adjust the `step_script` to handle the archive structure (tar.gz with subdirectory, flat archive, raw binary, zip, etc.)
- For raw binaries (no archive), use `cp ./source/{name}/FILENAME` without `pushd`

### Template B: Source Compilation

```rust
use anyhow::Result;
use indoc::formatdoc;
use vorpal_sdk::{
    api::artifact::ArtifactSystem::{Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux},
    artifact::{step, Artifact, ArtifactSource},
    context::ConfigContext,
};

#[derive(Default)]
pub struct PascalName;

impl PascalName {
    pub fn new() -> Self {
        Self
    }

    pub async fn build(self, context: &mut ConfigContext) -> Result<String> {
        let name = "artifact-name";
        let version = "X.Y.Z";

        let source_path = format!("https://example.org/releases/{name}-{version}.tar.gz");
        let source = ArtifactSource::new(name, &source_path).build();

        let step_script = formatdoc! {"
            mkdir -pv \"$VORPAL_OUTPUT\"

            pushd ./source/{name}/{name}-{version}

            ./configure --prefix=\"$VORPAL_OUTPUT\"

            make -j$(nproc 2>/dev/null || sysctl -n hw.ncpu)
            make install",
        };

        let steps = vec![step::shell(context, vec![], vec![], step_script, vec![]).await?];

        let systems = vec![Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux];

        Artifact::new(name, steps, systems)
            .with_aliases(vec![format!("{name}:{version}")])
            .with_sources(vec![source])
            .build(context)
            .await
    }
}
```

### Template C: Source with Dependencies

```rust
use crate::artifact::{dep_a, dep_b};
use anyhow::Result;
use indoc::formatdoc;
use vorpal_sdk::{
    api::artifact::ArtifactSystem::{Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux},
    artifact::{get_env_key, step, Artifact, ArtifactSource},
    context::ConfigContext,
};

#[derive(Default)]
pub struct PascalName<'a> {
    dep_a: Option<&'a str>,
    dep_b: Option<&'a str>,
}

impl<'a> PascalName<'a> {
    pub fn new() -> Self {
        Self {
            dep_a: None,
            dep_b: None,
        }
    }

    pub fn with_dep_a(mut self, dep_a: &'a str) -> Self {
        self.dep_a = Some(dep_a);
        self
    }

    pub fn with_dep_b(mut self, dep_b: &'a str) -> Self {
        self.dep_b = Some(dep_b);
        self
    }

    pub async fn build(self, context: &mut ConfigContext) -> Result<String> {
        let dep_a = match self.dep_a {
            Some(val) => val,
            None => &dep_a::DepA::new().build(context).await?,
        };

        let dep_b = match self.dep_b {
            Some(val) => val,
            None => &dep_b::DepB::new().build(context).await?,
        };

        let name = "artifact-name";
        let version = "X.Y.Z";

        let path = format!("https://example.org/releases/{name}-{version}.tar.gz");
        let source = ArtifactSource::new(name, &path).build();

        let script = formatdoc! {"
            mkdir -pv \"$VORPAL_OUTPUT\"

            pushd ./source/{name}/{name}-{version}

            export CPPFLAGS=\"-I{dep_a}/include -I{dep_b}/include\"
            export LDFLAGS=\"-L{dep_a}/lib -L{dep_b}/lib -Wl,-rpath,{dep_a}/lib -Wl,-rpath,{dep_b}/lib\"

            ./configure --prefix=\"$VORPAL_OUTPUT\"

            make
            make install",
            dep_a = get_env_key(&dep_a.to_string()),
            dep_b = get_env_key(&dep_b.to_string()),
        };

        let steps = vec![
            step::shell(
                context,
                vec![dep_a.to_string(), dep_b.to_string()],
                vec![],
                script,
                vec![],
            )
            .await?,
        ];

        let systems = vec![Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux];

        Artifact::new(name, steps, systems)
            .with_aliases(vec![format!("{name}:{version}")])
            .with_sources(vec![source])
            .build(context)
            .await
    }
}
```

**Notes for Pattern C:**
- Import dependencies via `use crate::artifact::{dep_module};`
- Each dependency gets an `Option<&'a str>` field, a `with_dep()` builder method, and a `match` block in `build()` that auto-builds if `None`
- Dependencies that themselves have dependencies should be wired: e.g., `dep_a::DepA::new().with_shared_dep(shared_dep).build(context).await?`
- Pass all dependency artifact strings in the `step::shell` artifacts vec
- Use `get_env_key(&dep.to_string())` in `formatdoc!` to reference dependency paths in shell scripts

### Template D: Mixed (pre-built + source)

See `src/artifact/ttyd.rs` for the full reference. The key pattern is:

```rust
let (sources, step_script, step_artifacts) = match context.get_system() {
    Aarch64Linux => {
        // Pre-built binary path
        let path = format!("https://...");
        let script = formatdoc! {"
            mkdir -pv \"$VORPAL_OUTPUT/bin\"
            cp ./source/{name}/BINARY \"$VORPAL_OUTPUT/bin/{name}\"
            chmod +x \"$VORPAL_OUTPUT/bin/{name}\""
        };
        (vec![ArtifactSource::new(name, &path).build()], script, vec![])
    }
    Aarch64Darwin | X8664Darwin => {
        // Source compilation with dependencies
        let path = format!("https://...");
        let script = formatdoc! {"..."};
        let artifacts = vec![dep.to_string()];
        (vec![ArtifactSource::new(name, &path).build()], script, artifacts)
    }
    _ => return Err(anyhow::anyhow!("Unsupported system")),
};
```

## Step 5: Handle Dependencies

If the artifact requires dependencies (Pattern C or D):

1. Check if each dependency already exists as `src/artifact/{dep_snake_name}.rs`
2. If a dependency artifact **does not exist**, you must **add it first** before adding the main artifact — follow this entire skill workflow recursively for the missing dependency
3. Ensure dependency artifacts are registered in `src/artifact.rs` and `src/vorpal.rs` before the dependent artifact

## Step 6: Registry Wiring

For **each** new artifact, make exactly 3 changes:

### 6a. Create the artifact file
Already done in Step 4: `src/artifact/{snake_name}.rs`

### 6b. Register the module in `src/artifact.rs`
Insert `pub mod {snake_name};` in **alphabetical order** among the existing `pub mod` lines.

### 6c. Wire into `src/vorpal.rs`
Two insertions, both in **alphabetical order**:

1. **Import**: Add `{snake_name}::{PascalName}` into the `use vorpal_artifacts::artifact::{...}` block, maintaining alphabetical order
2. **Build call**: Add `{PascalName}::new().build(context).await?;` in the `// Artifacts` section, maintaining alphabetical order among existing build calls

## Step 7: Verification (Mandatory Gate)

**IMPORTANT: The artifact is NOT considered done until `vorpal build <artifact-name>` succeeds. This step is a mandatory validation gate — do NOT mark the task as complete, close the issue, or report success unless the build passes.**

Run these commands in order:

```bash
cargo check
```

If `cargo check` fails, fix the errors and re-run until it passes.

```bash
cargo fmt
```

```bash
cargo check
```

If `cargo check` passes after formatting, run the build for each new artifact:

```bash
vorpal build <artifact-name>
```

If the build fails, debug and fix the artifact implementation. Common issues:
- Wrong download URL or URL pattern
- Wrong archive extraction path (check what directory the archive creates)
- Missing build dependencies
- Platform-specific build flags needed

Re-run `vorpal build <artifact-name>` until it succeeds with no errors.

**A successful `vorpal build <artifact-name>` is the ONLY criteria that validates the artifact works. The task MUST NOT be reported as complete or successful if this command has not run and passed.**

## Step 8: Edge Cases

- **Artifact already exists**: Skip with a message like "Artifact `{name}` already exists at `src/artifact/{snake_name}.rs`, skipping."
- **Hyphenated names**: `golangci-lint` → file `golangci_lint.rs`, struct `GolangciLint`, name variable `"golangci-lint"`
- **Names with numbers**: `awscli2` → struct `Awscli2` (number stays attached to the word segment)
- **Archive variations**:
  - `.tar.gz` with subdirectory: `pushd ./source/{name}/{extracted-dir}` then copy
  - Flat `.tar.gz`: extract directly, `cp ./source/{name}/binary`
  - Raw binary (no archive): `cp ./source/{name}/binary`
  - `.zip` archives: use `unzip` in the script
  - `.tar.xz` / `.tar.bz2`: handled automatically by the source fetcher
- **Multiple artifacts in one invocation**: Process each independently, running through all steps for each
- **Non-GitHub sources**: Acceptable if they are the official source (e.g., `ffmpeg.org`, `sqlite.org`, `gnupg.org`, `dl.k8s.io`)
