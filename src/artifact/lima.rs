use anyhow::Result;
use indoc::formatdoc;
use vorpal_sdk::{
    api::artifact::ArtifactSystem::{Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux},
    artifact::{step, Artifact, ArtifactSource},
    context::ConfigContext,
};

#[derive(Default)]
pub struct Lima;

impl Lima {
    pub fn new() -> Self {
        Self
    }

    pub async fn build(self, context: &mut ConfigContext) -> Result<String> {
        let name = "lima";
        let source_version = "2.0.3";

        let source_system = match context.get_system() {
            Aarch64Darwin => "Darwin-arm64",
            Aarch64Linux => "Linux-aarch64",
            X8664Darwin => "Darwin-x86_64",
            X8664Linux => "Linux-x86_64",
            _ => return Err(anyhow::anyhow!("Unsupported system for lima artifact")),
        };

        let source_path = format!(
            "https://github.com/lima-vm/lima/releases/download/v{source_version}/lima-{source_version}-{source_system}.tar.gz"
        );

        let source = ArtifactSource::new(name, &source_path).build();

        let step_script = formatdoc! {"
            mkdir -pv \"$VORPAL_OUTPUT\"
            pushd ./source/{name}
            cp -r bin \"$VORPAL_OUTPUT/\"
            cp -r libexec \"$VORPAL_OUTPUT/\"
            cp -r share \"$VORPAL_OUTPUT/\"
            chmod +x \"$VORPAL_OUTPUT/bin/\"*",
        };

        let steps = vec![step::shell(context, vec![], vec![], step_script, vec![]).await?];

        let systems = vec![Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux];

        Artifact::new(name, steps, systems)
            .with_aliases(vec![format!("{name}:{source_version}")])
            .with_sources(vec![source])
            .build(context)
            .await
    }
}
