use crate::artifact::{cmake, libuv, mbedtls, zlib};
use anyhow::Result;
use indoc::formatdoc;
use vorpal_sdk::{
    api::artifact::ArtifactSystem::{Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux},
    artifact::{get_env_key, step, Artifact, ArtifactSource},
    context::ConfigContext,
};

#[derive(Default)]
pub struct Libwebsockets<'a> {
    cmake: Option<&'a str>,
    zlib: Option<&'a str>,
    libuv: Option<&'a str>,
    mbedtls: Option<&'a str>,
}

impl<'a> Libwebsockets<'a> {
    pub fn new() -> Self {
        Self {
            cmake: None,
            zlib: None,
            libuv: None,
            mbedtls: None,
        }
    }

    pub fn with_cmake(mut self, cmake: &'a str) -> Self {
        self.cmake = Some(cmake);
        self
    }

    pub fn with_zlib(mut self, zlib: &'a str) -> Self {
        self.zlib = Some(zlib);
        self
    }

    pub fn with_libuv(mut self, libuv: &'a str) -> Self {
        self.libuv = Some(libuv);
        self
    }

    pub fn with_mbedtls(mut self, mbedtls: &'a str) -> Self {
        self.mbedtls = Some(mbedtls);
        self
    }

    pub async fn build(self, context: &mut ConfigContext) -> Result<String> {
        let cmake = match self.cmake {
            Some(val) => val,
            None => &cmake::Cmake::new().build(context).await?,
        };

        let zlib = match self.zlib {
            Some(val) => val,
            None => &zlib::Zlib::new().build(context).await?,
        };

        let libuv = match self.libuv {
            Some(val) => val,
            None => &libuv::Libuv::new().build(context).await?,
        };

        let mbedtls = match self.mbedtls {
            Some(val) => val,
            None => &mbedtls::Mbedtls::new().build(context).await?,
        };

        let name = "libwebsockets";
        let version = "4.5.2";

        let path =
            format!("https://github.com/warmcat/libwebsockets/archive/refs/tags/v{version}.tar.gz");

        let source = ArtifactSource::new(name, &path).build();

        let script = formatdoc! {"
            mkdir -pv \"$VORPAL_OUTPUT\"

            LWS_SRC=\"$(pwd)/source/{name}/{name}-{version}\"

            sed 's/ websockets_shared//g' \"$LWS_SRC/cmake/libwebsockets-config.cmake.in\" > \"$LWS_SRC/cmake/libwebsockets-config.cmake.in.tmp\"
            mv \"$LWS_SRC/cmake/libwebsockets-config.cmake.in.tmp\" \"$LWS_SRC/cmake/libwebsockets-config.cmake.in\"

            BUILD_DIR=\"$(pwd)/build\"
            mkdir -p \"$BUILD_DIR\"

            pushd \"$BUILD_DIR\"

            {cmake}/bin/cmake \
                -DCMAKE_BUILD_TYPE=RELEASE \
                -DCMAKE_INSTALL_PREFIX=\"$VORPAL_OUTPUT\" \
                -DCMAKE_FIND_LIBRARY_SUFFIXES=\".a\" \
                -DCMAKE_PREFIX_PATH=\"{zlib};{libuv};{mbedtls}\" \
                -DLWS_WITHOUT_TESTAPPS=ON \
                -DLWS_WITH_MBEDTLS=ON \
                -DLWS_WITH_LIBUV=ON \
                -DLWS_STATIC_PIC=ON \
                -DLWS_WITH_SHARED=OFF \
                -DLWS_UNIX_SOCK=ON \
                -DLWS_IPV6=ON \
                -DLWS_ROLE_RAW_FILE=OFF \
                -DLWS_WITH_HTTP2=ON \
                -DLWS_WITH_HTTP_BASIC_AUTH=OFF \
                -DLWS_WITH_UDP=OFF \
                -DLWS_WITHOUT_CLIENT=ON \
                -DLWS_WITHOUT_EXTENSIONS=OFF \
                -DLWS_WITH_LEJP=OFF \
                -DLWS_WITH_LEJP_CONF=OFF \
                -DLWS_WITH_LWSAC=OFF \
                -DLWS_WITH_SEQUENCER=OFF \
                -DLWS_WITH_SYS_FAULT_INJECTION=OFF \
                -DLWS_WITH_SYS_METRICS=OFF \
                -DLWS_WITH_DLO=OFF \
                \"$LWS_SRC\"

            make -j$(nproc 2>/dev/null || sysctl -n hw.ncpu) install
            popd",
            cmake = get_env_key(&cmake.to_string()),
            zlib = get_env_key(&zlib.to_string()),
            libuv = get_env_key(&libuv.to_string()),
            mbedtls = get_env_key(&mbedtls.to_string()),
        };

        let steps = vec![
            step::shell(
                context,
                vec![
                    cmake.to_string(),
                    zlib.to_string(),
                    libuv.to_string(),
                    mbedtls.to_string(),
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
