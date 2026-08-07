use super::error::RunError;
use std::path::{Path, PathBuf};
use std::process::ExitStatus;

/// Executa `java -cp <classpath> <jvm-args> <main-class>` (seção 8) com
/// stdio herdado do processo pai — a saída do programa do usuário aparece
/// direto no terminal, `jvmfast run` nunca captura/reformata. `classpath`
/// já inclui tanto o `target/classes` compilado quanto as dependências do
/// `project.lock` — montagem disso é responsabilidade de quem chama
/// (`cli::run`), não deste módulo, que só sabe invocar `java`.
pub fn run_main_class(
    java: &Path,
    classpath: &[PathBuf],
    jvm_args: &[String],
    main_class: &str,
) -> Result<ExitStatus, RunError> {
    let classpath_value = std::env::join_paths(classpath).map_err(RunError::Classpath)?;

    let mut command = std::process::Command::new(java);
    command.arg("-cp").arg(classpath_value);
    command.args(jvm_args);
    command.arg(main_class);

    command.status().map_err(|source| RunError::Spawn {
        path: java.to_path_buf(),
        source,
    })
}
