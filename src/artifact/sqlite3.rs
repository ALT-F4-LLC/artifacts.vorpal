use anyhow::Result;
use indoc::formatdoc;
use vorpal_sdk::{
    api::artifact::ArtifactSystem::{Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux},
    artifact::{step, Artifact, ArtifactSource},
    context::ConfigContext,
};

#[derive(Default)]
pub struct Sqlite3;

impl Sqlite3 {
    pub fn new() -> Self {
        Self
    }

    pub async fn build(self, context: &mut ConfigContext) -> Result<String> {
        let name = "sqlite3";
        let version = "3.51.2";
        let version_tag = "3510200";
        let year = "2026";

        let path = format!("https://www.sqlite.org/{year}/sqlite-autoconf-{version_tag}.tar.gz");

        let source = ArtifactSource::new(name, &path).build();

        let script = formatdoc! {"
            mkdir -pv \"$VORPAL_OUTPUT\"
            pushd ./source/{name}/sqlite-autoconf-{version_tag}
            ./configure --prefix=\"$VORPAL_OUTPUT\"
            make
            make install",
        };

        let steps = vec![step::shell(context, vec![], vec![], script, vec![]).await?];

        let systems = vec![Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux];

        Artifact::new(name, steps, systems)
            .with_aliases(vec![format!("{name}:{version}")])
            .with_sources(vec![source])
            .build(context)
            .await
    }
}
