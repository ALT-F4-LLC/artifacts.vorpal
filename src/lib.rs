use vorpal_sdk::api::artifact::{
    ArtifactSystem,
    ArtifactSystem::{Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux},
};

pub mod devenv;

pub const DEFAULT_SYSTEMS: [ArtifactSystem; 4] =
    [Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux];
