// `CliError` (src/cli/error.rs) faz fan-in de erros de quase todo
// subsistema (workspace, resolve, download, lockfile, manifest...) — o
// maior deles já passa do limiar de ~128 bytes que o clippy usa para
// `result_large_err`. Boxar cada variante exigiria `impl From` manual em
// vez do `#[from]` do thiserror (que só gera `From<T>`, não
// `From<T> for Box<T>`) por pouco ganho real: `CliError` só é construído
// uma vez por invocação de comando, nunca num hot loop, então o custo de
// cópia extra é irrelevante na prática.
#![allow(clippy::result_large_err)]

mod build;
mod context;
mod edit;
mod error;
mod import;
mod install;
mod jdk;
mod run;
mod test;
mod tree;
mod why;

pub use build::run as build;
pub use edit::{add_dependency, remove_dependency, ManifestEditError};
pub use error::CliError;
pub use import::run as import_pom;
pub use import::run_gradle as import_gradle;
pub use install::{add, install, remove, InstallSummary};
pub use jdk::{
    ensure_project_jdk, install_jdk, list as list_jdks, resolve_project_java_version, use_jdk,
};
pub use run::run as run_program;
pub use test::{run as test, TestOptions};
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
    Install {
        /// Instala a JDK do projeto automaticamente sem pedir confirmação
        /// (seção 7) — para uso em CI.
        #[arg(long)]
        yes: bool,
    },
    /// Regenera project.lock, ignorando a validade do existente.
    Update {
        /// Atualização de uma única coordenada — não suportado ainda.
        coordinate: Option<String>,
        /// Instala a JDK do projeto automaticamente sem pedir confirmação
        /// (seção 7) — para uso em CI.
        #[arg(long)]
        yes: bool,
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
    /// Compila src/main/java para target/classes (exige project.lock válido).
    Build,
    /// Compila e executa [run].main-class (exige project.lock válido).
    Run {
        /// Qual módulo do workspace executar (seção 12, Fase 5); default: raiz.
        #[arg(long)]
        module: Option<String>,
    },
    /// Compila e roda testes via JUnit Platform Console (seção 8.1).
    Test {
        /// `"tag:fast"` filtra por tag JUnit; qualquer outro valor é um
        /// glob de nome de classe (ex. `"*.UserTest"`).
        #[arg(long)]
        filter: Option<String>,
        /// Não suportado — o Console Launcher não tem stop-on-first-failure nativo.
        #[arg(long = "fail-fast")]
        fail_fast: bool,
        /// Grava relatórios JUnit XML em target/test-reports.
        #[arg(long = "report-xml")]
        report_xml: bool,
        /// Restringe a compilação/execução a um módulo do workspace (seção
        /// 12, Fase 5); default: todos os módulos, com [dev-dependencies]
        /// lidas do módulo raiz.
        #[arg(long)]
        module: Option<String>,
    },
    /// Exibe a árvore de dependências resolvida.
    Tree,
    /// Explica a origem de um artefato no grafo resolvido.
    Why { coordinate: String },
    /// Gera project.toml a partir de um pom.xml existente (seção 10).
    ImportPom {
        /// Caminho do pom.xml a importar; default: `pom.xml` na raiz do projeto.
        pom: Option<String>,
    },
    /// Gera project.toml a partir de um projeto Gradle existente, via a
    /// Tooling API (seção 10).
    ImportGradle {
        /// Diretório do projeto Gradle a importar; default: raiz do projeto atual.
        project: Option<String>,
    },
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
    /// Define a JDK default global (já precisa estar instalada).
    Use { version: String },
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
        Command::Install { yes } => install::install(root, false, yes)
            .await
            .map(|s| format_summary(&s)),
        Command::Update {
            coordinate: Some(_),
            ..
        } => Err(CliError::TargetedUpdateNotSupported),
        Command::Update {
            coordinate: None,
            yes,
        } => install::install(root, true, yes)
            .await
            .map(|s| format_summary(&s)),
        Command::Add { coordinate, dev } => install::add(root, &coordinate, dev)
            .await
            .map(|s| format_summary(&s)),
        Command::Remove { coordinate } => install::remove(root, &coordinate)
            .await
            .map(|s| format_summary(&s)),
        Command::Build => build::run(root),
        Command::Run { module } => run::run(root, module),
        Command::Test {
            filter,
            fail_fast,
            report_xml,
            module,
        } => {
            test::run(
                root,
                test::TestOptions {
                    filter,
                    fail_fast,
                    report_xml,
                    module,
                },
            )
            .await
        }
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
        Command::Jdk {
            action: JdkCommand::Use { version },
        } => jdk::use_jdk(&version),
        Command::ImportPom { pom } => import::run(root, pom),
        Command::ImportGradle { project } => import::run_gradle(root, project),
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
