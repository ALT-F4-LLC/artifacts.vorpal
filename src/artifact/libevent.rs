use anyhow::Result;
use indoc::formatdoc;
use vorpal_sdk::{
    api::artifact::ArtifactSystem::{Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux},
    artifact::{step, ArtifactBuilder, ArtifactSourceBuilder},
    context::ConfigContext,
};

pub async fn build(context: &mut ConfigContext) -> Result<String> {
    let name = "libevent";
    let version = "2.1.12";

    let path = format!(
        "https://github.com/libevent/libevent/releases/download/release-{version}-stable/libevent-{version}-stable.tar.gz"
    );

    let source = ArtifactSourceBuilder::new(name, &path).build();

    let script = formatdoc! {"
        mkdir -pv \"$VORPAL_OUTPUT\"
        pushd ./source/{name}/{name}-{version}-stable
        ./configure \
            --disable-openssl \
            --enable-shared \
            --prefix=\"$VORPAL_OUTPUT\"
        make
        make install",
    };

    let steps = vec![step::shell(context, vec![], vec![], script, vec![]).await?];

    let systems = vec![Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux];

    ArtifactBuilder::new(name, steps, systems)
        .with_aliases(vec![format!("{name}:{version}")])
        .with_sources(vec![source])
        .build(context)
        .await
}
