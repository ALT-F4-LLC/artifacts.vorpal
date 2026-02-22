use crate::artifact::cmake;
use anyhow::Result;
use indoc::formatdoc;
use vorpal_sdk::{
    api::artifact::ArtifactSystem::{Aarch64Darwin, X8664Darwin},
    artifact::{get_env_key, step, Artifact, ArtifactSource},
    context::ConfigContext,
};

#[derive(Default)]
pub struct JsonC<'a> {
    cmake: Option<&'a str>,
}

impl<'a> JsonC<'a> {
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

        let name = "json-c";
        let version = "0.18";
        let tag = "json-c-0.18-20240915";

        let path = format!("https://github.com/json-c/json-c/archive/refs/tags/{tag}.tar.gz");

        let source = ArtifactSource::new(name, &path).build();

        let script = formatdoc! {"
            mkdir -pv \"$VORPAL_OUTPUT\"

            BUILD_DIR=\"$(pwd)/build\"
            mkdir -p \"$BUILD_DIR\"

            pushd \"$BUILD_DIR\"

            {cmake}/bin/cmake \
                -DCMAKE_BUILD_TYPE=RELEASE \
                -DCMAKE_INSTALL_PREFIX=\"$VORPAL_OUTPUT\" \
                -DCMAKE_POLICY_VERSION_MINIMUM=3.5 \
                -DBUILD_SHARED_LIBS=OFF \
                -DBUILD_TESTING=OFF \
                -DDISABLE_THREAD_LOCAL_STORAGE=ON \
                \"$(pwd)/../source/{name}/json-c-{tag}\"

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
