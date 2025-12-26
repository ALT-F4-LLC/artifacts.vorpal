use anyhow::Result;
use indoc::formatdoc;
use vorpal_sdk::{
    api::artifact::ArtifactSystem::{Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux},
    artifact::{step, Artifact, ArtifactSource},
    context::ConfigContext,
};

pub async fn build(context: &mut ConfigContext) -> Result<String> {
    let name = "golangci-lint";
    let source_version = "2.7.2";

    let source_system = match context.get_system() {
        Aarch64Darwin => "darwin-arm64",
        Aarch64Linux => "linux-arm64",
        X8664Darwin => "darwin-amd64",
        X8664Linux => "linux-amd64",
        _ => {
            return Err(anyhow::anyhow!(
                "Unsupported system for golangci-lint artifact"
            ))
        }
    };

    let source_path = format!(
        "https://github.com/golangci/golangci-lint/releases/download/v{source_version}/golangci-lint-{source_version}-{source_system}.tar.gz"
    );

    let source = ArtifactSource::new(name, &source_path).build();

    let step_script = formatdoc! {"
        mkdir -pv \"$VORPAL_OUTPUT/bin\"
        pushd ./source/{name}/golangci-lint-{source_version}-{source_system}
        cp golangci-lint \"$VORPAL_OUTPUT/bin/golangci-lint\"",
    };

    let steps = vec![step::shell(context, vec![], vec![], step_script, vec![]).await?];

    let systems = vec![Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux];

    Artifact::new(name, steps, systems)
        .with_aliases(vec![format!("{name}:{source_version}")])
        .with_sources(vec![source])
        .build(context)
        .await
}
