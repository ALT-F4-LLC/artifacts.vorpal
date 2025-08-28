use anyhow::Result;
use indoc::formatdoc;
use vorpal_sdk::{
    api::artifact::ArtifactSystem::{Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux},
    artifact::{step, ArtifactBuilder, ArtifactSourceBuilder},
    context::ConfigContext,
};

pub async fn build(context: &mut ConfigContext) -> Result<String> {
    let name = "terraform";
    let source_version = "1.13.1";

    let source_system = match context.get_system() {
        X8664Darwin => "darwin_amd64",
        Aarch64Darwin => "darwin_arm64",
        X8664Linux => "linux_amd64",
        Aarch64Linux => "linux_arm64",
        _ => return Err(anyhow::anyhow!("Unsupported system for terraform artifact")),
    };

    let source_path = format!(
        "https://releases.hashicorp.com/terraform/{source_version}/terraform_{source_version}_{source_system}.zip"
    );

    let source = ArtifactSourceBuilder::new(name, &source_path).build();

    let step_script = formatdoc! {"
        mkdir -pv \"$VORPAL_OUTPUT/bin\"
        pushd ./source/{name}
        cp terraform \"$VORPAL_OUTPUT/bin/terraform\"
        chmod +x \"$VORPAL_OUTPUT/bin/terraform\"",
    };

    let steps = vec![step::shell(context, vec![], vec![], step_script, vec![]).await?];

    let systems = vec![Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux];

    ArtifactBuilder::new(name, steps, systems)
        .with_aliases(vec![format!("{name}:{source_version}")])
        .with_sources(vec![source])
        .build(context)
        .await
}

