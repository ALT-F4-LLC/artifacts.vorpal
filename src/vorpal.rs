use anyhow::Result;
use vorpal_artifacts::{
    artifact::{
        bat, bottom, direnv, doppler, fd, lazygit, libevent, ncurses, nginx, ripgrep, starship,
        terraform, tmux,
    },
    DevEnvBuilder, DEFAULT_SYSTEMS,
};
use vorpal_sdk::context::get_context;

#[tokio::main]
async fn main() -> Result<()> {
    let context = &mut get_context().await?;

    // Development Environment

    DevEnvBuilder::new("dev", DEFAULT_SYSTEMS.to_vec())
        .build(context)
        .await?;

    // Artifacts

    bat::build(context).await?;
    bottom::build(context).await?;
    direnv::build(context).await?;
    doppler::build(context).await?;
    fd::build(context).await?;
    lazygit::build(context).await?;
    libevent::build(context).await?;
    ncurses::build(context).await?;
    nginx::build(context).await?;
    ripgrep::build(context).await?;
    starship::build(context).await?;
    terraform::build(context).await?;
    tmux::build(context).await?;

    context.run().await
}
