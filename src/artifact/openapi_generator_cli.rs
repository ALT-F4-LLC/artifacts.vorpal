use crate::artifact::openjdk::Openjdk;
use anyhow::Result;
use indoc::formatdoc;
use vorpal_sdk::{
    api::artifact::ArtifactSystem::{Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux},
    artifact::{get_env_key, step, Artifact, ArtifactSource},
    context::ConfigContext,
};

#[derive(Default)]
pub struct OpenapiGeneratorCli<'a> {
    openjdk: Option<&'a str>,
}

impl<'a> OpenapiGeneratorCli<'a> {
    pub fn new() -> Self {
        Self { openjdk: None }
    }

    pub fn with_openjdk(mut self, openjdk: &'a str) -> Self {
        self.openjdk = Some(openjdk);
        self
    }

    pub async fn build(self, context: &mut ConfigContext) -> Result<String> {
        let openjdk = match self.openjdk {
            Some(val) => val,
            None => &Openjdk::new().build(context).await?,
        };

        let name = "openapi-generator-cli";
        let source_version = "7.18.0";

        let source_path = format!(
            "https://repo1.maven.org/maven2/org/openapitools/openapi-generator-cli/{source_version}/openapi-generator-cli-{source_version}.jar"
        );

        let source = ArtifactSource::new(name, &source_path).build();

        let env_openjdk = get_env_key(&openjdk.to_string());

        let step_script = formatdoc! {"
            mkdir -p \"$VORPAL_OUTPUT/bin\"

            pushd ./source/{name}

            cp META-INF/MANIFEST.MF ../MANIFEST.MF

            jar cfm ../openapi-generator-cli.jar ../MANIFEST.MF .

            mv -v ../openapi-generator-cli.jar \"$VORPAL_OUTPUT/openapi-generator-cli.jar\"

            cat << 'EOF' > \"$VORPAL_OUTPUT/bin/openapi-generator-cli\"
            #!/bin/sh
            JAVA_HOME={env_openjdk}/Contents/Home
            PATH=$JAVA_HOME/bin:$PATH
            java -jar \"$VORPAL_OUTPUT/openapi-generator-cli.jar\" \"$@\"
            EOF

            chmod +x \"$VORPAL_OUTPUT/bin/openapi-generator-cli\""
        };

        let environments = [
            format!("JAVA_HOME={env_openjdk}/Contents/Home"),
            "PATH=$JAVA_HOME/bin:$PATH".to_string(),
        ];

        let steps = vec![
            step::shell(
                context,
                vec![openjdk.to_string()],
                environments.to_vec(),
                step_script,
                vec![],
            )
            .await?,
        ];

        let systems = vec![Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux];

        Artifact::new(name, steps, systems)
            .with_aliases(vec![format!("{name}:{source_version}")])
            .with_sources(vec![source])
            .build(context)
            .await
    }
}
