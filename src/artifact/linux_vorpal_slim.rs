use anyhow::Result;
use indoc::formatdoc;
use vorpal_sdk::{
    api::artifact::ArtifactSystem::{Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux},
    artifact::{step, Artifact, ArtifactSource},
    context::ConfigContext,
};

pub async fn build(context: &mut ConfigContext, linux_vorpal: &str) -> Result<String> {
    let name = "linux-vorpal-slim";
    let source_version = "latest";

    let source = ArtifactSource::new(name, ".")
        .with_includes(vec!["script/linux-vorpal-slim/clean.sh".to_string()])
        .build();

    let step_script = formatdoc! {"
        mkdir -pv \"$VORPAL_OUTPUT/bin\"

        pushd ./source/{name}

        ls -alh",
    };

    let artifacts = vec![linux_vorpal.to_string()];

    let steps = vec![step::shell(context, artifacts, vec![], step_script, vec![]).await?];

    let systems = vec![Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux];

    Artifact::new(name, steps, systems)
        .with_aliases(vec![format!("{name}:{source_version}")])
        .with_sources(vec![source])
        .build(context)
        .await
}
