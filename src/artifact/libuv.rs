use crate::artifact::cmake;
use anyhow::Result;
use indoc::formatdoc;
use vorpal_sdk::{
    api::artifact::ArtifactSystem::{Aarch64Darwin, X8664Darwin},
    artifact::{get_env_key, step, Artifact, ArtifactSource},
    context::ConfigContext,
};

#[derive(Default)]
pub struct Libuv<'a> {
    cmake: Option<&'a str>,
}

impl<'a> Libuv<'a> {
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

        let name = "libuv";
        let version = "1.52.0";

        let path = format!("https://github.com/libuv/libuv/archive/refs/tags/v{version}.tar.gz");

        let source = ArtifactSource::new(name, &path).build();

        let script = formatdoc! {"
            mkdir -pv \"$VORPAL_OUTPUT\"

            BUILD_DIR=\"$(pwd)/build\"
            mkdir -p \"$BUILD_DIR\"

            pushd \"$BUILD_DIR\"
            {cmake}/bin/cmake \
                -DCMAKE_BUILD_TYPE=RELEASE \
                -DCMAKE_INSTALL_PREFIX=\"$VORPAL_OUTPUT\" \
                -DCMAKE_C_FLAGS=\"-fPIC\" \
                -DBUILD_TESTING=OFF \
                -DLIBUV_BUILD_SHARED=OFF \
                \"$(pwd)/../source/{name}/{name}-{version}\"
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
