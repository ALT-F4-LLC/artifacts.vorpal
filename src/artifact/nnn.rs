use crate::artifact::ncurses;
use anyhow::Result;
use indoc::formatdoc;
use vorpal_sdk::{
    api::artifact::ArtifactSystem::{Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux},
    artifact::{get_env_key, step, Artifact, ArtifactSource},
    context::ConfigContext,
};

pub async fn build(context: &mut ConfigContext) -> Result<String> {
    let ncurses = ncurses::build(context).await?;

    let name = "nnn";
    let version = "5.1";

    let path = format!("https://github.com/jarun/nnn/archive/refs/tags/v{version}.tar.gz");

    let source = ArtifactSource::new(name, &path).build();

    let script = formatdoc! {"
        mkdir -pv \"$VORPAL_OUTPUT\"

        pushd ./source/{name}/nnn-{version}

        export CPPFLAGS=\"-I{ncurses}/include -I{ncurses}/include/ncursesw\"
        export LDFLAGS=\"-L{ncurses}/lib -Wl,-rpath,{ncurses}/lib\"
        export PKG_CONFIG_PATH=\"{ncurses}/lib/pkgconfig\"

        make PREFIX=\"$VORPAL_OUTPUT\"
        make PREFIX=\"$VORPAL_OUTPUT\" install",
        ncurses = get_env_key(&ncurses),
    };

    let steps = vec![step::shell(context, vec![ncurses], vec![], script, vec![]).await?];

    let systems = vec![Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux];

    Artifact::new(name, steps, systems)
        .with_aliases(vec![format!("{name}:{version}")])
        .with_sources(vec![source])
        .build(context)
        .await
}
