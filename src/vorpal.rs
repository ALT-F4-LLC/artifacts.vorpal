use anyhow::Result;
use vorpal_artifacts::{
    artifact::{
        argocd::Argocd, awscli2::Awscli2, bat::Bat, beads::Beads, bottom::Bottom, crane::Crane,
        cue::Cue, direnv::Direnv, doppler::Doppler, fd::Fd, fluxcd::Fluxcd,
        golangci_lint::GolangciLint, gpg::Gpg, helm::Helm, jj::Jj, jq::Jq, just::Just, k9s::K9s,
        kn::Kn, kubectl::Kubectl, kubeseal::Kubeseal, lazygit::Lazygit, libassuan::Libassuan,
        libevent::Libevent, libgcrypt::Libgcrypt, libgpg_error::LibgpgError, libksba::Libksba,
        lima::Lima, ncurses::Ncurses, neovim::Neovim, nginx::Nginx, nnn::Nnn, npth::Npth,
        openapi_generator_cli::OpenapiGeneratorCli, openjdk::Openjdk, pkg_config::PkgConfig,
        readline::Readline, ripgrep::Ripgrep, skopeo::Skopeo, sqlite3::Sqlite3, starship::Starship,
        terraform::Terraform, tmux::Tmux, umoci::Umoci, vhs::Vhs, yq::Yq, zsh::Zsh,
    },
    ProjectEnvironment, DEFAULT_SYSTEMS,
};
use vorpal_sdk::context::get_context;

#[tokio::main]
async fn main() -> Result<()> {
    let context = &mut get_context().await?;

    // Artifacts

    let libevent = Libevent::new().build(context).await?;

    let libgpg_error = LibgpgError::new().build(context).await?;

    let libassuan = Libassuan::new()
        .with_libgpg_error(&libgpg_error)
        .build(context)
        .await?;

    let libgcrypt = Libgcrypt::new()
        .with_libgpg_error(&libgpg_error)
        .build(context)
        .await?;

    let libksba = Libksba::new()
        .with_libgpg_error(&libgpg_error)
        .build(context)
        .await?;

    let ncurses = Ncurses::new().build(context).await?;

    let npth = Npth::new().build(context).await?;

    let openjdk = Openjdk::new().build(context).await?;

    let pkg_config = PkgConfig::new().build(context).await?;

    let readline = Readline::new()
        .with_ncurses(&ncurses)
        .build(context)
        .await?;

    Argocd::new().build(context).await?;

    Awscli2::new().build(context).await?;

    Bat::new().build(context).await?;

    Beads::new().build(context).await?;

    Bottom::new().build(context).await?;

    Crane::new().build(context).await?;

    Cue::new().build(context).await?;

    Direnv::new().build(context).await?;

    Doppler::new().build(context).await?;

    Fd::new().build(context).await?;

    Fluxcd::new().build(context).await?;

    GolangciLint::new().build(context).await?;

    Gpg::new()
        .with_libassuan(&libassuan)
        .with_libgcrypt(&libgcrypt)
        .with_libgpg_error(&libgpg_error)
        .with_libksba(&libksba)
        .with_npth(&npth)
        .build(context)
        .await?;

    Helm::new().build(context).await?;

    Jj::new().build(context).await?;

    Jq::new().build(context).await?;

    Just::new().build(context).await?;

    K9s::new().build(context).await?;

    Kn::new().build(context).await?;

    Kubectl::new().build(context).await?;

    Kubeseal::new().build(context).await?;

    Lazygit::new().build(context).await?;

    Lima::new().build(context).await?;

    Neovim::new().build(context).await?;

    Nginx::new().build(context).await?;

    Nnn::new()
        .with_ncurses(&ncurses)
        .with_pkg_config(&pkg_config)
        .with_readline(&readline)
        .build(context)
        .await?;

    OpenapiGeneratorCli::new()
        .with_openjdk(&openjdk)
        .build(context)
        .await?;

    Ripgrep::new().build(context).await?;

    Skopeo::new().build(context).await?;

    Sqlite3::new().build(context).await?;

    Starship::new().build(context).await?;

    Terraform::new().build(context).await?;

    Tmux::new()
        .with_libevent(&libevent)
        .with_ncurses(&ncurses)
        .build(context)
        .await?;

    Umoci::new().build(context).await?;

    Vhs::new().build(context).await?;

    Yq::new().build(context).await?;

    Zsh::new().with_ncurses(&ncurses).build(context).await?;

    // Development Environment

    ProjectEnvironment::new("dev", DEFAULT_SYSTEMS.to_vec())
        .build(context)
        .await?;

    context.run().await
}
