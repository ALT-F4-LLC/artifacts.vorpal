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

            # Build-step smoke test: prove pi launches from the built wrapper and resolves its
            # bundled theme assets in the real output layout. `pi --version` short-circuits before
            # initTheme, so drive the main startup path with stdin closed (non-tty -> immediate EOF
            # -> clean exit) and assert it exits 0 and never hits the theme ENOENT.
            echo 'Running pi theme-resolution smoke test against built wrapper...'
            smoke_home=\"$VORPAL_WORKSPACE/{name}-smoke-home\"
            mkdir -pv \"$smoke_home\"
            set +e
            smoke_out=\"$(HOME=\"$smoke_home\" \"$VORPAL_OUTPUT/bin/{name}\" </dev/null 2>&1)\"
            smoke_rc=$?
            set -e
            printf '%s\\n' \"$smoke_out\" | head -20
            if [ \"$smoke_rc\" -ne 0 ] || printf '%s' \"$smoke_out\" | grep -qi 'dark.json'; then
                echo \"ERROR: pi smoke test failed (rc=$smoke_rc) - theme assets not resolved\"
                exit 1
            fi
            echo 'pi smoke test OK: launched past initTheme; theme assets resolved.'",
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
