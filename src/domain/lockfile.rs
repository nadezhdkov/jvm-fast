use serde::{Deserialize, Serialize};

/// Forma de `project.lock` (docs/architecture.md seção 4) — deriva
/// `Serialize`/`Deserialize` direto sobre o tipo de domínio (sem DTO
/// separado), já que a forma TOML bate 1:1 com estes campos.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lockfile {
    pub version: u32,
    #[serde(rename = "manifest-hash")]
    pub manifest_hash: String,
    /// Major version da JDK efetivamente selecionada na resolução (seção 3,
    /// seção 7) — sempre concreta (`"21"`), nunca o alias `"lts"`: se
    /// `[project].java-version` no manifesto for `"lts"`, esta é a versão
    /// LTS mais recente no momento em que o lock foi gerado, gravada aqui
    /// para que builds seguintes não troquem de JDK silenciosamente quando
    /// uma nova LTS for lançada — só `jvmfast update` reavalia o alias.
    #[serde(rename = "java-version")]
    pub java_version: String,
    #[serde(rename = "package", default)]
    pub packages: Vec<LockedPackage>,
    #[serde(rename = "request", default)]
    pub requests: Vec<LockedRequest>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockedPackage {
    pub name: String,
    pub version: String,
    pub sha256: String,
    #[serde(rename = "resolved-from")]
    pub resolved_from: String,
    #[serde(default)]
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockedRequest {
    pub module: String,
    pub name: String,
    pub version: String,
    pub depth: u32,
}
