use anyhow::Result;
use indoc::formatdoc;
use vorpal_sdk::{
    api::artifact::ArtifactSystem::{Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux},
    artifact::{linux_vorpal::LinuxVorpal, step, Artifact, ArtifactSource},
    context::ConfigContext,
};

#[derive(Default)]
pub struct LinuxVorpalSlim<'a> {
    linux_vorpal: Option<&'a str>,
}

impl<'a> LinuxVorpalSlim<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_linux_vorpal(mut self, linux_vorpal: &'a str) -> Self {
        self.linux_vorpal = Some(linux_vorpal);
        self
    }

    pub async fn build(self, context: &mut ConfigContext) -> Result<String> {
        let name = "linux-vorpal-slim";
        let source_version = "latest";

        let source = ArtifactSource::new(name, ".")
            .with_includes(vec!["script/linux-vorpal-slim.sh".to_string()])
            .build();

        let step_script = formatdoc! {"
            mkdir -pv \"$VORPAL_OUTPUT/bin\"

            pushd ./source/{name}

            ls -alh",
        };

        let linux_vorpal = match self.linux_vorpal {
            Some(val) => val,
            None => &LinuxVorpal::new().build(context).await?,
        };

        let artifacts = vec![linux_vorpal.to_string()];

        let steps = vec![step::shell(context, artifacts, vec![], step_script, vec![]).await?];

        let systems = vec![Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux];

        Artifact::new(name, steps, systems)
            .with_aliases(vec![format!("{name}:{source_version}")])
            .with_sources(vec![source])
            .build(context)
            .await
    }
}
