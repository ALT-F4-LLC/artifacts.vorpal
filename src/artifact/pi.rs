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
        let version = "0.84.3";

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

            cp -R ./source/{name}/{name} \"$VORPAL_OUTPUT/lib/{name}\"
            chmod +x \"$VORPAL_OUTPUT/lib/{name}/{name}\"

            cat << EOF > \"$VORPAL_OUTPUT/bin/{name}\"
            #!/bin/sh
            exec \"$VORPAL_OUTPUT/lib/{name}/{name}\" \"\\$@\"
            EOF

            chmod +x \"$VORPAL_OUTPUT/bin/{name}\"

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
