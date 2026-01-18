use crate::artifact::{libevent::Libevent, ncurses::Ncurses};
use anyhow::Result;
use indoc::formatdoc;
use vorpal_sdk::{
    api::artifact::ArtifactSystem::{Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux},
    artifact::{get_env_key, step, Artifact, ArtifactSource},
    context::ConfigContext,
};

#[derive(Default)]
pub struct Tmux<'a> {
    libevent: Option<&'a str>,
    ncurses: Option<&'a str>,
}

impl<'a> Tmux<'a> {
    pub fn new() -> Self {
        Self {
            libevent: None,
            ncurses: None,
        }
    }

    pub fn with_libevent(mut self, libevent: &'a str) -> Self {
        self.libevent = Some(libevent);
        self
    }

    pub fn with_ncurses(mut self, ncurses: &'a str) -> Self {
        self.ncurses = Some(ncurses);
        self
    }

    pub async fn build(self, context: &mut ConfigContext) -> Result<String> {
        let libevent = match self.libevent {
            Some(val) => val,
            None => &Libevent::new().build(context).await?,
        };

        let ncurses = match self.ncurses {
            Some(val) => val,
            None => &Ncurses::new().build(context).await?,
        };

        let name = "tmux";
        let version = "3.5a";

        let path = format!(
            "https://github.com/tmux/tmux/releases/download/{version}/tmux-{version}.tar.gz"
        );

        let source = ArtifactSource::new(name, &path).build();

        let script = formatdoc! {"
            mkdir -pv \"$VORPAL_OUTPUT\"

            pushd ./source/{name}/tmux-{version}

            export CPPFLAGS=\"-I{libevent}/include -I{ncurses}/include -I{ncurses}/include/ncursesw\"
            export LDFLAGS=\"-L{libevent}/lib -L{ncurses}/lib -Wl,-rpath,{libevent}/lib -Wl,-rpath,{ncurses}/lib\"

            ./configure --disable-utf8proc --prefix=\"$VORPAL_OUTPUT\"

            make
            make install",
            libevent = get_env_key(&libevent.to_string()),
            ncurses = get_env_key(&ncurses.to_string()),
        };

        let steps = vec![
            step::shell(
                context,
                vec![libevent.to_string(), ncurses.to_string()],
                vec![],
                script,
                vec![],
            )
            .await?,
        ];

        let systems = vec![Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux];

        Artifact::new(name, steps, systems)
            .with_aliases(vec![format!("{name}:{version}")])
            .with_sources(vec![source])
            .build(context)
            .await
    }
}
