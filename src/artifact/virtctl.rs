use anyhow::Result;
use indoc::formatdoc;
use vorpal_sdk::{
    api::artifact::ArtifactSystem::{Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux},
    artifact::{step, Artifact, ArtifactSource},
    context::ConfigContext,
};

#[derive(Default)]
pub struct Virtctl;

impl Virtctl {
    pub fn new() -> Self {
        Self
    }

    pub async fn build(self, context: &mut ConfigContext) -> Result<String> {
        let name = "virtctl";
        let source_version = "1.8.4";

        let source_system = match context.get_system() {
            Aarch64Darwin => "darwin-arm64",
            Aarch64Linux => "linux-arm64",
            X8664Darwin => "darwin-amd64",
            X8664Linux => "linux-amd64",
            _ => return Err(anyhow::anyhow!("Unsupported system for {name} artifact")),
        };

        let source_file = format!("{name}-v{source_version}-{source_system}");

        let source_path = format!(
            "https://github.com/kubevirt/kubevirt/releases/download/v{source_version}/{source_file}"
        );

        let source = ArtifactSource::new(name, &source_path).build();

        let step_script = formatdoc! {"
            mkdir -pv \"$VORPAL_OUTPUT/bin\"
            cp ./source/{name}/{source_file} \"$VORPAL_OUTPUT/bin/{name}\"
            chmod +x \"$VORPAL_OUTPUT/bin/{name}\"",
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
