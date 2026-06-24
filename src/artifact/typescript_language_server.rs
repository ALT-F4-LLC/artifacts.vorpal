use anyhow::Result;
use indoc::formatdoc;
use vorpal_sdk::{
    api::artifact::ArtifactSystem::{Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux},
    artifact::{get_env_key, nodejs::NodeJS, step, Artifact, ArtifactSource},
    context::ConfigContext,
};

#[derive(Default)]
pub struct TypescriptLanguageServer<'a> {
    node: Option<&'a str>,
}

impl<'a> TypescriptLanguageServer<'a> {
    pub fn new() -> Self {
        Self { node: None }
    }

    pub fn with_node(mut self, node: &'a str) -> Self {
        self.node = Some(node);
        self
    }

    pub async fn build(self, context: &mut ConfigContext) -> Result<String> {
        let node = match self.node {
            Some(val) => val,
            None => &NodeJS::new().build(context).await?,
        };

        let name = "typescript-language-server";
        let source_version = "5.3.0";

        let source_path = format!(
            "https://registry.npmjs.org/typescript-language-server/-/typescript-language-server-{source_version}.tgz"
        );

        let source = ArtifactSource::new(name, &source_path).build();

        let env_node = get_env_key(&node.to_string());

        let step_script = formatdoc! {"
            mkdir -pv \"$VORPAL_OUTPUT/bin\" \"$VORPAL_OUTPUT/lib/node_modules/typescript-language-server\"

            pushd ./source/{name}/package

            cp -Rv * \"$VORPAL_OUTPUT/lib/node_modules/typescript-language-server/.\"

            popd

            cat << EOF > \"$VORPAL_OUTPUT/bin/typescript-language-server\"
            #!/bin/sh
            exec {env_node}/bin/node \"$VORPAL_OUTPUT/lib/node_modules/typescript-language-server/lib/cli.mjs\" \"\\$@\"
            EOF

            chmod +x \"$VORPAL_OUTPUT/bin/typescript-language-server\""
        };

        let steps =
            vec![step::shell(context, vec![node.to_string()], vec![], step_script, vec![]).await?];

        let systems = vec![Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux];

        Artifact::new(name, steps, systems)
            .with_aliases(vec![format!("{name}:{source_version}")])
            .with_sources(vec![source])
            .build(context)
            .await
    }
}
