use crate::artifact::ncurses::Ncurses;
use anyhow::Result;
use indoc::formatdoc;
use vorpal_sdk::{
    api::artifact::ArtifactSystem::{Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux},
    artifact::{get_env_key, step, Artifact, ArtifactSource},
    context::ConfigContext,
};

#[derive(Default)]
pub struct Zsh<'a> {
    ncurses: Option<&'a str>,
}

impl<'a> Zsh<'a> {
    pub fn new() -> Self {
        Self { ncurses: None }
    }

    pub fn with_ncurses(mut self, ncurses: &'a str) -> Self {
        self.ncurses = Some(ncurses);
        self
    }

    pub async fn build(self, context: &mut ConfigContext) -> Result<String> {
        let ncurses = match self.ncurses {
            Some(val) => val,
            None => &Ncurses::new().build(context).await?,
        };

        let name = "zsh";
        let version = "5.9";

        let path = format!(
            "https://downloads.sourceforge.net/project/zsh/zsh/{version}/zsh-{version}.tar.xz"
        );

        let source = ArtifactSource::new(name, &path).build();

        let script = formatdoc! {"
            mkdir -pv \"$VORPAL_OUTPUT\"

            pushd ./source/{name}/zsh-{version}

            export CFLAGS=\"-Wno-incompatible-pointer-types\"
            export CPPFLAGS=\"-I{ncurses}/include -I{ncurses}/include/ncursesw\"
            export LDFLAGS=\"-L{ncurses}/lib -Wl,-rpath,{ncurses}/lib\"

            ./configure --prefix=\"$VORPAL_OUTPUT\"

            make
            make install",
            ncurses = get_env_key(&ncurses.to_string()),
        };

        let steps =
            vec![step::shell(context, vec![ncurses.to_string()], vec![], script, vec![]).await?];

        let systems = vec![Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux];

        Artifact::new(name, steps, systems)
            .with_aliases(vec![format!("{name}:{version}")])
            .with_sources(vec![source])
            .build(context)
            .await
    }
}
