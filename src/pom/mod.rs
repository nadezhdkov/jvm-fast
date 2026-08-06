mod error;
mod xml;

pub use error::PomParseError;
pub use xml::parse_pom_xml;

/// Uma dependência declarada em `<dependencies>` — versão sempre concreta
/// (interpolação de `${propriedade}` e herança de POM pai não são
/// suportadas nesta passada; um POM real que dependa disso faz o parser
/// falhar silenciosamente para uma string vazia/literal, não um erro — ver
/// nota em `docs/architecture.md` seção 13 sobre a verbosidade de metadados
/// Maven).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PomDependency {
    pub coordinate: String,
    pub version: String,
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
