use anyhow::Result;
use indoc::formatdoc;
use vorpal_sdk::{
    api::artifact::ArtifactSystem::{Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux},
    artifact::{step, Artifact, ArtifactSource},
    context::ConfigContext,
};

#[derive(Default)]
pub struct Openjdk;

impl Openjdk {
    pub fn new() -> Self {
        Self
    }

    pub async fn build(self, context: &mut ConfigContext) -> Result<String> {
        let name = "openjdk";
        let source_version = "25.0.1";

        let source_system = match context.get_system() {
            Aarch64Darwin => "macos-aarch64",
            Aarch64Linux => "linux-aarch64",
            X8664Darwin => "macos-x64",
            X8664Linux => "linux-x64",
            _ => return Err(anyhow::anyhow!("Unsupported system for openjdk artifact")),
        };

        let source_path = format!(
            "https://download.java.net/java/GA/jdk25.0.1/2fbf10d8c78e40bd87641c434705079d/8/GPL/openjdk-{source_version}_{source_system}_bin.tar.gz"
        );

        let source = ArtifactSource::new(name, &source_path).build();
        let source_affix = match context.get_system() {
            Aarch64Darwin | X8664Darwin => ".jdk",
            _ => "",
        };

        let step_script = formatdoc! {"
            pushd ./source/{name}/jdk-{source_version}{source_affix}
            cp -Rv * \"$VORPAL_OUTPUT/.\""
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
