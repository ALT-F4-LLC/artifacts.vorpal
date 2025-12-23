use crate::artifact::{libevent, ncurses};
use anyhow::Result;
use indoc::formatdoc;
use vorpal_sdk::{
    api::artifact::ArtifactSystem::{Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux},
    artifact::{get_env_key, step, Artifact, ArtifactSource},
    context::ConfigContext,
};

pub async fn build(context: &mut ConfigContext) -> Result<String> {
    let libevent = libevent::build(context).await?;
    let ncurses = ncurses::build(context).await?;

    let name = "tmux";
    let version = "3.5a";

    let path =
        format!("https://github.com/tmux/tmux/releases/download/{version}/tmux-{version}.tar.gz");

    let source = ArtifactSource::new(name, &path).build();

    let script = formatdoc! {"
        mkdir -pv \"$VORPAL_OUTPUT\"

        pushd ./source/{name}/tmux-{version}

        export CPPFLAGS=\"-I{libevent}/include -I{ncurses}/include -I{ncurses}/include/ncursesw\"
        export LDFLAGS=\"-L{libevent}/lib -L{ncurses}/lib -Wl,-rpath,{libevent}/lib -Wl,-rpath,{ncurses}/lib\"

        ./configure --disable-utf8proc --prefix=\"$VORPAL_OUTPUT\"

        make
        make install",
        libevent = get_env_key(&libevent),
        ncurses = get_env_key(&ncurses),
    };

    let steps = vec![step::shell(context, vec![libevent, ncurses], vec![], script, vec![]).await?];

    let systems = vec![Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux];

    Artifact::new(name, steps, systems)
        .with_aliases(vec![format!("{name}:{version}")])
        .with_sources(vec![source])
        .build(context)
        .await
}
