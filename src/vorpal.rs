use anyhow::Result;
use vorpal_artifacts::{
    artifact::{
        argocd, awscli2, bat, bottom, crane, cue, direnv, doppler, fd, fluxcd, golangci_lint, gpg,
        helm, jq, just, k9s, kn, kubectl, kubeseal, lazygit, libevent, ncurses, neovim, nginx, nnn,
        openapi_generator_cli, openjdk, pkg_config, ripgrep, skopeo, starship, terraform, tmux,
        umoci, yq, zsh,
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

    argocd::build(context).await?;
    awscli2::build(context).await?;
    bat::build(context).await?;
    bottom::build(context).await?;
    crane::build(context).await?;
    cue::build(context).await?;
    direnv::build(context).await?;
    doppler::build(context).await?;
    fd::build(context).await?;
    fluxcd::build(context).await?;
    golangci_lint::build(context).await?;
    gpg::build(context).await?;
    helm::build(context).await?;
    jq::build(context).await?;
    just::build(context).await?;
    k9s::build(context).await?;
    kn::build(context).await?;
    kubectl::build(context).await?;
    kubeseal::build(context).await?;
    lazygit::build(context).await?;
    libevent::build(context).await?;
    ncurses::build(context).await?;
    neovim::build(context).await?;
    nginx::build(context).await?;
    nnn::build(context).await?;
    openapi_generator_cli::build(context, openjdk).await?;
    pkg_config::build(context).await?;
    ripgrep::build(context).await?;
    skopeo::build(context).await?;
    starship::build(context).await?;
    terraform::build(context).await?;
    tmux::build(context).await?;
    umoci::build(context).await?;
    yq::build(context).await?;
    zsh::build(context).await?;

    context.run().await
}
