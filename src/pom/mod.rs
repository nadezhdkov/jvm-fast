mod error;
mod http;
mod xml;

pub use error::PomParseError;
pub use http::{HttpPomError, HttpPomProvider};
pub use xml::parse_pom_xml;

/// Uma dependência declarada em `<dependencies>` — versão sempre concreta
/// (interpolação de `${propriedade}` e herança de POM pai não são
/// suportadas nesta passada; um POM real que dependa disso faz o parser
/// falhar silenciosamente para uma string vazia/literal, não um erro — ver
/// nota em `docs/architecture.md` seção 13 sobre a verbosidade de metadados
/// Maven).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PomDependency {
    pub coordinate: String,
    pub version: String,
    /// `<scope>` cru do XML — string vazia é o default do Maven
    /// (`compile`), nunca tratada como "sem escopo"/ausente. Interpretação
    /// de quais escopos propagam transitivamente vive em
    /// `graph::build_graph` (`compile`/`runtime`/vazio propagam;
    /// `test`/`provided`/`system` não — seção 6.2), não aqui: este tipo só
    /// espelha o XML, sem embutir regra de resolução.
    pub scope: String,
    /// Coordenadas `groupId:artifactId` listadas em `<exclusions>` — usado
    /// só por `crate::import` (seção 10, `jvmfast import-pom`) para
    /// preencher `[exclusions]`; a resolução normal (`crate::graph`) nunca
    /// lê este campo, exclusões de projeto vêm de `Module.exclusions`
    /// (declaradas em `project.toml`, seção 3.4), não do POM de uma
    /// transitiva.
    pub exclusions: Vec<String>,
}

/// Uma entrada de `<dependencyManagement>` já interpretada.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ManagedDependencyEntry {
    pub coordinate: String,
    pub version: String,
    /// `true` para uma entrada `<type>pom</type><scope>import</scope>` —
    /// import transitivo de outro BOM, não uma versão gerenciada direta.
    pub is_bom_import: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ParsedPom {
    pub dependencies: Vec<PomDependency>,
    pub managed_dependencies: Vec<ManagedDependencyEntry>,
    /// `<project><artifactId>` — vazio se ausente (POM depende de um
    /// `<parent>` para isso; herança de POM pai não é suportada, mesma
    /// lacuna documentada em `crate::pom::xml`). Só consumido por
    /// `crate::import`; a resolução normal nunca precisa do nome do
    /// próprio projeto.
    pub project_artifact_id: String,
    /// `<project><version>` — vazio se ausente (herdada de `<parent>`,
    /// mesma lacuna acima). Só consumido por `crate::import`.
    pub project_version: String,
    /// `<project><properties>`, cru — usado por `crate::import` para
    /// interpolar `${propriedade}` em `<version>` no momento do import
    /// (docs/architecture.md seção 10). A resolução normal
    /// (`crate::graph`/`crate::bom`) nunca interpola propriedades — POMs
    /// reais que dependem disso continuam fora do escopo documentado em
    /// `crate::pom::xml`.
    pub properties: std::collections::HashMap<String, String>,
    /// `<project><repositories><repository>`, em ordem de declaração —
    /// `(id, url)`, `id` vazio se a entrada não declarar um. Só consumido
    /// por `crate::import`.
    pub repositories: Vec<(String, String)>,
    /// `<project><profiles>` estava presente — jvm-fast não tem
    /// equivalente, `crate::import` só reporta a presença.
    pub has_profiles: bool,
    /// `<project><build><plugins>` estava presente — mesma lógica de
    /// `has_profiles`.
    pub has_plugins: bool,
}

/// Abstrai de onde um POM vem — fixture local em teste hoje, HTTP + cache
/// (seção 5/6.2) quando o marco de download existir. Usado tanto pela
/// resolução de BOMs (`crate::bom`) quanto pela construção do grafo de
/// transitivas (`crate::graph`) — as duas consultam POMs pela mesma via.
pub trait PomProvider {
    fn fetch(
        &self,
        coordinate: &str,
        version: &str,
    ) -> Result<ParsedPom, Box<dyn std::error::Error + Send + Sync>>;
}
