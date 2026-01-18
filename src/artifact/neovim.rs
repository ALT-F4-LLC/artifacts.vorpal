use anyhow::Result;
use indoc::formatdoc;
use vorpal_sdk::{
    api::artifact::ArtifactSystem::{Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux},
    artifact::{step, Artifact, ArtifactSource},
    context::ConfigContext,
};

#[derive(Default)]
pub struct Neovim;

impl Neovim {
    pub fn new() -> Self {
        Self
    }

    pub async fn build(self, context: &mut ConfigContext) -> Result<String> {
        let name = "neovim";
        let source_version = "0.11.5";

        let source_system = match context.get_system() {
            Aarch64Darwin => "macos-arm64",
            Aarch64Linux => "linux-arm64",
            X8664Darwin => "macos-x86_64",
            X8664Linux => "linux-x86_64",
            _ => return Err(anyhow::anyhow!("Unsupported system for neovim artifact")),
        };

        let source_path = format!(
            "https://github.com/neovim/neovim/releases/download/v{source_version}/nvim-{source_system}.tar.gz"
        );

        let source = ArtifactSource::new(name, &source_path).build();

        let step_script = formatdoc! {"
            pushd ./source/{name}/nvim-{source_system}
            cp -Rv * \"$VORPAL_OUTPUT/.\"",
        };

        let steps = vec![step::shell(context, vec![], vec![], step_script, vec![]).await?];

        let systems = vec![Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux];

        Artifact::new(name, steps, systems)
            .with_aliases(vec![format!("{name}:{source_version}")])
            .with_sources(vec![source])
            .build(context)
            .await
    }
}
