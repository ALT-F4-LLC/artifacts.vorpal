use anyhow::Result;
use indoc::formatdoc;
use vorpal_sdk::{
    api::artifact::ArtifactSystem::{Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux},
    artifact::{step, Artifact, ArtifactSource},
    context::ConfigContext,
};

#[derive(Default)]
pub struct X264;

impl X264 {
    pub fn new() -> Self {
        Self
    }

    pub async fn build(self, context: &mut ConfigContext) -> Result<String> {
        let name = "x264";
        let version = "20191217-2245-stable";

        let source_path = format!(
            "https://download.videolan.org/pub/videolan/x264/snapshots/x264-snapshot-{version}.tar.bz2"
        );
        let source = ArtifactSource::new(name, &source_path).build();

        // x264's 2019 config.guess misdetects Apple Silicon.
        let (host, asm_flag) = match context.get_system() {
            Aarch64Darwin => ("aarch64-apple-darwin", ""),
            Aarch64Linux => ("aarch64-unknown-linux-gnu", ""),
            X8664Darwin => ("x86_64-apple-darwin", " --disable-asm"),
            X8664Linux => ("x86_64-unknown-linux-gnu", " --disable-asm"),
            _ => return Err(anyhow::anyhow!("Unsupported system for {name} artifact")),
        };

        let step_script = formatdoc! {"
            mkdir -pv \"$VORPAL_OUTPUT\"

            pushd ./source/{name}/{name}-snapshot-{version}

            ./configure --prefix=\"$VORPAL_OUTPUT\" --enable-static --enable-pic --disable-cli{asm_flag} --host={host}

            make -j$(nproc 2>/dev/null || sysctl -n hw.ncpu)
            make install",
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
