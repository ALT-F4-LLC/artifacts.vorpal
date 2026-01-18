use crate::artifact::{ncurses::Ncurses, pkg_config::PkgConfig, readline::Readline};
use anyhow::Result;
use indoc::formatdoc;
use vorpal_sdk::{
    api::artifact::ArtifactSystem::{Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux},
    artifact::{get_env_key, step, Artifact, ArtifactSource},
    context::ConfigContext,
};

#[derive(Default)]
pub struct Nnn<'a> {
    ncurses: Option<&'a str>,
    pkg_config: Option<&'a str>,
    readline: Option<&'a str>,
}

impl<'a> Nnn<'a> {
    pub fn new() -> Self {
        Self {
            ncurses: None,
            pkg_config: None,
            readline: None,
        }
    }

    pub fn with_ncurses(mut self, ncurses: &'a str) -> Self {
        self.ncurses = Some(ncurses);
        self
    }

    pub fn with_pkg_config(mut self, pkg_config: &'a str) -> Self {
        self.pkg_config = Some(pkg_config);
        self
    }

    pub fn with_readline(mut self, readline: &'a str) -> Self {
        self.readline = Some(readline);
        self
    }

    pub async fn build(self, context: &mut ConfigContext) -> Result<String> {
        let ncurses = match self.ncurses {
            Some(val) => val,
            None => &Ncurses::new().build(context).await?,
        };

        let pkg_config = match self.pkg_config {
            Some(val) => val,
            None => &PkgConfig::new().build(context).await?,
        };

        let readline = match self.readline {
            Some(val) => val,
            None => &Readline::new().with_ncurses(ncurses).build(context).await?,
        };

        let name = "nnn";
        let version = "5.1";

        let path = format!("https://github.com/jarun/nnn/archive/refs/tags/v{version}.tar.gz");

        let source = ArtifactSource::new(name, &path).build();

        let script = formatdoc! {"
            mkdir -pv \"$VORPAL_OUTPUT\"

            pushd ./source/{name}/nnn-{version}

            export PATH=\"{pkg_config}/bin:$PATH\"
            export CPPFLAGS=\"-I{ncurses}/include -I{ncurses}/include/ncursesw -I{readline}/include\"
            export LDFLAGS=\"-L{ncurses}/lib -L{readline}/lib -Wl,-rpath,{ncurses}/lib -Wl,-rpath,{readline}/lib\"
            export PKG_CONFIG_PATH=\"{ncurses}/lib/pkgconfig:{readline}/lib/pkgconfig\"

            make PREFIX=\"$VORPAL_OUTPUT\"
            make PREFIX=\"$VORPAL_OUTPUT\" install",
            ncurses = get_env_key(&ncurses.to_string()),
            pkg_config = get_env_key(&pkg_config.to_string()),
            readline = get_env_key(&readline.to_string()),
        };

        let steps = vec![
            step::shell(
                context,
                vec![
                    ncurses.to_string(),
                    pkg_config.to_string(),
                    readline.to_string(),
                ],
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
