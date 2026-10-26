use anyhow::Result;
use vorpal_sdk::{
    api::artifact::ArtifactSystem::{Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux},
    artifact::{language::go::Go, ArtifactSource},
    context::ConfigContext,
};

#[derive(Default)]
pub struct Govulncheck;

impl Govulncheck {
    pub fn new() -> Self {
        Self
    }

    pub async fn build(self, context: &mut ConfigContext) -> Result<String> {
        let name = "govulncheck";
        let version = "1.9.0";

        let source_path =
            format!("https://github.com/golang/vuln/archive/refs/tags/v{version}.tar.gz");
        let source = ArtifactSource::new(name, source_path.as_str()).build();

        let build_directory = format!("./vuln-{version}");
        let build_path = format!("./cmd/{name}");

        let systems = vec![Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux];

        Go::new(name, systems)
            .with_alias(format!("{name}:{version}"))
            .with_build_directory(build_directory.as_str())
            .with_build_path(build_path.as_str())
            .with_source(source)
            .build(context)
            .await
    }
}
