use anyhow::Result;
use indoc::formatdoc;
use vorpal_sdk::{
    api::artifact::ArtifactSystem::{Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux},
    artifact::{step, Artifact, ArtifactSource},
    context::ConfigContext,
};

#[derive(Default)]
pub struct Rumdl;

impl Rumdl {
    pub fn new() -> Self {
        Self
    }

    pub async fn build(self, context: &mut ConfigContext) -> Result<String> {
        let name = "rumdl";
        let source_version = "0.3.2";

        let source_system = match context.get_system() {
            Aarch64Darwin => "aarch64-apple-darwin",
            Aarch64Linux => "aarch64-unknown-linux-gnu",
            X8664Darwin => "x86_64-apple-darwin",
            X8664Linux => "x86_64-unknown-linux-musl",
            _ => return Err(anyhow::anyhow!("Unsupported system for rumdl artifact")),
        };

        let source_path = format!(
            "https://github.com/rvben/rumdl/releases/download/v{source_version}/rumdl-v{source_version}-{source_system}.tar.gz"
        );

        let source = ArtifactSource::new(name, &source_path).build();

        let step_script = formatdoc! {"
            mkdir -pv \"$VORPAL_OUTPUT/bin\"
            cp ./source/{name}/rumdl \"$VORPAL_OUTPUT/bin/rumdl\"
            chmod +x \"$VORPAL_OUTPUT/bin/rumdl\"",
        };

        let steps = vec![step::shell(context, &[], &[], step_script, &[]).await?];

        let systems = vec![Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux];

        Artifact::new(name, steps, systems)
            .with_aliases(vec![format!("{name}:{source_version}")])
            .with_sources(vec![source])
            .build(context)
            .await
    }
}
