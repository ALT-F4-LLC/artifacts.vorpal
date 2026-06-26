use anyhow::Result;
use indoc::formatdoc;
use vorpal_sdk::{
    api::artifact::ArtifactSystem::{Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux},
    artifact::{step, Artifact, ArtifactSource},
    context::ConfigContext,
};

#[derive(Default)]
pub struct Herdr;

impl Herdr {
    pub fn new() -> Self {
        Self
    }

    pub async fn build(self, context: &mut ConfigContext) -> Result<String> {
        let name = "herdr";
        let version = "0.7.1";

        let source_system = match context.get_system() {
            Aarch64Darwin => "macos-aarch64",
            Aarch64Linux => "linux-aarch64",
            X8664Darwin => "macos-x86_64",
            X8664Linux => "linux-x86_64",
            _ => return Err(anyhow::anyhow!("Unsupported system for {name} artifact")),
        };

        let source_path = format!(
            "https://github.com/ogulcancelik/herdr/releases/download/v{version}/{name}-{source_system}"
        );

        let source = ArtifactSource::new(name, &source_path).build();

        let step_script = formatdoc! {"
            mkdir -pv \"$VORPAL_OUTPUT/bin\"
            cp ./source/{name}/{name}-{source_system} \"$VORPAL_OUTPUT/bin/{name}\"
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
