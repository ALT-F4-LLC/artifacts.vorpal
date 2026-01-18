use crate::artifact::{libassuan, libgcrypt, libgpg_error, libksba, npth};
use anyhow::Result;
use indoc::formatdoc;
use vorpal_sdk::{
    api::artifact::ArtifactSystem::{Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux},
    artifact::{get_env_key, step, Artifact, ArtifactSource},
    context::ConfigContext,
};

#[derive(Default)]
pub struct Gpg<'a> {
    libassuan: Option<&'a str>,
    libgcrypt: Option<&'a str>,
    libgpg_error: Option<&'a str>,
    libksba: Option<&'a str>,
    npth: Option<&'a str>,
}

impl<'a> Gpg<'a> {
    pub fn new() -> Self {
        Self {
            libassuan: None,
            libgcrypt: None,
            libgpg_error: None,
            libksba: None,
            npth: None,
        }
    }

    pub fn with_libassuan(mut self, libassuan: &'a str) -> Self {
        self.libassuan = Some(libassuan);
        self
    }

    pub fn with_libgcrypt(mut self, libgcrypt: &'a str) -> Self {
        self.libgcrypt = Some(libgcrypt);
        self
    }

    pub fn with_libgpg_error(mut self, libgpg_error: &'a str) -> Self {
        self.libgpg_error = Some(libgpg_error);
        self
    }

    pub fn with_libksba(mut self, libksba: &'a str) -> Self {
        self.libksba = Some(libksba);
        self
    }

    pub fn with_npth(mut self, npth: &'a str) -> Self {
        self.npth = Some(npth);
        self
    }

    pub async fn build(self, context: &mut ConfigContext) -> Result<String> {
        let libgpg_error = match self.libgpg_error {
            Some(val) => val,
            None => &libgpg_error::LibgpgError::new().build(context).await?,
        };

        let libassuan = match self.libassuan {
            Some(val) => val,
            None => {
                &libassuan::Libassuan::new()
                    .with_libgpg_error(libgpg_error)
                    .build(context)
                    .await?
            }
        };

        let libgcrypt = match self.libgcrypt {
            Some(val) => val,
            None => {
                &libgcrypt::Libgcrypt::new()
                    .with_libgpg_error(libgpg_error)
                    .build(context)
                    .await?
            }
        };

        let libksba = match self.libksba {
            Some(val) => val,
            None => {
                &libksba::Libksba::new()
                    .with_libgpg_error(libgpg_error)
                    .build(context)
                    .await?
            }
        };

        let npth = match self.npth {
            Some(val) => val,
            None => &npth::Npth::new().build(context).await?,
        };

        let name = "gpg";
        let version = "2.5.16";

        let path = format!("https://gnupg.org/ftp/gcrypt/gnupg/gnupg-{version}.tar.bz2");

        let source = ArtifactSource::new(name, &path).build();

        let script = formatdoc! {"
            mkdir -pv \"$VORPAL_OUTPUT\"

            pushd ./source/{name}/gnupg-{version}

            export PATH=\"{libgpg_error}/bin:{npth}/bin:{libgcrypt}/bin:{libassuan}/bin:{libksba}/bin:$PATH\"
            export PKG_CONFIG_PATH=\"{libgpg_error}/lib/pkgconfig:{npth}/lib/pkgconfig:{libgcrypt}/lib/pkgconfig:{libassuan}/lib/pkgconfig:{libksba}/lib/pkgconfig\"
            export CPPFLAGS=\"-I{libgpg_error}/include -I{npth}/include -I{libgcrypt}/include -I{libassuan}/include -I{libksba}/include\"
            export LDFLAGS=\"-L{libgpg_error}/lib -L{npth}/lib -L{libgcrypt}/lib -L{libassuan}/lib -L{libksba}/lib -Wl,-rpath,{libgpg_error}/lib -Wl,-rpath,{npth}/lib -Wl,-rpath,{libgcrypt}/lib -Wl,-rpath,{libassuan}/lib -Wl,-rpath,{libksba}/lib\"

            ./configure \
                --prefix=\"$VORPAL_OUTPUT\" \
                --with-libgpg-error-prefix={libgpg_error} \
                --with-npth-prefix={npth} \
                --with-libgcrypt-prefix={libgcrypt} \
                --with-libassuan-prefix={libassuan} \
                --with-ksba-prefix={libksba} \
                --disable-doc

            make
            make install",
            libassuan = get_env_key(&libassuan.to_string()),
            libgcrypt = get_env_key(&libgcrypt.to_string()),
            libgpg_error = get_env_key(&libgpg_error.to_string()),
            libksba = get_env_key(&libksba.to_string()),
            npth = get_env_key(&npth.to_string()),
        };

        let steps = vec![
            step::shell(
                context,
                vec![
                    libassuan.to_string(),
                    libgcrypt.to_string(),
                    libgpg_error.to_string(),
                    libksba.to_string(),
                    npth.to_string(),
                ],
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
