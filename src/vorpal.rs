use anyhow::Result;
use vorpal_artifacts::{
    artifact::{
        argocd::Argocd, awscli2::Awscli2, bat::Bat, beads::Beads, bottom::Bottom, cmake::Cmake,
        crane::Crane, cue::Cue, direnv::Direnv, doppler::Doppler, fd::Fd, ffmpeg::Ffmpeg,
        fluxcd::Fluxcd, golangci_lint::GolangciLint, gpg::Gpg, helm::Helm, jj::Jj, jq::Jq,
        json_c::JsonC, just::Just, k9s::K9s, kn::Kn, kubectl::Kubectl, kubeseal::Kubeseal,
        lazygit::Lazygit, libassuan::Libassuan, libevent::Libevent, libgcrypt::Libgcrypt,
        libgpg_error::LibgpgError, libksba::Libksba, libuv::Libuv, libwebsockets::Libwebsockets,
        lima::Lima, mbedtls::Mbedtls, ncurses::Ncurses, neovim::Neovim, nginx::Nginx, nnn::Nnn,
        npth::Npth, openapi_generator_cli::OpenapiGeneratorCli, openjdk::Openjdk,
        pkg_config::PkgConfig, readline::Readline, ripgrep::Ripgrep, skopeo::Skopeo,
        sqlite3::Sqlite3, starship::Starship, terraform::Terraform, tmux::Tmux, ttyd::Ttyd,
        umoci::Umoci, vhs::Vhs, yq::Yq, zsh::Zsh,
    },
    ProjectEnvironment, DEFAULT_SYSTEMS,
};
use vorpal_sdk::context::get_context;

#[tokio::main]
async fn main() -> Result<()> {
    let context = &mut get_context().await?;

    // Artifacts

    Argocd::new().build(context).await?;
    Awscli2::new().build(context).await?;
    Bat::new().build(context).await?;
    Beads::new().build(context).await?;
    Bottom::new().build(context).await?;
    Cmake::new().build(context).await?;
    Crane::new().build(context).await?;
    Cue::new().build(context).await?;
    Direnv::new().build(context).await?;
    Doppler::new().build(context).await?;
    Fd::new().build(context).await?;
    Ffmpeg::new().build(context).await?;
    Fluxcd::new().build(context).await?;
    GolangciLint::new().build(context).await?;
    Gpg::new().build(context).await?;
    Helm::new().build(context).await?;
    Jj::new().build(context).await?;
    Jq::new().build(context).await?;
    JsonC::new().build(context).await?;
    Just::new().build(context).await?;
    K9s::new().build(context).await?;
    Kn::new().build(context).await?;
    Kubectl::new().build(context).await?;
    Kubeseal::new().build(context).await?;
    Lazygit::new().build(context).await?;
    Libassuan::new().build(context).await?;
    Libevent::new().build(context).await?;
    Libgcrypt::new().build(context).await?;
    LibgpgError::new().build(context).await?;
    Libksba::new().build(context).await?;
    Libuv::new().build(context).await?;
    Libwebsockets::new().build(context).await?;
    Lima::new().build(context).await?;
    Mbedtls::new().build(context).await?;
    Ncurses::new().build(context).await?;
    Neovim::new().build(context).await?;
    Nginx::new().build(context).await?;
    Nnn::new().build(context).await?;
    Npth::new().build(context).await?;
    OpenapiGeneratorCli::new().build(context).await?;
    Openjdk::new().build(context).await?;
    PkgConfig::new().build(context).await?;
    Readline::new().build(context).await?;
    Ripgrep::new().build(context).await?;
    Skopeo::new().build(context).await?;
    Sqlite3::new().build(context).await?;
    Starship::new().build(context).await?;
    Terraform::new().build(context).await?;
    Tmux::new().build(context).await?;
    Ttyd::new().build(context).await?;
    Umoci::new().build(context).await?;
    Vhs::new().build(context).await?;
    Yq::new().build(context).await?;
    Zsh::new().build(context).await?;

    // Development Environment

    ProjectEnvironment::new("dev", DEFAULT_SYSTEMS.to_vec())
        .build(context)
        .await?;

    context.run().await
}
