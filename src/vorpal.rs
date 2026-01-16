use anyhow::Result;
use vorpal_artifacts::{
    artifact::{
        argocd, awscli2, bat, bottom, crane, cue, direnv, doppler, fd, fluxcd, golangci_lint, gpg,
        helm, jq, just, k9s, kn, kubectl, kubeseal, lazygit, libassuan, libevent, libgcrypt,
        libgpg_error, libksba, ncurses, neovim, nginx, nnn, npth, openapi_generator_cli, openjdk,
        pkg_config, readline, ripgrep, skopeo, starship, terraform, tmux, umoci, yq, zsh,
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

    let libevent = libevent::build(context).await?;
    let libgpg_error = libgpg_error::build(context).await?;
    let libassuan = libassuan::build(context, &libgpg_error).await?;
    let libgcrypt = libgcrypt::build(context, &libgpg_error).await?;
    let libksba = libksba::build(context, &libgpg_error).await?;
    let ncurses = ncurses::build(context).await?;
    let npth = npth::build(context).await?;
    let openjdk = openjdk::build(context).await?;
    let pkg_config = pkg_config::build(context).await?;
    let readline = readline::build(context, &ncurses).await?;

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
    gpg::build(
        context,
        &libassuan,
        &libgcrypt,
        &libgpg_error,
        &libksba,
        &npth,
    )
    .await?;
    helm::build(context).await?;
    jq::build(context).await?;
    just::build(context).await?;
    k9s::build(context).await?;
    kn::build(context).await?;
    kubectl::build(context).await?;
    kubeseal::build(context).await?;
    lazygit::build(context).await?;
    neovim::build(context).await?;
    nginx::build(context).await?;
    nnn::build(context, &ncurses, &pkg_config, &readline).await?;
    openapi_generator_cli::build(context, openjdk).await?;
    ripgrep::build(context).await?;
    skopeo::build(context).await?;
    starship::build(context).await?;
    terraform::build(context).await?;
    tmux::build(context, &libevent, &ncurses).await?;
    umoci::build(context).await?;
    yq::build(context).await?;
    zsh::build(context, &ncurses).await?;

    context.run().await
}
