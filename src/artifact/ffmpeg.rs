use anyhow::Result;
use indoc::formatdoc;
use vorpal_sdk::{
    api::artifact::ArtifactSystem::{Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux},
    artifact::{step, Artifact, ArtifactSource},
    context::ConfigContext,
};

#[derive(Default)]
pub struct Ffmpeg;

impl Ffmpeg {
    pub fn new() -> Self {
        Self
    }

    pub async fn build(self, context: &mut ConfigContext) -> Result<String> {
        let name = "ffmpeg";
        let source_version = "7.1.3";

        let source_path =
            format!("https://ffmpeg.org/releases/ffmpeg-{source_version}.tar.xz");

        let source = ArtifactSource::new(name, &source_path).build();

        let step_script = formatdoc! {"
            mkdir -pv \"$VORPAL_OUTPUT\"

            pushd ./source/{name}/ffmpeg-{source_version}

            ./configure \
                --prefix=\"$VORPAL_OUTPUT\" \
                --disable-doc \
                --disable-debug \
                --enable-gpl

            make -j$(nproc 2>/dev/null || sysctl -n hw.ncpu)
            make install",
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
