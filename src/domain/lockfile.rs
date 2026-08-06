/// Forma de `project.lock` (docs/architecture.md seção 4). Nenhum código
/// lê ou escreve este arquivo ainda — o marco de lockfile I/O adiciona
/// `Serialize`/`Deserialize` quando essa funcionalidade for implementada.
pub struct Lockfile {
    pub version: u32,
    pub manifest_hash: String,
    pub packages: Vec<LockedPackage>,
    pub requests: Vec<LockedRequest>,
}

pub struct LockedPackage {
    pub name: String,
    pub version: String,
    pub sha256: String,
    pub resolved_from: String,
    pub dependencies: Vec<String>,
}

pub struct LockedRequest {
    pub module: String,
    pub name: String,
    pub version: String,
    pub depth: u32,
}
