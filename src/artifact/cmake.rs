use anyhow::Result;
use indoc::formatdoc;
use vorpal_sdk::{
    api::artifact::ArtifactSystem::{Aarch64Darwin, X8664Darwin},
    artifact::{step, Artifact, ArtifactSource},
    context::ConfigContext,
};

#[derive(Default)]
pub struct Cmake;

impl Cmake {
    pub fn new() -> Self {
        Self
    }

    pub async fn build(self, context: &mut ConfigContext) -> Result<String> {
        let name = "cmake";
        let version = "4.2.3";

        let path = format!(
            "https://github.com/Kitware/CMake/releases/download/v{version}/cmake-{version}-macos-universal.tar.gz"
        );

        let source = ArtifactSource::new(name, &path).build();

        let script = formatdoc! {"
            mkdir -pv \"$VORPAL_OUTPUT/bin\"
            cp -v ./source/{name}/{name}-{version}-macos-universal/CMake.app/Contents/bin/* \"$VORPAL_OUTPUT/bin/\"
            cp -rv ./source/{name}/{name}-{version}-macos-universal/CMake.app/Contents/share \"$VORPAL_OUTPUT/share\"",
        };

        let steps = vec![step::shell(context, vec![], vec![], script, vec![]).await?];

        let systems = vec![Aarch64Darwin, X8664Darwin];

        Artifact::new(name, steps, systems)
            .with_aliases(vec![format!("{name}:{version}")])
            .with_sources(vec![source])
            .build(context)
            .await
    }
}
