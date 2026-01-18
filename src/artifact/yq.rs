use anyhow::Result;
use indoc::formatdoc;
use vorpal_sdk::{
    api::artifact::ArtifactSystem::{Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux},
    artifact::{step, Artifact, ArtifactSource},
    context::ConfigContext,
};

#[derive(Default)]
pub struct Yq;

impl Yq {
    pub fn new() -> Self {
        Self
    }

    pub async fn build(self, context: &mut ConfigContext) -> Result<String> {
        let name = "yq";
        let source_version = "4.50.1";

        let source_system = match context.get_system() {
            Aarch64Darwin => "darwin_arm64",
            Aarch64Linux => "linux_arm64",
            X8664Darwin => "darwin_amd64",
            X8664Linux => "linux_amd64",
            _ => return Err(anyhow::anyhow!("Unsupported system for yq artifact")),
        };

        let source_path = format!(
            "https://github.com/mikefarah/yq/releases/download/v{source_version}/yq_{source_system}.tar.gz",
        );

        let source = ArtifactSource::new(name, &source_path).build();

        let step_script = formatdoc! {"
            mkdir -pv \"$VORPAL_OUTPUT/bin\"
            pushd ./source/{name}
            cp yq_{source_system} \"$VORPAL_OUTPUT/bin/yq\"
            chmod +x \"$VORPAL_OUTPUT/bin/yq\"",
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
