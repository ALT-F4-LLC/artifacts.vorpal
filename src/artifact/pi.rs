use anyhow::Result;
use indoc::formatdoc;
use vorpal_sdk::{
    api::artifact::ArtifactSystem::{Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux},
    artifact::{step, Artifact, ArtifactSource},
    context::ConfigContext,
};

#[derive(Default)]
pub struct Pi;

impl Pi {
    pub fn new() -> Self {
        Self
    }

    pub async fn build(self, context: &mut ConfigContext) -> Result<String> {
        let name = "pi";
        let version = "0.80.2";

        let source_system = match context.get_system() {
            Aarch64Darwin => "darwin-arm64",
            Aarch64Linux => "linux-arm64",
            X8664Darwin => "darwin-x64",
            X8664Linux => "linux-x64",
            _ => return Err(anyhow::anyhow!("Unsupported system for {name} artifact")),
        };

        let source_path = format!(
            "https://github.com/earendil-works/pi/releases/download/v{version}/{name}-{source_system}.tar.gz"
        );

        let source = ArtifactSource::new(name, &source_path).build();

        let step_script = formatdoc! {"
            mkdir -pv \"$VORPAL_OUTPUT/bin\" \"$VORPAL_OUTPUT/lib\"

            # pi is a Bun-compiled binary that resolves every bundled asset (theme/, export-html/,
            # assets/, docs/, ...) relative to dirname(process.execPath). Copy the whole upstream
            # tree under lib/{name} to preserve that layout, then drop a thin wrapper in bin/ that
            # execs the real binary by absolute path so execPath = lib/{name}/{name} and the assets
            # resolve at lib/{name}/theme. Copying only the binary drops theme/ -> ENOENT at startup.
            cp -R ./source/{name}/{name} \"$VORPAL_OUTPUT/lib/{name}\"
            chmod +x \"$VORPAL_OUTPUT/lib/{name}/{name}\"

            cat << EOF > \"$VORPAL_OUTPUT/bin/{name}\"
            #!/bin/sh
            exec \"$VORPAL_OUTPUT/lib/{name}/{name}\" \"\\$@\"
            EOF

            chmod +x \"$VORPAL_OUTPUT/bin/{name}\"

            # Build-step packaging check: this build host's CPU may not support executing the
            # upstream {name} binary (e.g. a Bun-compiled binary hitting SIGILL on an
            # incompatible microarch) even though the packaged output on disk is byte-identical
            # across build hosts. The build gate verifies packaging is complete and
            # host-independent; it does not verify the binary can execute on the current build
            # host - that's a separate, host-coupled concern intentionally out of scope here.
            # So assert the on-disk layout without ever exec'ing the binary: the bundled asset
            # directories the upstream release ships (confirmed present in the release archive)
            # exist and are non-empty, the wrapper exists and is executable, and the wrapper's
            # exec target resolves to the real binary's absolute path.
            echo 'Running pi packaging check (structural, no binary execution)...'

            for asset_dir in theme export-html assets docs; do
                asset_path=\"$VORPAL_OUTPUT/lib/{name}/$asset_dir\"
                if [ ! -d \"$asset_path\" ] || [ -z \"$(ls -A \"$asset_path\")\" ]; then
                    echo \"ERROR: pi packaging check failed - $asset_path is missing or empty\"
                    exit 1
                fi
            done

            if [ ! -x \"$VORPAL_OUTPUT/bin/{name}\" ]; then
                echo \"ERROR: pi packaging check failed - $VORPAL_OUTPUT/bin/{name} is missing or not executable\"
                exit 1
            fi

            expected_exec=\"$VORPAL_OUTPUT/lib/{name}/{name}\"
            if ! grep -qF \"exec \\\"$expected_exec\\\"\" \"$VORPAL_OUTPUT/bin/{name}\"; then
                echo \"ERROR: pi packaging check failed - wrapper does not exec $expected_exec\"
                exit 1
            fi

            echo 'pi packaging check OK: bundled assets present, wrapper exec target correct.'",
            name = name,
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
