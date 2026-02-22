use crate::artifact::{cmake, json_c, libuv, libwebsockets, mbedtls, zlib};
use anyhow::Result;
use indoc::formatdoc;
use vorpal_sdk::{
    api::artifact::ArtifactSystem::{Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux},
    artifact::{get_env_key, step, Artifact, ArtifactSource},
    context::ConfigContext,
};

#[derive(Default)]
pub struct Ttyd<'a> {
    cmake: Option<&'a str>,
    json_c: Option<&'a str>,
    libuv: Option<&'a str>,
    libwebsockets: Option<&'a str>,
    mbedtls: Option<&'a str>,
    zlib: Option<&'a str>,
}

impl<'a> Ttyd<'a> {
    pub fn new() -> Self {
        Self {
            cmake: None,
            json_c: None,
            libuv: None,
            libwebsockets: None,
            mbedtls: None,
            zlib: None,
        }
    }

    pub fn with_cmake(mut self, cmake: &'a str) -> Self {
        self.cmake = Some(cmake);
        self
    }

    pub fn with_json_c(mut self, json_c: &'a str) -> Self {
        self.json_c = Some(json_c);
        self
    }

    pub fn with_libuv(mut self, libuv: &'a str) -> Self {
        self.libuv = Some(libuv);
        self
    }

    pub fn with_libwebsockets(mut self, libwebsockets: &'a str) -> Self {
        self.libwebsockets = Some(libwebsockets);
        self
    }

    pub fn with_mbedtls(mut self, mbedtls: &'a str) -> Self {
        self.mbedtls = Some(mbedtls);
        self
    }

    pub fn with_zlib(mut self, zlib: &'a str) -> Self {
        self.zlib = Some(zlib);
        self
    }

    pub async fn build(self, context: &mut ConfigContext) -> Result<String> {
        let cmake = match self.cmake {
            Some(val) => val,
            None => &cmake::Cmake::new().build(context).await?,
        };

        let json_c = match self.json_c {
            Some(val) => val,
            None => &json_c::JsonC::new().build(context).await?,
        };

        let libuv = match self.libuv {
            Some(val) => val,
            None => &libuv::Libuv::new().with_cmake(cmake).build(context).await?,
        };

        let libwebsockets = match self.libwebsockets {
            Some(val) => val,
            None => &libwebsockets::Libwebsockets::new().build(context).await?,
        };

        let mbedtls = match self.mbedtls {
            Some(val) => val,
            None => &mbedtls::Mbedtls::new().build(context).await?,
        };

        let name = "ttyd";
        let version = "1.7.7";

        let (sources, step_script, step_artifacts) = match context.get_system() {
            Aarch64Linux => {
                let path = format!(
                    "https://github.com/tsl0922/ttyd/releases/download/{version}/ttyd.aarch64"
                );
                let script = formatdoc! {"
                    mkdir -pv \"$VORPAL_OUTPUT/bin\"
                    cp ./source/{name}/ttyd.aarch64 \"$VORPAL_OUTPUT/bin/ttyd\"
                    chmod +x \"$VORPAL_OUTPUT/bin/ttyd\""
                };
                let sources = vec![ArtifactSource::new(name, &path).build()];
                (sources, script, vec![])
            }
            X8664Linux => {
                let path = format!(
                    "https://github.com/tsl0922/ttyd/releases/download/{version}/ttyd.x86_64"
                );
                let script = formatdoc! {"
                    mkdir -pv \"$VORPAL_OUTPUT/bin\"
                    cp ./source/{name}/ttyd.x86_64 \"$VORPAL_OUTPUT/bin/ttyd\"
                    chmod +x \"$VORPAL_OUTPUT/bin/ttyd\""
                };
                let sources = vec![ArtifactSource::new(name, &path).build()];
                (sources, script, vec![])
            }
            Aarch64Darwin | X8664Darwin => {
                let zlib = match self.zlib {
                    Some(val) => val.to_string(),
                    None => zlib::Zlib::new().build(context).await?,
                };

                let ttyd_path =
                    format!("https://github.com/tsl0922/ttyd/archive/refs/tags/{version}.tar.gz");

                let script = formatdoc! {"
                    mkdir -pv \"$VORPAL_OUTPUT/bin\"

                    BUILD_DIR=\"$(pwd)/build\"
                    mkdir -p \"$BUILD_DIR\"

                    pushd \"$BUILD_DIR\"

                    {cmake}/bin/cmake \
                        -DCMAKE_INSTALL_PREFIX=\"$VORPAL_OUTPUT\" \
                        -DCMAKE_PREFIX_PATH=\"{zlib};{json_c};{libuv};{mbedtls};{libwebsockets}\" \
                        -DCMAKE_BUILD_TYPE=RELEASE \
                        -DLIBUV_INCLUDE_DIR=\"{libuv}/include\" \
                        -DLIBUV_LIBRARY=\"{libuv}/lib/libuv.a\" \
                        \"$(pwd)/../source/{name}/{name}-{version}\"

                    make install
                    popd

                    chmod +x \"$VORPAL_OUTPUT/bin/ttyd\"",
                    cmake = get_env_key(&cmake.to_string()),
                    json_c = get_env_key(&json_c.to_string()),
                    libuv = get_env_key(&libuv.to_string()),
                    libwebsockets = get_env_key(&libwebsockets.to_string()),
                    mbedtls = get_env_key(&mbedtls.to_string()),
                    zlib = get_env_key(&zlib),
                };

                let sources = vec![ArtifactSource::new(name, &ttyd_path).build()];

                let artifacts = vec![
                    cmake.to_string(),
                    json_c.to_string(),
                    libuv.to_string(),
                    libwebsockets.to_string(),
                    mbedtls.to_string(),
                    zlib,
                ];

                (sources, script, artifacts)
            }
            _ => return Err(anyhow::anyhow!("Unsupported system for ttyd artifact")),
        };

        let steps = vec![step::shell(context, step_artifacts, vec![], step_script, vec![]).await?];

        let systems = vec![Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux];

        Artifact::new(name, steps, systems)
            .with_aliases(vec![format!("{name}:{version}")])
            .with_sources(sources)
            .build(context)
            .await
    }
}
