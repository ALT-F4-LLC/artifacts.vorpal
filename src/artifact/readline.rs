use anyhow::Result;
use indoc::formatdoc;
use vorpal_sdk::{
    api::artifact::ArtifactSystem::{Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux},
    artifact::{get_env_key, step, Artifact, ArtifactSource},
    context::ConfigContext,
};

pub async fn build(context: &mut ConfigContext, ncurses: &String) -> Result<String> {
    let name = "readline";
    let version = "8.2";

    let path = format!("https://ftpmirror.gnu.org/readline/readline-{version}.tar.gz");
    let source = ArtifactSource::new(name, &path).build();

    let step_script = formatdoc! {"
        mkdir -pv \"$VORPAL_OUTPUT\"
        pushd ./source/{name}/{name}-{version}

        export CPPFLAGS=\"-I{ncurses}/include -I{ncurses}/include/ncursesw\"
        export LDFLAGS=\"-L{ncurses}/lib -Wl,-rpath,{ncurses}/lib\"

        ./configure \
            --prefix=\"$VORPAL_OUTPUT\" \
            --with-curses

        make
        make install",
        ncurses = get_env_key(ncurses),
    };

    let steps =
        vec![step::shell(context, vec![ncurses.clone()], vec![], step_script, vec![]).await?];
    let systems = vec![Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux];

    Artifact::new(name, steps, systems)
        .with_aliases(vec![format!("{name}:{version}")])
        .with_sources(vec![source])
        .build(context)
        .await
}
