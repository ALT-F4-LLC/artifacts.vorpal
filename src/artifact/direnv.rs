use anyhow::Result;
use indoc::formatdoc;
use vorpal_sdk::{
    api::artifact::ArtifactSystem::{Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux},
    artifact::{step, ArtifactBuilder, ArtifactSourceBuilder},
    context::ConfigContext,
};

pub async fn build(context: &mut ConfigContext) -> Result<String> {
    let name = "direnv";
    let source_version = "v2.37.1";

    let source_system = match context.get_system() {
        X8664Darwin => "darwin-amd64",
        Aarch64Darwin => "darwin-arm64",
        X8664Linux => "linux-amd64",
        Aarch64Linux => "linux-arm64",
        _ => return Err(anyhow::anyhow!("Unsupported system for direnv artifact")),
    };

    let step_script = formatdoc! {"
        mkdir -pv \"$VORPAL_OUTPUT/bin\"
        curl -L \"https://github.com/direnv/direnv/releases/download/{source_version}/direnv.{source_system}\" -o \"$VORPAL_OUTPUT/bin/direnv\"
        chmod +x \"$VORPAL_OUTPUT/bin/direnv\"",
    };

    let steps = vec![step::shell(context, vec![], vec![], step_script, vec![]).await?];

    let systems = vec![Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux];

    ArtifactBuilder::new(name, steps, systems)
        .with_aliases(vec![format!("{name}:{source_version}")])
        .build(context)
        .await
}

