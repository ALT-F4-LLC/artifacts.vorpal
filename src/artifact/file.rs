use anyhow::Result;
use indoc::formatdoc;
use vorpal_sdk::{
    api::artifact::ArtifactSystem,
    artifact::{step, Artifact},
    context::ConfigContext,
};

pub struct File<'a> {
    content: &'a str,
    name: &'a str,
    systems: Vec<ArtifactSystem>,
}

impl<'a> File<'a> {
    pub fn new(content: &'a str, name: &'a str, systems: Vec<ArtifactSystem>) -> Self {
        Self {
            content,
            name,
            systems,
        }
    }

    pub async fn build(self, context: &mut ConfigContext) -> Result<String> {
        let step_script = formatdoc! {"
            #!/bin/bash
            set -euo pipefail

            cat << 'EOF' > $VORPAL_OUTPUT/{name}
            {contents}
            EOF

            chmod 644 $VORPAL_OUTPUT/{name}
        ",
            contents = self.content,
            name = self.name,
        };

        let step = step::shell(context, vec![], vec![], step_script, vec![]).await?;

        Artifact::new(self.name, vec![step], self.systems)
            .build(context)
            .await
    }
}
