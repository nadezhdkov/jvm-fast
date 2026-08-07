use crate::cache::CacheStore;
use crate::cli::context::{cache_root, resolve_base_url};
use crate::cli::edit::{add_dependency, remove_dependency};
use crate::cli::jdk::{ensure_project_jdk, resolve_project_java_version};
use crate::cli::CliError;
use crate::domain::{DependencyGraph, Workspace};
use crate::download::{ArtifactRequest, DownloadClient};
use crate::lockfile::{build_lockfile, is_lockfile_valid, write_lockfile};
use crate::maven::{artifact_filename, artifact_url};
use crate::pom::HttpPomProvider;
use crate::resolve::resolve;
use crate::workspace::{current_manifest_hash, load_workspace};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

#[derive(Debug)]
pub struct InstallSummary {
    pub package_count: usize,
    pub downloaded_count: usize,
    pub reused_from_cache_count: usize,
    /// `true` quando o `project.lock` existente já era válido e a
    /// resolução inteira (passos 3–5 da seção 6.2) foi pulada — só passo 6
    /// (download) rodou, exatamente como o fluxograma da seção 6 descreve.
    pub reused_lockfile: bool,
}

/// `jvmfast install`/`jvmfast update` (seção 6.2 passos 1–8 completos, a
/// primeira vez que todo o pipeline roda ponta a ponta: carregar workspace
/// → checar validade do lock → resolver (BOMs+grafo+mediação) → baixar
/// artefatos ausentes do cache → gerar/escrever `project.lock`).
///
/// `force=true` (usado por `update`) ignora a validade do lock existente e
/// sempre re-resolve; `force=false` (usado por `install`) segue o
/// fluxograma da seção 6: lock válido → pula direto para o download dos
/// artefatos ainda ausentes do cache, sem re-fetch de POM nenhum.
pub async fn install(root: &Path, force: bool, yes: bool) -> Result<InstallSummary, CliError> {
    let workspace = load_workspace(root)?;
    let current_hash = current_manifest_hash(root)?;
    // `load_workspace` sintetiza um `Lockfile` vazio com o hash *atual* já
    // preenchido quando `project.lock` não existe em disco (estado honesto
    // de "nunca resolvido", seção 4) — `is_lockfile_valid` sozinho não
    // distingue isso de um lock real que bate por coincidência, então a
    // existência do arquivo em disco entra na conta também.
    let lock_exists = root.join("project.lock").is_file();
    let lock_is_valid =
        !force && lock_exists && is_lockfile_valid(&workspace.lockfile, &current_hash);

    let cache_store = Arc::new(CacheStore::new(cache_root()));
    let download_client = DownloadClient::new(&workspace.config.network)?;
    let max_concurrent = workspace.config.network.concurrent_downloads.max(1) as usize;

    if lock_is_valid {
        ensure_project_jdk(&workspace.lockfile.java_version, yes).await?;
        let (downloaded, reused) = download_locked_packages(
            &workspace,
            &download_client,
            Arc::clone(&cache_store),
            max_concurrent,
        )
        .await?;
        return Ok(InstallSummary {
            package_count: workspace.lockfile.packages.len(),
            downloaded_count: downloaded,
            reused_from_cache_count: reused,
            reused_lockfile: true,
        });
    }

    let java_version = resolve_project_java_version(root, yes).await?;

    let base_url = resolve_base_url(root)?;
    let resolve_output = {
        let modules = workspace.modules.clone();
        let provider_base_url = base_url.clone();
        // `HttpPomProvider` usa `reqwest::blocking`, que constrói seu
        // próprio runtime tokio internamente — construí-lo (não só usá-lo)
        // de dentro do runtime async já em execução entra em pânico
        // ("Cannot drop a runtime..."), então tanto a criação quanto o uso
        // do provider precisam estar dentro do `spawn_blocking`.
        tokio::task::spawn_blocking(move || {
            let provider = HttpPomProvider::new(provider_base_url);
            resolve(&modules, &provider)
        })
        .await??
    };

    let (checksums, requests, reused) = plan_downloads(
        &resolve_output.graph,
        &base_url,
        &download_client,
        &cache_store,
    )
    .await?;
    let downloaded = requests.len();

    let results = download_client
        .download_many(requests, Arc::clone(&cache_store), max_concurrent)
        .await;
    let failed = results.iter().filter(|r| r.is_err()).count();
    if failed > 0 {
        return Err(CliError::DownloadsFailed(failed, downloaded));
    }

    let lockfile = build_lockfile(
        &resolve_output.graph,
        current_hash,
        &checksums,
        &base_url,
        &java_version,
    )?;
    write_lockfile(&root.join("project.lock"), &lockfile)?;

    Ok(InstallSummary {
        package_count: downloaded + reused,
        downloaded_count: downloaded,
        reused_from_cache_count: reused,
        reused_lockfile: false,
    })
}

/// `jvmfast add <coord>@<version>` (seção 9.3, escopo reduzido: exige
/// versão explícita — "latest release" via consulta de metadata do
/// repositório é um marco futuro, não implementado ainda, então é rejeitado
/// como erro tipado em vez de silenciosamente escolher algo). Edita
/// `project.toml` e então re-resolve (`force=true`), igual a `update`.
pub async fn add(
    root: &Path,
    coordinate_spec: &str,
    dev: bool,
) -> Result<InstallSummary, CliError> {
    if dev {
        return Err(CliError::DevDependenciesNotSupported);
    }
    let (coordinate, version) = coordinate_spec
        .split_once('@')
        .ok_or_else(|| CliError::VersionOmittedNotSupported(coordinate_spec.to_string()))?;

    add_dependency(&root.join("project.toml"), coordinate, version)?;
    // `add`/`remove` só editam `[dependencies]`, nunca `[project].java-version`
    // — não faz sentido bloquear a edição do manifesto numa confirmação
    // interativa de JDK por causa disso, então sempre se comportam como
    // `--yes` para esse passo específico (mesmo raciocínio de `force=true`
    // logo abaixo: sempre re-resolvem).
    install(root, true, true).await
}

/// `jvmfast remove <coord>` — remove do manifesto e re-resolve.
pub async fn remove(root: &Path, coordinate: &str) -> Result<InstallSummary, CliError> {
    let removed = remove_dependency(&root.join("project.toml"), coordinate)?;
    if !removed {
        return Err(CliError::DependencyNotDeclared(coordinate.to_string()));
    }
    install(root, true, true).await
}

/// Passo 6 da seção 6.2 quando o lock já é válido: nenhum POM é buscado de
/// novo — o próprio `project.lock` já tem `sha256`/`resolved-from` por
/// pacote, então cada artefato ausente do cache é reconstruído direto dele.
async fn download_locked_packages(
    workspace: &Workspace,
    download_client: &DownloadClient,
    cache_store: Arc<CacheStore>,
    max_concurrent: usize,
) -> Result<(usize, usize), CliError> {
    let mut requests = Vec::new();
    let mut reused = 0;

    for package in &workspace.lockfile.packages {
        let filename = artifact_filename(&package.name, &package.version, "jar")?;
        if cache_store.is_cached(&package.sha256, &filename) {
            reused += 1;
            continue;
        }
        let url = artifact_url(
            &package.resolved_from,
            &package.name,
            &package.version,
            "jar",
        )?;
        requests.push(ArtifactRequest {
            url,
            filename,
            expected_sha256: package.sha256.clone(),
        });
    }

    let downloaded = requests.len();
    let results = download_client
        .download_many(requests, cache_store, max_concurrent)
        .await;
    let failed = results.iter().filter(|r| r.is_err()).count();
    if failed > 0 {
        return Err(CliError::DownloadsFailed(failed, downloaded));
    }
    Ok((downloaded, reused))
}

/// Para cada nó do grafo mediado, busca o checksum publicado (sidecar
/// `.sha256`, seção 6.2 passo 7 — "ou do repositório, se o lock está sendo
/// gerado agora") e monta a lista de artefatos que de fato precisam ser
/// baixados (pulando os já presentes no cache local pelo hash).
async fn plan_downloads(
    graph: &DependencyGraph,
    base_url: &str,
    download_client: &DownloadClient,
    cache_store: &CacheStore,
) -> Result<(HashMap<String, String>, Vec<ArtifactRequest>, usize), CliError> {
    let mut checksums = HashMap::new();
    let mut requests = Vec::new();
    let mut reused = 0;

    for node in graph.nodes.values() {
        let jar_url = artifact_url(base_url, &node.coordinate, &node.selected, "jar")?;
        let checksum = download_client.fetch_checksum(&jar_url).await?;
        let filename = artifact_filename(&node.coordinate, &node.selected, "jar")?;

        checksums.insert(
            format!("{}@{}", node.coordinate, node.selected),
            checksum.clone(),
        );

        if cache_store.is_cached(&checksum, &filename) {
            reused += 1;
        } else {
            requests.push(ArtifactRequest {
                url: jar_url,
                filename,
                expected_sha256: checksum,
            });
        }
    }

    Ok((checksums, requests, reused))
}
