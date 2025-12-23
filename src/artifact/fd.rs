use anyhow::Result;
use indoc::formatdoc;
use vorpal_sdk::{
    api::artifact::ArtifactSystem::{Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux},
    artifact::{step, Artifact, ArtifactSource},
    context::ConfigContext,
};

pub async fn build(context: &mut ConfigContext) -> Result<String> {
    let name = "fd";
    let source_version = "10.2.0";

    let source_system = match context.get_system() {
        Aarch64Darwin => "aarch64-apple-darwin",
        Aarch64Linux => "aarch64-unknown-linux-gnu",
        X8664Darwin => "x86_64-apple-darwin",
        X8664Linux => "x86_64-unknown-linux-musl",
        _ => return Err(anyhow::anyhow!("Unsupported system for fd artifact")),
    };

    let source_path = format!(
        "https://github.com/sharkdp/fd/releases/download/v{source_version}/fd-v{source_version}-{source_system}.tar.gz"
    );

    let source = ArtifactSource::new(name, &source_path).build();

    let step_script = formatdoc! {"
        mkdir -pv \"$VORPAL_OUTPUT/bin\"
        pushd ./source/{name}
        cp */fd \"$VORPAL_OUTPUT/bin/fd\"
        chmod +x \"$VORPAL_OUTPUT/bin/fd\"",
    };

    let steps = vec![step::shell(context, vec![], vec![], step_script, vec![]).await?];

    let systems = vec![Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux];

    Artifact::new(name, steps, systems)
        .with_aliases(vec![format!("{name}:{source_version}")])
        .with_sources(vec![source])
        .build(context)
        .await
}
