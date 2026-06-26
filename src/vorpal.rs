use anyhow::Result;
use vorpal_artifacts::{
    artifact::{
        argocd::Argocd, awscli2::Awscli2, bash_language_server::BashLanguageServer, bat::Bat,
        beads::Beads, bottom::Bottom, cmake::Cmake, crane::Crane, cue::Cue, delta::Delta,
        direnv::Direnv, dockerfile_language_server::DockerfileLanguageServer, doppler::Doppler,
        eksctl::Eksctl, fd::Fd, ffmpeg::Ffmpeg, fluxcd::Fluxcd, fzf::Fzf, glow::Glow,
        golangci_lint::GolangciLint, gpg::Gpg, gum::Gum, helm::Helm, herdr::Herdr, jj::Jj, jq::Jq,
        json_c::JsonC, just::Just, k9s::K9s, kind::Kind, kn::Kn, kubectl::Kubectl,
        kubeseal::Kubeseal, lazygit::Lazygit, libassuan::Libassuan, libevent::Libevent,
        libgcrypt::Libgcrypt, libgpg_error::LibgpgError, libksba::Libksba, libuv::Libuv,
        libwebsockets::Libwebsockets, lima::Lima, lua_language_server::LuaLanguageServer,
        mbedtls::Mbedtls, ncurses::Ncurses, neovim::Neovim, nginx::Nginx, nnn::Nnn, npth::Npth,
        op::Op, openapi_generator_cli::OpenapiGeneratorCli, opencode::Opencode, openjdk::Openjdk,
        pi::Pi, pkg_config::PkgConfig, readline::Readline, ripgrep::Ripgrep, sccache::Sccache,
        sesh::Sesh, skopeo::Skopeo, sqlite3::Sqlite3, starship::Starship, talosctl::Talosctl,
        terraform::Terraform, terraform_ls::TerraformLs, tmux::Tmux, tree_sitter::TreeSitter,
        ttyd::Ttyd, typescript::Typescript, typescript_language_server::TypescriptLanguageServer,
        umoci::Umoci, uv::Uv, vhs::Vhs, vscode_langservers_extracted::VscodeLangserversExtracted,
        yaml_language_server::YamlLanguageServer, yq::Yq, zoxide::Zoxide, zsh::Zsh,
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
    BashLanguageServer::new().build(context).await?;
    Bat::new().build(context).await?;
    Beads::new().build(context).await?;
    Bottom::new().build(context).await?;
    Cmake::new().build(context).await?;
    Crane::new().build(context).await?;
    Cue::new().build(context).await?;
    Delta::new().build(context).await?;
    Direnv::new().build(context).await?;
    DockerfileLanguageServer::new().build(context).await?;
    Doppler::new().build(context).await?;
    Eksctl::new().build(context).await?;
    Fd::new().build(context).await?;
    Ffmpeg::new().build(context).await?;
    Fluxcd::new().build(context).await?;
    Fzf::new().build(context).await?;
    Glow::new().build(context).await?;
    GolangciLint::new().build(context).await?;
    Gpg::new().build(context).await?;
    Gum::new().build(context).await?;
    Helm::new().build(context).await?;
    Herdr::new().build(context).await?;
    Jj::new().build(context).await?;
    Jq::new().build(context).await?;
    JsonC::new().build(context).await?;
    Just::new().build(context).await?;
    K9s::new().build(context).await?;
    Kind::new().build(context).await?;
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
    LuaLanguageServer::new().build(context).await?;
    Mbedtls::new().build(context).await?;
    Ncurses::new().build(context).await?;
    Neovim::new().build(context).await?;
    Nginx::new().build(context).await?;
    Nnn::new().build(context).await?;
    Npth::new().build(context).await?;
    Op::new().build(context).await?;
    OpenapiGeneratorCli::new().build(context).await?;
    Opencode::new().build(context).await?;
    Openjdk::new().build(context).await?;
    Pi::new().build(context).await?;
    PkgConfig::new().build(context).await?;
    Readline::new().build(context).await?;
    Ripgrep::new().build(context).await?;
    Sccache::new().build(context).await?;
    Sesh::new().build(context).await?;
    Skopeo::new().build(context).await?;
    Sqlite3::new().build(context).await?;
    Starship::new().build(context).await?;
    Talosctl::new().build(context).await?;
    Terraform::new().build(context).await?;
    TerraformLs::new().build(context).await?;
    Tmux::new().build(context).await?;
    TreeSitter::new().build(context).await?;
    Ttyd::new().build(context).await?;
    Typescript::new().build(context).await?;
    TypescriptLanguageServer::new().build(context).await?;
    Umoci::new().build(context).await?;
    Uv::new().build(context).await?;
    Vhs::new().build(context).await?;
    VscodeLangserversExtracted::new().build(context).await?;
    YamlLanguageServer::new().build(context).await?;
    Yq::new().build(context).await?;
    Zoxide::new().build(context).await?;
    Zsh::new().build(context).await?;

    // Development Environment

    ProjectEnvironment::new("dev", DEFAULT_SYSTEMS.to_vec())
        .build(context)
        .await?;

    context.run().await
}
