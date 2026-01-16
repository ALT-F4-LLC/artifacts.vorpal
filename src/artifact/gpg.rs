use anyhow::Result;
use indoc::formatdoc;
use vorpal_sdk::{
    api::artifact::ArtifactSystem::{Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux},
    artifact::{get_env_key, step, Artifact, ArtifactSource},
    context::ConfigContext,
};

pub async fn build(
    context: &mut ConfigContext,
    libassuan: &String,
    libgcrypt: &String,
    libgpg_error: &String,
    libksba: &String,
    npth: &String,
) -> Result<String> {
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
        libassuan = get_env_key(libassuan),
        libgcrypt = get_env_key(libgcrypt),
        libgpg_error = get_env_key(libgpg_error),
        libksba = get_env_key(libksba),
        npth = get_env_key(npth),
    };

    let steps = vec![
        step::shell(
            context,
            vec![
                libassuan.clone(),
                libgcrypt.clone(),
                libgpg_error.clone(),
                libksba.clone(),
                npth.clone(),
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
