use anyhow::Result;
use indoc::formatdoc;
use vorpal_sdk::{
    api::artifact::ArtifactSystem::{Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux},
    artifact::{step, Artifact, ArtifactSource},
    context::ConfigContext,
};

pub async fn build(context: &mut ConfigContext) -> Result<String> {
    let name = "k9s";
    let source_version = "0.50.18";

    let source_system = match context.get_system() {
        Aarch64Darwin => "Darwin_arm64",
        Aarch64Linux => "Linux_arm64",
        X8664Darwin => "Darwin_amd64",
        X8664Linux => "Linux_amd64",
        _ => return Err(anyhow::anyhow!("Unsupported system for k9s artifact")),
    };

    let source_path = format!(
        "https://github.com/derailed/k9s/releases/download/v{source_version}/k9s_{source_system}.tar.gz"
    );

    let source = ArtifactSource::new(name, &source_path).build();

    let step_script = formatdoc! {"
        mkdir -pv \"$VORPAL_OUTPUT/bin\"
        pushd ./source/{name}
        cp k9s \"$VORPAL_OUTPUT/bin/k9s\"
        chmod +x \"$VORPAL_OUTPUT/bin/k9s\"",
    };

    let steps = vec![step::shell(context, vec![], vec![], step_script, vec![]).await?];

    let systems = vec![Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux];

    Artifact::new(name, steps, systems)
        .with_aliases(vec![format!("{name}:{source_version}")])
        .with_sources(vec![source])
        .build(context)
        .await
}
