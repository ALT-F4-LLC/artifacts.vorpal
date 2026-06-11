use anyhow::Result;
use indoc::formatdoc;
use vorpal_sdk::{
    api::artifact::ArtifactSystem::{Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux},
    artifact::{step, Artifact, ArtifactSource},
    context::ConfigContext,
};

#[derive(Default)]
pub struct Talosctl;

impl Talosctl {
    pub fn new() -> Self {
        Self
    }

    pub async fn build(self, context: &mut ConfigContext) -> Result<String> {
        let name = "talosctl";
        let version = "1.13.4";

        let (source_os, source_arch) = match context.get_system() {
            Aarch64Darwin => ("darwin", "arm64"),
            Aarch64Linux => ("linux", "arm64"),
            X8664Darwin => ("darwin", "amd64"),
            X8664Linux => ("linux", "amd64"),
            _ => return Err(anyhow::anyhow!("Unsupported system for talosctl artifact")),
        };

        let source_path = format!(
            "https://github.com/siderolabs/talos/releases/download/v{version}/talosctl-{source_os}-{source_arch}"
        );

        let source = ArtifactSource::new(name, &source_path).build();

        let step_script = formatdoc! {"
            mkdir -pv \"$VORPAL_OUTPUT/bin\"
            cp ./source/{name}/talosctl-{source_os}-{source_arch} \"$VORPAL_OUTPUT/bin/talosctl\"
            chmod +x \"$VORPAL_OUTPUT/bin/talosctl\"",
        };

        let steps = vec![step::shell(context, vec![], vec![], step_script, vec![]).await?];

        let systems = vec![Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux];

        Artifact::new(name, steps, systems)
            .with_aliases(vec![format!("{name}:{version}")])
            .with_sources(vec![source])
            .build(context)
            .await
    }
}
