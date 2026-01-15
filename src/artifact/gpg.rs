use crate::artifact::{libassuan, libgcrypt, libgpg_error, libksba, npth};
use anyhow::Result;
use indoc::formatdoc;
use vorpal_sdk::{
    api::artifact::ArtifactSystem::{Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux},
    artifact::{get_env_key, step, Artifact, ArtifactSource},
    context::ConfigContext,
};

pub async fn build(context: &mut ConfigContext) -> Result<String> {
    let libgpg_error = libgpg_error::build(context).await?;
    let npth = npth::build(context).await?;
    let libgcrypt = libgcrypt::build(context).await?;
    let libassuan = libassuan::build(context).await?;
    let libksba = libksba::build(context).await?;

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
        libgpg_error = get_env_key(&libgpg_error),
        npth = get_env_key(&npth),
        libgcrypt = get_env_key(&libgcrypt),
        libassuan = get_env_key(&libassuan),
        libksba = get_env_key(&libksba),
    };

    let steps = vec![
        step::shell(
            context,
            vec![libgpg_error, npth, libgcrypt, libassuan, libksba],
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
