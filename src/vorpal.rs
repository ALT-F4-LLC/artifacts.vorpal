use anyhow::Result;
use vorpal_artifacts::{
    artifact::{
        bat, bottom, cue, direnv, doppler, fd, golangci_lint, just, lazygit, libevent, ncurses,
        nginx, openapi_generator_cli, openjdk, ripgrep, starship, terraform, tmux,
    },
    ProjectEnvironment, DEFAULT_SYSTEMS,
};
use vorpal_sdk::context::get_context;

#[tokio::main]
async fn main() -> Result<()> {
    let context = &mut get_context().await?;

    // Development Environment

    ProjectEnvironment::new("dev", DEFAULT_SYSTEMS.to_vec())
        .build(context)
        .await?;

    // Artifacts

    let openjdk = openjdk::build(context).await?;

    bat::build(context).await?;
    bottom::build(context).await?;
    cue::build(context).await?;
    direnv::build(context).await?;
    doppler::build(context).await?;
    fd::build(context).await?;
    golangci_lint::build(context).await?;
    just::build(context).await?;
    lazygit::build(context).await?;
    libevent::build(context).await?;
    ncurses::build(context).await?;
    nginx::build(context).await?;
    openapi_generator_cli::build(context, openjdk).await?;
    ripgrep::build(context).await?;
    starship::build(context).await?;
    terraform::build(context).await?;
    tmux::build(context).await?;

    context.run().await
}
