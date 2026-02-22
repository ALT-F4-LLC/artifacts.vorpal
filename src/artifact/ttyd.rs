use anyhow::Result;
use indoc::formatdoc;
use vorpal_sdk::{
    api::artifact::ArtifactSystem::{Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux},
    artifact::{step, Artifact, ArtifactSource},
    context::ConfigContext,
};

#[derive(Default)]
pub struct Ttyd;

impl Ttyd {
    pub fn new() -> Self {
        Self
    }

    pub async fn build(self, context: &mut ConfigContext) -> Result<String> {
        let name = "ttyd";
        let source_version = "1.7.7";

        let cmake_version = "3.31.6";
        let zlib_version = "1.3.1";
        let json_c_version = "0.17";
        let libuv_version = "1.44.2";
        let mbedtls_version = "2.28.5";
        let lws_version = "4.3.3";

        let (sources, step_script) = match context.get_system() {
            Aarch64Linux => {
                let path = format!(
                    "https://github.com/tsl0922/ttyd/releases/download/{source_version}/ttyd.aarch64"
                );
                let script = formatdoc! {"
                    mkdir -pv \"$VORPAL_OUTPUT/bin\"
                    cp ./source/{name}/ttyd.aarch64 \"$VORPAL_OUTPUT/bin/ttyd\"
                    chmod +x \"$VORPAL_OUTPUT/bin/ttyd\""
                };
                let sources = vec![ArtifactSource::new(name, &path).build()];
                (sources, script)
            }
            X8664Linux => {
                let path = format!(
                    "https://github.com/tsl0922/ttyd/releases/download/{source_version}/ttyd.x86_64"
                );
                let script = formatdoc! {"
                    mkdir -pv \"$VORPAL_OUTPUT/bin\"
                    cp ./source/{name}/ttyd.x86_64 \"$VORPAL_OUTPUT/bin/ttyd\"
                    chmod +x \"$VORPAL_OUTPUT/bin/ttyd\""
                };
                let sources = vec![ArtifactSource::new(name, &path).build()];
                (sources, script)
            }
            Aarch64Darwin | X8664Darwin => {
                let ttyd_path = format!(
                    "https://github.com/tsl0922/ttyd/archive/refs/tags/{source_version}.tar.gz"
                );
                let cmake_path = format!(
                    "https://github.com/Kitware/CMake/releases/download/v{cmake_version}/cmake-{cmake_version}-macos-universal.tar.gz"
                );
                let zlib_path = format!(
                    "https://github.com/madler/zlib/releases/download/v{zlib_version}/zlib-{zlib_version}.tar.gz"
                );
                let json_c_path = format!(
                    "https://s3.amazonaws.com/json-c_releases/releases/json-c-{json_c_version}.tar.gz"
                );
                let libuv_path = format!(
                    "https://github.com/libuv/libuv/archive/refs/tags/v{libuv_version}.tar.gz"
                );
                let mbedtls_path = format!(
                    "https://github.com/Mbed-TLS/mbedtls/archive/refs/tags/v{mbedtls_version}.tar.gz"
                );
                let lws_path = format!(
                    "https://github.com/warmcat/libwebsockets/archive/refs/tags/v{lws_version}.tar.gz"
                );

                let script = formatdoc! {"
                    mkdir -pv \"$VORPAL_OUTPUT/bin\"

                    STAGE_DIR=\"$(pwd)/stage\"
                    BUILD_DIR=\"$(pwd)/build\"
                    mkdir -p \"$STAGE_DIR\" \"$BUILD_DIR\"
                    export PKG_CONFIG_PATH=\"$STAGE_DIR/lib/pkgconfig\"

                    CMAKE=\"$(pwd)/source/ttyd-cmake/cmake-{cmake_version}-macos-universal/CMake.app/Contents/bin/cmake\"

                    echo \"=== Building zlib ===\"
                    pushd ./source/ttyd-zlib/zlib-{zlib_version}
                    ./configure --static --prefix=\"$STAGE_DIR\"
                    make -j$(sysctl -n hw.ncpu) install
                    popd

                    echo \"=== Building json-c ===\"
                    mkdir -p \"$BUILD_DIR/json-c\" && pushd \"$BUILD_DIR/json-c\"
                    \"$CMAKE\" \
                        -DCMAKE_BUILD_TYPE=RELEASE \
                        -DCMAKE_INSTALL_PREFIX=\"$STAGE_DIR\" \
                        -DBUILD_SHARED_LIBS=OFF \
                        -DBUILD_TESTING=OFF \
                        -DDISABLE_THREAD_LOCAL_STORAGE=ON \
                        \"$(pwd)/../../source/ttyd-json-c/json-c-{json_c_version}\"
                    make -j$(sysctl -n hw.ncpu) install
                    popd

                    echo \"=== Building libuv ===\"
                    mkdir -p \"$BUILD_DIR/libuv\" && pushd \"$BUILD_DIR/libuv\"
                    \"$CMAKE\" \
                        -DCMAKE_BUILD_TYPE=RELEASE \
                        -DCMAKE_INSTALL_PREFIX=\"$STAGE_DIR\" \
                        -DCMAKE_C_FLAGS=\"-fPIC\" \
                        -DBUILD_TESTING=OFF \
                        \"$(pwd)/../../source/ttyd-libuv/libuv-{libuv_version}\"
                    make -j$(sysctl -n hw.ncpu) install
                    popd

                    # Remove shared libraries and create static lib symlink for libuv
                    find \"$STAGE_DIR/lib\" -name '*.dylib' -delete 2>/dev/null || true
                    ln -sf \"$STAGE_DIR/lib/libuv_a.a\" \"$STAGE_DIR/lib/libuv.a\"

                    echo \"=== Building mbedtls ===\"
                    mkdir -p \"$BUILD_DIR/mbedtls\" && pushd \"$BUILD_DIR/mbedtls\"
                    \"$CMAKE\" \
                        -DCMAKE_BUILD_TYPE=RELEASE \
                        -DCMAKE_INSTALL_PREFIX=\"$STAGE_DIR\" \
                        -DENABLE_TESTING=OFF \
                        -DUSE_SHARED_MBEDTLS_LIBRARY=OFF \
                        \"$(pwd)/../../source/ttyd-mbedtls/mbedtls-{mbedtls_version}\"
                    make -j$(sysctl -n hw.ncpu) install
                    popd

                    echo \"=== Building libwebsockets ===\"
                    LWS_SRC=\"$(pwd)/source/ttyd-lws/libwebsockets-{lws_version}\"
                    sed 's/ websockets_shared//g' \"$LWS_SRC/cmake/libwebsockets-config.cmake.in\" > \"$LWS_SRC/cmake/libwebsockets-config.cmake.in.tmp\"
                    mv \"$LWS_SRC/cmake/libwebsockets-config.cmake.in.tmp\" \"$LWS_SRC/cmake/libwebsockets-config.cmake.in\"
                    mkdir -p \"$BUILD_DIR/lws\" && pushd \"$BUILD_DIR/lws\"
                    \"$CMAKE\" \
                        -DCMAKE_BUILD_TYPE=RELEASE \
                        -DCMAKE_INSTALL_PREFIX=\"$STAGE_DIR\" \
                        -DCMAKE_FIND_LIBRARY_SUFFIXES=\".a\" \
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
                        \"$LWS_SRC\"
                    make -j$(sysctl -n hw.ncpu) install
                    popd

                    echo \"=== Building ttyd ===\"
                    mkdir -p \"$BUILD_DIR/ttyd\" && pushd \"$BUILD_DIR/ttyd\"
                    \"$CMAKE\" \
                        -DCMAKE_INSTALL_PREFIX=\"$VORPAL_OUTPUT\" \
                        -DCMAKE_PREFIX_PATH=\"$STAGE_DIR\" \
                        -DCMAKE_BUILD_TYPE=RELEASE \
                        \"$(pwd)/../../source/{name}/ttyd-{source_version}\"
                    make install
                    popd

                    chmod +x \"$VORPAL_OUTPUT/bin/ttyd\""
                };

                let sources = vec![
                    ArtifactSource::new(name, &ttyd_path).build(),
                    ArtifactSource::new("ttyd-cmake", &cmake_path).build(),
                    ArtifactSource::new("ttyd-zlib", &zlib_path).build(),
                    ArtifactSource::new("ttyd-json-c", &json_c_path).build(),
                    ArtifactSource::new("ttyd-libuv", &libuv_path).build(),
                    ArtifactSource::new("ttyd-mbedtls", &mbedtls_path).build(),
                    ArtifactSource::new("ttyd-lws", &lws_path).build(),
                ];
                (sources, script)
            }
            _ => return Err(anyhow::anyhow!("Unsupported system for ttyd artifact")),
        };

        let steps = vec![step::shell(context, vec![], vec![], step_script, vec![]).await?];

        let systems = vec![Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux];

        Artifact::new(name, steps, systems)
            .with_aliases(vec![format!("{name}:{source_version}")])
            .with_sources(sources)
            .build(context)
            .await
    }
}
