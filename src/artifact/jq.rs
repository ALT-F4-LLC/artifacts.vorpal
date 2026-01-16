use anyhow::Result;
use indoc::formatdoc;
use vorpal_sdk::{
    api::artifact::ArtifactSystem::{Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux},
    artifact::{step, Artifact, ArtifactSource},
    context::ConfigContext,
};

pub async fn build(context: &mut ConfigContext) -> Result<String> {
    let name = "jq";
    let source_version = "1.8.1";

    let source_system = match context.get_system() {
        Aarch64Darwin => "macos-arm64",
        Aarch64Linux => "linux-arm64",
        X8664Darwin => "macos-amd64",
        X8664Linux => "linux-amd64",
        _ => return Err(anyhow::anyhow!("Unsupported system for jq artifact")),
    };

    let source_path = format!(
        "https://github.com/jqlang/jq/releases/download/jq-{source_version}/jq-{source_system}"
    );

    let source = ArtifactSource::new(name, &source_path).build();

    let step_script = formatdoc! {"
        mkdir -pv \"$VORPAL_OUTPUT/bin\"
        cp ./source/{name}/jq-{source_system} \"$VORPAL_OUTPUT/bin/jq\"
        chmod +x \"$VORPAL_OUTPUT/bin/jq\"",
    };

    let steps = vec![step::shell(context, vec![], vec![], step_script, vec![]).await?];

    let systems = vec![Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux];

    Artifact::new(name, steps, systems)
        .with_aliases(vec![format!("{name}:{source_version}")])
        .with_sources(vec![source])
        .build(context)
        .await
}
