// `CliError` (src/cli/error.rs) faz fan-in de erros de quase todo
// subsistema (workspace, resolve, download, lockfile, manifest...) — o
// maior deles já passa do limiar de ~128 bytes que o clippy usa para
// `result_large_err`. Boxar cada variante exigiria `impl From` manual em
// vez do `#[from]` do thiserror (que só gera `From<T>`, não
// `From<T> for Box<T>`) por pouco ganho real: `CliError` só é construído
// uma vez por invocação de comando, nunca num hot loop, então o custo de
// cópia extra é irrelevante na prática.
#![allow(clippy::result_large_err)]

mod context;
mod edit;
mod error;
mod install;
mod jdk;
mod tree;
mod why;

pub use edit::{add_dependency, remove_dependency, ManifestEditError};
pub use error::CliError;
pub use install::{add, install, remove, InstallSummary};
pub use tree::format_tree;
pub use why::format_why;

use crate::domain::Module;
use crate::pom::HttpPomProvider;
use crate::resolve::{resolve, ResolveOutput};
use crate::workspace::load_workspace;
use clap::{Parser, Subcommand};
use std::path::Path;

#[derive(Parser)]
#[command(
    name = "jvmfast",
    version,
    about = "jvm-fast: gerenciador de dependências e build para Java"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Resolve e baixa conforme project.lock (ou gera um se ausente/inválido).
    Install,
    /// Regenera project.lock, ignorando a validade do existente.
    Update {
        /// Atualização de uma única coordenada — não suportado ainda.
        coordinate: Option<String>,
    },
    /// Adiciona uma dependência a [dependencies] e resolve.
    Add {
        /// `groupId:artifactId@version` — versão sempre explícita nesta versão.
        coordinate: String,
        #[arg(long)]
        dev: bool,
    },
    /// Remove uma dependência de [dependencies] e re-resolve.
    Remove { coordinate: String },
    /// Exibe a árvore de dependências resolvida.
    Tree,
    /// Explica a origem de um artefato no grafo resolvido.
    Why { coordinate: String },
    /// Gerenciamento de JDK — instala/lista distribuições Temurin (seção 7).
    Jdk {
        #[command(subcommand)]
        action: JdkCommand,
    },
}

#[derive(Subcommand)]
pub enum JdkCommand {
    /// Instala a última release Temurin de uma major version (ex. `21`).
    Install { version: String },
    /// Lista as JDKs instaladas.
    List,
}

/// Ponto de entrada único, chamado por `main.rs` dentro de um runtime
/// tokio já iniciado — `main.rs` fica deliberadamente fino (só
/// `Cli::parse()` + `run`), toda a lógica testável vive aqui e nos módulos
/// irmãos.
pub async fn run(cli: Cli) -> std::process::ExitCode {
    let root = std::env::current_dir().expect("current directory should be accessible");
    match dispatch(&root, cli.command).await {
        Ok(message) => {
            println!("{message}");
            std::process::ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("error: {err}");
            std::process::ExitCode::FAILURE
        }
    }
}

async fn dispatch(root: &Path, command: Command) -> Result<String, CliError> {
    match command {
        Command::Install => install::install(root, false)
            .await
            .map(|s| format_summary(&s)),
        Command::Update {
            coordinate: Some(_),
        } => Err(CliError::TargetedUpdateNotSupported),
        Command::Update { coordinate: None } => install::install(root, true)
            .await
            .map(|s| format_summary(&s)),
        Command::Add { coordinate, dev } => install::add(root, &coordinate, dev)
            .await
            .map(|s| format_summary(&s)),
        Command::Remove { coordinate } => install::remove(root, &coordinate)
            .await
            .map(|s| format_summary(&s)),
        Command::Tree => {
            let workspace = load_workspace(root)?;
            let output = resolve_for_diagnostics(root, workspace.modules.clone()).await?;
            Ok(format_tree(
                &output.graph,
                &output.module_roots,
                &workspace.modules,
            ))
        }
        Command::Why { coordinate } => {
            let workspace = load_workspace(root)?;
            let output = resolve_for_diagnostics(root, workspace.modules.clone()).await?;
            format_why(&output.graph, &output.module_roots, &coordinate)
                .ok_or(CliError::CoordinateNotResolved(coordinate))
        }
        Command::Jdk {
            action: JdkCommand::Install { version },
        } => jdk::install_jdk(&version).await,
        Command::Jdk {
            action: JdkCommand::List,
        } => jdk::list(),
    }
}

/// `tree`/`why` resolvem de novo em memória em vez de reconstruir a partir
/// de um `project.lock` existente — ver gap documentado em
/// `src/cli/why.rs`.
async fn resolve_for_diagnostics(
    root: &Path,
    modules: Vec<Module>,
) -> Result<ResolveOutput, CliError> {
    let base_url = context::resolve_base_url(root)?;
    // Ver comentário equivalente em `install.rs`: construir o
    // `HttpPomProvider` (reqwest::blocking) precisa acontecer dentro do
    // `spawn_blocking`, não antes.
    let output = tokio::task::spawn_blocking(move || {
        let provider = HttpPomProvider::new(base_url);
        resolve(&modules, &provider)
    })
    .await??;
    Ok(output)
}

fn format_summary(summary: &InstallSummary) -> String {
    if summary.reused_lockfile {
        format!(
            "{} package(s) in project.lock — {} downloaded, {} already cached",
            summary.package_count, summary.downloaded_count, summary.reused_from_cache_count
        )
    } else {
        format!(
            "resolved {} package(s) — {} downloaded, {} already cached — project.lock written",
            summary.package_count, summary.downloaded_count, summary.reused_from_cache_count
        )
    }
}
