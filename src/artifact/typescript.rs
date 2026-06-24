use anyhow::Result;
use indoc::formatdoc;
use vorpal_sdk::{
    api::artifact::ArtifactSystem::{Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux},
    artifact::{get_env_key, nodejs::NodeJS, step, Artifact, ArtifactSource},
    context::ConfigContext,
};

#[derive(Default)]
pub struct Typescript<'a> {
    node: Option<&'a str>,
}

impl<'a> Typescript<'a> {
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

        let name = "typescript";
        let source_version = "6.0.3";

        let source_path =
            format!("https://registry.npmjs.org/typescript/-/typescript-{source_version}.tgz");

        let source = ArtifactSource::new(name, &source_path).build();

        let env_node = get_env_key(&node.to_string());

        let step_script = formatdoc! {"
            mkdir -pv \"$VORPAL_OUTPUT/bin\" \"$VORPAL_OUTPUT/lib/node_modules/typescript\"

            pushd ./source/{name}/package

            cp -Rv * \"$VORPAL_OUTPUT/lib/node_modules/typescript/.\"

            popd

            cat << EOF > \"$VORPAL_OUTPUT/bin/tsc\"
            #!/bin/sh
            exec {env_node}/bin/node \"$VORPAL_OUTPUT/lib/node_modules/typescript/bin/tsc\" \"\\$@\"
            EOF

            cat << EOF > \"$VORPAL_OUTPUT/bin/tsserver\"
            #!/bin/sh
            exec {env_node}/bin/node \"$VORPAL_OUTPUT/lib/node_modules/typescript/bin/tsserver\" \"\\$@\"
            EOF

            chmod +x \"$VORPAL_OUTPUT/bin/tsc\" \"$VORPAL_OUTPUT/bin/tsserver\""
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
