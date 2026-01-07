use anyhow::Result;
use indoc::formatdoc;
use vorpal_sdk::{
    api::artifact::ArtifactSystem::{Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux},
    artifact::{step, Artifact, ArtifactSource},
    context::ConfigContext,
};

pub async fn build(context: &mut ConfigContext) -> Result<String> {
    let name = "pkg-config";

    let source_version = "0.29.2";

    let source_path =
        format!("https://pkgconfig.freedesktop.org/releases/pkg-config-{source_version}.tar.gz");

    let source = ArtifactSource::new(name, source_path.as_str()).build();

    let step_script = formatdoc! {"
        mkdir -pv \"$VORPAL_OUTPUT/bin\"

        pushd ./source/{name}/pkg-config-{source_version}

        CFLAGS=\"-Wno-error=int-conversion\" ./configure --prefix=$VORPAL_OUTPUT --with-internal-glib

        make
        make install",
    };

    let steps = vec![step::shell(context, vec![], vec![], step_script, vec![]).await?];

    let systems = vec![Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux];

    Artifact::new(name, steps, systems)
        .with_aliases(vec![format!("{name}:{source_version}")])
        .with_sources(vec![source])
        .build(context)
        .await
}
