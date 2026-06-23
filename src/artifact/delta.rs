use anyhow::Result;
use indoc::formatdoc;
use vorpal_sdk::{
    api::artifact::ArtifactSystem::{Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux},
    artifact::{step, Artifact, ArtifactSource},
    context::ConfigContext,
};

#[derive(Default)]
pub struct Delta;

impl Delta {
    pub fn new() -> Self {
        Self
    }

    pub async fn build(self, context: &mut ConfigContext) -> Result<String> {
        let name = "delta";
        let version = "0.18.2";

        let source_system = match context.get_system() {
            Aarch64Darwin => "aarch64-apple-darwin",
            Aarch64Linux => "aarch64-unknown-linux-gnu",
            X8664Darwin => "x86_64-apple-darwin",
            X8664Linux => "x86_64-unknown-linux-musl",
            _ => return Err(anyhow::anyhow!("Unsupported system for {name} artifact")),
        };

        let source_path = format!(
            "https://github.com/dandavison/delta/releases/download/{version}/{name}-{version}-{source_system}.tar.gz"
        );

        let source = ArtifactSource::new(name, &source_path).build();

        let step_script = formatdoc! {"
            mkdir -pv \"$VORPAL_OUTPUT/bin\"
            pushd ./source/{name}/{name}-{version}-{source_system}
            cp delta \"$VORPAL_OUTPUT/bin/delta\"
            chmod +x \"$VORPAL_OUTPUT/bin/delta\"",
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
