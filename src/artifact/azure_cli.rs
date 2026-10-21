use anyhow::Result;
use indoc::formatdoc;
use vorpal_sdk::{
    api::artifact::ArtifactSystem::{Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux},
    artifact::{cpython::Cpython, get_env_key, step, Artifact, ArtifactSource},
    context::ConfigContext,
};

#[derive(Default)]
pub struct AzureCli;

impl AzureCli {
    pub fn new() -> Self {
        Self
    }

    pub async fn build(self, context: &mut ConfigContext) -> Result<String> {
        let cpython = Cpython::new().build(context).await?;

        let name = "azure-cli";
        let version = "2.91.0";

        let source_system = match context.get_system() {
            Aarch64Darwin => "macos-arm64",
            Aarch64Linux => "linux-arm64",
            X8664Darwin => "macos-x86_64",
            X8664Linux => "linux-x86_64",
            _ => return Err(anyhow::anyhow!("Unsupported system for {name} artifact")),
        };

        let source_path = format!(
            "https://github.com/Azure/azure-cli/releases/download/{name}-{version}/{name}-{version}-{source_system}.tar.gz"
        );

        let source = ArtifactSource::new(name, &source_path).build();

        let step_script = formatdoc! {"
            mkdir -pv \"$VORPAL_OUTPUT\"
            pushd ./source/{name}
            cp -R ./. \"$VORPAL_OUTPUT/\"
            chmod +x \"$VORPAL_OUTPUT/libexec/bin/az\"
            test -e \"$VORPAL_OUTPUT/libexec/bin/az\" || (echo 'ERROR: az launcher not found after copy' && exit 1)

            rm -f \"$VORPAL_OUTPUT/bin/az\"

            cat << EOF > \"$VORPAL_OUTPUT/bin/az\"
            #!/bin/sh
            if [ -z \"\\$AZ_PYTHON\" ]; then
                AZ_PYTHON={cpython}/bin/python3
            fi
            export AZ_PYTHON
            exec \"$VORPAL_OUTPUT/libexec/bin/az\" \"\\$@\"
            EOF

            chmod +x \"$VORPAL_OUTPUT/bin/az\"",
            cpython = get_env_key(&cpython),
        };

        let steps =
            vec![step::shell(context, &[cpython.to_string()], &[], step_script, &[]).await?];

        let systems = vec![Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux];

        Artifact::new(name, steps, systems)
            .with_aliases(vec![format!("{name}:{version}")])
            .with_sources(vec![source])
            .build(context)
            .await
    }
}
