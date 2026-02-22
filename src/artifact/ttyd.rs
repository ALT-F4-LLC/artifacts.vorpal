use anyhow::Result;
use indoc::formatdoc;
use vorpal_sdk::{
    api::artifact::ArtifactSystem::{Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux},
    artifact::{step, Artifact, ArtifactSource},
    context::ConfigContext,
};

#[derive(Default)]
pub struct Ttyd;

impl Ttyd {
    pub fn new() -> Self {
        Self
    }

    pub async fn build(self, context: &mut ConfigContext) -> Result<String> {
        let name = "ttyd";
        let source_version = "1.7.7";

        let (source_path, step_script) = match context.get_system() {
            Aarch64Linux => {
                let path = format!(
                    "https://github.com/tsl0922/ttyd/releases/download/{source_version}/ttyd.aarch64"
                );
                let script = formatdoc! {"
                    mkdir -pv \"$VORPAL_OUTPUT/bin\"
                    cp ./source/{name}/ttyd.aarch64 \"$VORPAL_OUTPUT/bin/ttyd\"
                    chmod +x \"$VORPAL_OUTPUT/bin/ttyd\""
                };
                (path, script)
            }
            X8664Linux => {
                let path = format!(
                    "https://github.com/tsl0922/ttyd/releases/download/{source_version}/ttyd.x86_64"
                );
                let script = formatdoc! {"
                    mkdir -pv \"$VORPAL_OUTPUT/bin\"
                    cp ./source/{name}/ttyd.x86_64 \"$VORPAL_OUTPUT/bin/ttyd\"
                    chmod +x \"$VORPAL_OUTPUT/bin/ttyd\""
                };
                (path, script)
            }
            Aarch64Darwin | X8664Darwin => {
                let path = format!(
                    "https://github.com/tsl0922/ttyd/releases/download/{source_version}/ttyd_darwin.zip"
                );
                let script = formatdoc! {"
                    mkdir -pv \"$VORPAL_OUTPUT/bin\"
                    pushd ./source/{name}
                    cp ttyd \"$VORPAL_OUTPUT/bin/ttyd\"
                    chmod +x \"$VORPAL_OUTPUT/bin/ttyd\""
                };
                (path, script)
            }
            _ => return Err(anyhow::anyhow!("Unsupported system for ttyd artifact")),
        };

        let source = ArtifactSource::new(name, &source_path).build();

        let steps = vec![step::shell(context, vec![], vec![], step_script, vec![]).await?];

        let systems = vec![Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux];

        Artifact::new(name, steps, systems)
            .with_aliases(vec![format!("{name}:{source_version}")])
            .with_sources(vec![source])
            .build(context)
            .await
    }
}
