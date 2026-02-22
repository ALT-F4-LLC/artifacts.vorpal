use anyhow::Result;
use indoc::formatdoc;
use vorpal_sdk::{
    api::artifact::ArtifactSystem::{Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux},
    artifact::{step, Artifact, ArtifactSource},
    context::ConfigContext,
};

#[derive(Default)]
pub struct Glow;

impl Glow {
    pub fn new() -> Self {
        Self
    }

    pub async fn build(self, context: &mut ConfigContext) -> Result<String> {
        let name = "glow";
        let source_version = "2.1.1";

        let source_system = match context.get_system() {
            Aarch64Darwin => "Darwin_arm64",
            Aarch64Linux => "Linux_arm64",
            X8664Darwin => "Darwin_x86_64",
            X8664Linux => "Linux_x86_64",
            _ => return Err(anyhow::anyhow!("Unsupported system for glow artifact")),
        };

        let source_path = format!(
            "https://github.com/charmbracelet/glow/releases/download/v{source_version}/glow_{source_version}_{source_system}.tar.gz"
        );

        let source = ArtifactSource::new(name, &source_path).build();

        let step_script = formatdoc! {"
            mkdir -pv \"$VORPAL_OUTPUT/bin\"
            pushd ./source/{name}
            cp glow_{source_version}_{source_system}/glow \"$VORPAL_OUTPUT/bin/glow\"
            chmod +x \"$VORPAL_OUTPUT/bin/glow\"",
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
