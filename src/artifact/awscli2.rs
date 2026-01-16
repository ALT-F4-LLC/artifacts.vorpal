use anyhow::Result;
use indoc::formatdoc;
use vorpal_sdk::{
    api::artifact::ArtifactSystem::{Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux},
    artifact::{step, Artifact, ArtifactSource},
    context::ConfigContext,
};

pub async fn build(context: &mut ConfigContext) -> Result<String> {
    let name = "awscli2";
    let source_version = "2.33.1";

    let (source_path, step_script) = match context.get_system() {
        Aarch64Linux => {
            let path = format!(
                "https://awscli.amazonaws.com/awscli-exe-linux-aarch64-{source_version}.zip"
            );
            let script = formatdoc! {"
                mkdir -pv \"$VORPAL_OUTPUT\"
                pushd ./source/{name}
                chmod +x ./aws/install
                ./aws/install --install-dir \"$VORPAL_OUTPUT\" --bin-dir \"$VORPAL_OUTPUT/bin\"",
            };
            (path, script)
        }
        X8664Linux => {
            let path = format!(
                "https://awscli.amazonaws.com/awscli-exe-linux-x86_64-{source_version}.zip"
            );
            let script = formatdoc! {"
                mkdir -pv \"$VORPAL_OUTPUT\"
                pushd ./source/{name}
                chmod +x ./aws/install
                ./aws/install --install-dir \"$VORPAL_OUTPUT\" --bin-dir \"$VORPAL_OUTPUT/bin\"",
            };
            (path, script)
        }
        Aarch64Darwin | X8664Darwin => {
            let path = format!("https://awscli.amazonaws.com/AWSCLIV2-{source_version}.pkg");
            let script = formatdoc! {"
                mkdir -pv \"$VORPAL_OUTPUT/bin\"
                pushd ./source/{name}
                pkgutil --expand-full AWSCLIV2-{source_version}.pkg extracted
                cp -Rv extracted/aws-cli.pkg/Payload/aws-cli/* \"$VORPAL_OUTPUT/.\"

                # Verify extracted files exist before creating symlinks
                test -f \"$VORPAL_OUTPUT/aws\" || (echo 'ERROR: aws executable not found after extraction' && exit 1)
                test -f \"$VORPAL_OUTPUT/aws_completer\" || (echo 'ERROR: aws_completer not found after extraction' && exit 1)

                ln -sf \"$VORPAL_OUTPUT/aws\" \"$VORPAL_OUTPUT/bin/aws\"
                ln -sf \"$VORPAL_OUTPUT/aws_completer\" \"$VORPAL_OUTPUT/bin/aws_completer\"",
            };
            (path, script)
        }
        _ => return Err(anyhow::anyhow!("Unsupported system for awscli2 artifact")),
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
