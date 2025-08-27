use anyhow::Result;
use vorpal_artifacts::{devenv::DevEnvBuilder, DEFAULT_SYSTEMS};
use vorpal_sdk::context::get_context;

#[tokio::main]
async fn main() -> Result<()> {
    let context = &mut get_context().await?;

    DevEnvBuilder::new("dev", DEFAULT_SYSTEMS.to_vec())
        .build(context)
        .await?;

    context.run().await
}
