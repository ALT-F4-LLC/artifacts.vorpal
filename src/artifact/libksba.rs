use crate::artifact::libgpg_error;
use anyhow::Result;
use indoc::formatdoc;
use vorpal_sdk::{
    api::artifact::ArtifactSystem::{Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux},
    artifact::{get_env_key, step, Artifact, ArtifactSource},
    context::ConfigContext,
};

pub async fn build(context: &mut ConfigContext) -> Result<String> {
    let libgpg_error = libgpg_error::build(context).await?;

    let name = "libksba";
    let version = "1.6.7";

    let path = format!("https://gnupg.org/ftp/gcrypt/libksba/libksba-{version}.tar.bz2");

    let source = ArtifactSource::new(name, &path).build();

    let script = formatdoc! {"
        mkdir -pv \"$VORPAL_OUTPUT\"

        pushd ./source/{name}/libksba-{version}

        export PATH=\"{libgpg_error}/bin:$PATH\"
        export CPPFLAGS=\"-I{libgpg_error}/include\"
        export LDFLAGS=\"-L{libgpg_error}/lib -Wl,-rpath,{libgpg_error}/lib\"

        ./configure --prefix=\"$VORPAL_OUTPUT\" --with-libgpg-error-prefix={libgpg_error}

        make
        make install",
        libgpg_error = get_env_key(&libgpg_error),
    };

    let steps = vec![step::shell(context, vec![libgpg_error], vec![], script, vec![]).await?];

    let systems = vec![Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux];

    Artifact::new(name, steps, systems)
        .with_aliases(vec![format!("{name}:{version}")])
        .with_sources(vec![source])
        .build(context)
        .await
}
