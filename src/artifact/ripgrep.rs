use anyhow::Result;
use indoc::formatdoc;
use vorpal_sdk::{
    api::artifact::ArtifactSystem::{Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux},
    artifact::{step, ArtifactBuilder, ArtifactSourceBuilder},
    context::ConfigContext,
};

pub async fn build(context: &mut ConfigContext) -> Result<String> {
    let name = "ripgrep";
    let source_version = "14.1.1";

    let source_system = match context.get_system() {
        X8664Darwin => "x86_64-apple-darwin",
        Aarch64Darwin => "aarch64-apple-darwin",
        X8664Linux => "x86_64-unknown-linux-musl",
        Aarch64Linux => "aarch64-unknown-linux-gnu",
        _ => return Err(anyhow::anyhow!("Unsupported system for ripgrep artifact")),
    };

    let source_path = format!(
        "https://github.com/BurntSushi/ripgrep/releases/download/{source_version}/ripgrep-{source_version}-{source_system}.tar.gz"
    );

    let source = ArtifactSourceBuilder::new(name, &source_path).build();

    let step_script = formatdoc! {"
        mkdir -pv \"$VORPAL_OUTPUT/bin\"
        pushd ./source/{name}
        cp */rg \"$VORPAL_OUTPUT/bin/rg\"
        chmod +x \"$VORPAL_OUTPUT/bin/rg\"",
    };

    let steps = vec![step::shell(context, vec![], vec![], step_script, vec![]).await?];

    let systems = vec![Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux];

    ArtifactBuilder::new(name, steps, systems)
        .with_aliases(vec![format!("{name}:{source_version}")])
        .with_sources(vec![source])
        .build(context)
        .await
}

