use anyhow::Result;
use indoc::formatdoc;
use vorpal_sdk::{
    api::artifact::ArtifactSystem::{Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux},
    artifact::{step, Artifact, ArtifactSource},
    context::ConfigContext,
};

#[derive(Default)]
pub struct Opencode;

impl Opencode {
    pub fn new() -> Self {
        Self
    }

    pub async fn build(self, context: &mut ConfigContext) -> Result<String> {
        let name = "opencode";
        let version = "1.17.11";

        let source_suffix = match context.get_system() {
            Aarch64Darwin => "darwin-arm64.zip",
            Aarch64Linux => "linux-arm64-musl.tar.gz",
            X8664Darwin => "darwin-x64.zip",
            X8664Linux => "linux-x64-musl.tar.gz",
            _ => return Err(anyhow::anyhow!("Unsupported system for {name} artifact")),
        };

        let source_path = format!(
            "https://github.com/anomalyco/opencode/releases/download/v{version}/{name}-{source_suffix}"
        );

        let source = ArtifactSource::new(name, &source_path).build();

        let step_script = formatdoc! {"
            mkdir -pv \"$VORPAL_OUTPUT/bin\"
            pushd ./source/{name}
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
