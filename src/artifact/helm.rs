use anyhow::Result;
use indoc::formatdoc;
use vorpal_sdk::{
    api::artifact::ArtifactSystem::{Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux},
    artifact::{step, Artifact, ArtifactSource},
    context::ConfigContext,
};

pub async fn build(context: &mut ConfigContext) -> Result<String> {
    let name = "helm";
    let source_version = "4.0.4";

    let source_system = match context.get_system() {
        Aarch64Darwin => "darwin-arm64",
        Aarch64Linux => "linux-arm64",
        X8664Darwin => "darwin-amd64",
        X8664Linux => "linux-amd64",
        _ => return Err(anyhow::anyhow!("Unsupported system for helm artifact")),
    };

    let source_path = format!("https://get.helm.sh/helm-v{source_version}-{source_system}.tar.gz");

    let source = ArtifactSource::new(name, &source_path).build();

    let step_script = formatdoc! {"
        mkdir -pv \"$VORPAL_OUTPUT/bin\"
        pushd ./source/{name}/{source_system}
        cp helm \"$VORPAL_OUTPUT/bin/helm\"
        chmod +x \"$VORPAL_OUTPUT/bin/helm\"",
    };

    let steps = vec![step::shell(context, vec![], vec![], step_script, vec![]).await?];

    let systems = vec![Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux];

    Artifact::new(name, steps, systems)
        .with_aliases(vec![format!("{name}:{source_version}")])
        .with_sources(vec![source])
        .build(context)
        .await
}
