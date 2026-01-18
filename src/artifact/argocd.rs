use anyhow::Result;
use indoc::formatdoc;
use vorpal_sdk::{
    api::artifact::ArtifactSystem::{Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux},
    artifact::{step, Artifact, ArtifactSource},
    context::ConfigContext,
};

#[derive(Default)]
pub struct Argocd;

impl Argocd {
    pub fn new() -> Self {
        Self
    }

    pub async fn build(self, context: &mut ConfigContext) -> Result<String> {
        let name = "argocd";
        let source_version = "3.2.3";

        let source_system = match context.get_system() {
            Aarch64Darwin => "darwin-arm64",
            Aarch64Linux => "linux-arm64",
            X8664Darwin => "darwin-amd64",
            X8664Linux => "linux-amd64",
            _ => return Err(anyhow::anyhow!("Unsupported system for argocd artifact")),
        };

        let source_path = format!(
            "https://github.com/argoproj/argo-cd/releases/download/v{source_version}/argocd-{source_system}"
        );

        let source = ArtifactSource::new(name, &source_path).build();

        let step_script = formatdoc! {"
            mkdir -pv \"$VORPAL_OUTPUT/bin\"
            pushd ./source/{name}
            cp argocd-{source_system} \"$VORPAL_OUTPUT/bin/argocd\"
            chmod +x \"$VORPAL_OUTPUT/bin/argocd\"",
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
