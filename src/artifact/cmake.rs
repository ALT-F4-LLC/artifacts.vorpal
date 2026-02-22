use anyhow::Result;
use indoc::formatdoc;
use vorpal_sdk::{
    api::artifact::ArtifactSystem::{Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux},
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

        let source_system = match context.get_system() {
            Aarch64Darwin | X8664Darwin => "macos-universal",
            Aarch64Linux => "linux-aarch64",
            X8664Linux => "linux-x86_64",
            _ => return Err(anyhow::anyhow!("Unsupported system for cmake artifact")),
        };

        let path = format!(
            "https://github.com/Kitware/CMake/releases/download/v{version}/cmake-{version}-{source_system}.tar.gz"
        );

        let source = ArtifactSource::new(name, &path).build();

        let script = match context.get_system() {
            Aarch64Darwin | X8664Darwin => formatdoc! {"
                mkdir -pv \"$VORPAL_OUTPUT/bin\"
                cp -v ./source/{name}/{name}-{version}-{source_system}/CMake.app/Contents/bin/* \"$VORPAL_OUTPUT/bin/\"
                cp -rv ./source/{name}/{name}-{version}-{source_system}/CMake.app/Contents/share \"$VORPAL_OUTPUT/share\"",
            },
            _ => formatdoc! {"
                mkdir -pv \"$VORPAL_OUTPUT/bin\"
                cp -v ./source/{name}/{name}-{version}-{source_system}/bin/* \"$VORPAL_OUTPUT/bin/\"
                cp -rv ./source/{name}/{name}-{version}-{source_system}/share \"$VORPAL_OUTPUT/share\"",
            },
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
