use crate::artifact::libgpg_error;
use anyhow::Result;
use indoc::formatdoc;
use vorpal_sdk::{
    api::artifact::ArtifactSystem::{Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux},
    artifact::{get_env_key, step, Artifact, ArtifactSource},
    context::ConfigContext,
};

#[derive(Default)]
pub struct Libgcrypt<'a> {
    libgpg_error: Option<&'a str>,
}

impl<'a> Libgcrypt<'a> {
    pub fn new() -> Self {
        Self { libgpg_error: None }
    }

    pub fn with_libgpg_error(mut self, libgpg_error: &'a str) -> Self {
        self.libgpg_error = Some(libgpg_error);
        self
    }

    pub async fn build(self, context: &mut ConfigContext) -> Result<String> {
        let libgpg_error = match self.libgpg_error {
            Some(val) => val,
            None => &libgpg_error::LibgpgError::new().build(context).await?,
        };

        let name = "libgcrypt";
        let version = "1.11.0";

        let path = format!("https://gnupg.org/ftp/gcrypt/libgcrypt/libgcrypt-{version}.tar.bz2");

        let source = ArtifactSource::new(name, &path).build();

        let script = formatdoc! {"
            mkdir -pv \"$VORPAL_OUTPUT\"

            pushd ./source/{name}/libgcrypt-{version}

            export PATH=\"{libgpg_error}/bin:$PATH\"
            export CPPFLAGS=\"-I{libgpg_error}/include\"
            export LDFLAGS=\"-L{libgpg_error}/lib -Wl,-rpath,{libgpg_error}/lib\"

            ./configure --prefix=\"$VORPAL_OUTPUT\" --with-libgpg-error-prefix={libgpg_error} --disable-doc

            make
            make install",
            libgpg_error = get_env_key(&libgpg_error.to_string()),
        };

        let steps = vec![
            step::shell(
                context,
                vec![libgpg_error.to_string()],
                vec![],
                script,
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
