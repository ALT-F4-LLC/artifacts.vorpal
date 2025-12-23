use anyhow::Result;
use indoc::formatdoc;
use vorpal_sdk::{
    api::artifact::ArtifactSystem::{Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux},
    artifact::{step, Artifact, ArtifactSource},
    context::ConfigContext,
};

pub async fn build(context: &mut ConfigContext) -> Result<String> {
    let name = "ncurses";
    let version = "6.5";

    let path = format!("https://invisible-island.net/archives/ncurses/ncurses-{version}.tar.gz");
    let source = ArtifactSource::new(name, &path).build();

    let step_script = formatdoc! {"
        mkdir -pv \"$VORPAL_OUTPUT\"
        pushd ./source/{name}/{name}-{version}
        ./configure \
            --enable-pc-files \
            --prefix=\"$VORPAL_OUTPUT\" \
            --with-pkg-config-libdir=\"$VORPAL_OUTPUT/lib/pkgconfig\" \
            --with-shared \
            --with-termlib
        make
        make install",
    };

    let steps = vec![step::shell(context, vec![], vec![], step_script, vec![]).await?];
    let systems = vec![Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux];

    Artifact::new(name, steps, systems)
        .with_aliases(vec![format!("{name}:{version}")])
        .with_sources(vec![source])
        .build(context)
        .await
}
