use crate::artifact::{pkg_config::PkgConfig, x264};
use anyhow::Result;
use indoc::formatdoc;
use vorpal_sdk::{
    api::artifact::ArtifactSystem::{Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux},
    artifact::{get_env_key, step, Artifact, ArtifactSource},
    context::ConfigContext,
};

#[derive(Default)]
pub struct Ffmpeg<'a> {
    x264: Option<&'a str>,
}

impl<'a> Ffmpeg<'a> {
    pub fn new() -> Self {
        Self { x264: None }
    }

    pub fn with_x264(mut self, x264: &'a str) -> Self {
        self.x264 = Some(x264);
        self
    }

    pub async fn build(self, context: &mut ConfigContext) -> Result<String> {
        let pkg_config = PkgConfig::new().build(context).await?;
        let x264 = match self.x264 {
            Some(val) => val,
            None => &x264::X264::new().build(context).await?,
        };

        let name = "ffmpeg";

        let version = "8.0.1";

        let source_path = format!("https://ffmpeg.org/releases/ffmpeg-{version}.tar.xz");
        let source = ArtifactSource::new(name, &source_path).build();

        let step_script = formatdoc! {"
            mkdir -pv \"$VORPAL_OUTPUT\"

            pushd ./source/{name}/ffmpeg-{version}

            export PATH=\"{pkg_config}/bin:${{PATH:-}}\"
            export CPPFLAGS=\"-I{x264}/include\"
            export LDFLAGS=\"-L{x264}/lib -Wl,-rpath,{x264}/lib\"
            export PKG_CONFIG_PATH=\"{x264}/lib/pkgconfig:${{PKG_CONFIG_PATH:-}}\"

            ./configure \
                --prefix=\"$VORPAL_OUTPUT\" \
                --disable-doc \
                --disable-debug \
                --disable-x86asm \
                --enable-gpl \
                --enable-libx264

            make -j$(nproc 2>/dev/null || sysctl -n hw.ncpu)
            make install",
            pkg_config = get_env_key(&pkg_config.to_string()),
            x264 = get_env_key(&x264.to_string()),
        };

        let steps = vec![
            step::shell(
                context,
                vec![x264.to_string(), pkg_config.to_string()],
                vec![],
                step_script,
                vec![],
            )
            .await?,
        ];

        let systems = vec![Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux];

        Artifact::new(name, steps, systems)
            .with_aliases(vec![format!("{name}:{version}")])
            .with_sources(vec![source])
            .build(context)
            .await
    }
}
