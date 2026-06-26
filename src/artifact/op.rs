use anyhow::Result;
use indoc::formatdoc;
use vorpal_sdk::{
    api::artifact::ArtifactSystem::{Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux},
    artifact::{step, Artifact, ArtifactSource},
    context::ConfigContext,
};

#[derive(Default)]
pub struct Op;

impl Op {
    pub fn new() -> Self {
        Self
    }

    pub async fn build(self, context: &mut ConfigContext) -> Result<String> {
        let name = "op";
        let source_version = "2.34.1";

        let source_system = match context.get_system() {
            Aarch64Darwin => "darwin_arm64",
            Aarch64Linux => "linux_arm64",
            X8664Darwin => "darwin_amd64",
            X8664Linux => "linux_amd64",
            _ => return Err(anyhow::anyhow!("Unsupported system for op artifact")),
        };

        let source_path = format!(
            "https://cache.agilebits.com/dist/1P/op2/pkg/v{source_version}/op_{source_system}_v{source_version}.zip"
        );

        let source = ArtifactSource::new(name, &source_path).build();

        let step_script = formatdoc! {"
            mkdir -pv \"$VORPAL_OUTPUT/bin\"
            pushd ./source/{name}
            cp op \"$VORPAL_OUTPUT/bin/op\"
            chmod +x \"$VORPAL_OUTPUT/bin/op\"",
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
