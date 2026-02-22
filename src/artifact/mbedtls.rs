use crate::artifact::cmake;
use anyhow::Result;
use indoc::formatdoc;
use vorpal_sdk::{
    api::artifact::ArtifactSystem::{Aarch64Darwin, X8664Darwin},
    artifact::{get_env_key, step, Artifact, ArtifactSource},
    context::ConfigContext,
};

#[derive(Default)]
pub struct Mbedtls<'a> {
    cmake: Option<&'a str>,
}

impl<'a> Mbedtls<'a> {
    pub fn new() -> Self {
        Self { cmake: None }
    }

    pub fn with_cmake(mut self, cmake: &'a str) -> Self {
        self.cmake = Some(cmake);
        self
    }

    pub async fn build(self, context: &mut ConfigContext) -> Result<String> {
        let cmake = match self.cmake {
            Some(val) => val,
            None => &cmake::Cmake::new().build(context).await?,
        };

        let name = "mbedtls";
        let version = "3.6.5";

        let path = format!(
            "https://github.com/Mbed-TLS/mbedtls/releases/download/mbedtls-{version}/mbedtls-{version}.tar.bz2"
        );

        let source = ArtifactSource::new(name, &path).build();

        let script = formatdoc! {"
            mkdir -pv \"$VORPAL_OUTPUT\"

            BUILD_DIR=\"$(pwd)/build\"
            mkdir -p \"$BUILD_DIR\"

            pushd \"$BUILD_DIR\"

            {cmake}/bin/cmake \
                -DCMAKE_BUILD_TYPE=RELEASE \
                -DCMAKE_INSTALL_PREFIX=\"$VORPAL_OUTPUT\" \
                -DENABLE_TESTING=OFF \
                -DUSE_SHARED_MBEDTLS_LIBRARY=OFF \
                \"$(pwd)/../source/{name}/mbedtls-{version}\"

            make -j$(sysctl -n hw.ncpu) install
            popd",
            cmake = get_env_key(&cmake.to_string()),
        };

        let steps =
            vec![step::shell(context, vec![cmake.to_string()], vec![], script, vec![]).await?];

        let systems = vec![Aarch64Darwin, X8664Darwin];

        Artifact::new(name, steps, systems)
            .with_aliases(vec![format!("{name}:{version}")])
            .with_sources(vec![source])
            .build(context)
            .await
    }
}
