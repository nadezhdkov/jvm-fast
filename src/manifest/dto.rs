use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer};
use std::collections::HashMap;
use std::fmt;

/// Forma bruta de `project.toml` (docs/architecture.md seção 3). Diverge
/// do modelo de domínio (`Module`) o bastante — `[project]`, `[repositories]`,
/// `[run]` não têm campo equivalente em `Module` — para justificar um DTO
/// separado em vez de derivar serde direto no tipo de domínio
/// (docs/CONVENTIONS.md).
#[derive(Deserialize)]
pub struct ProjectManifest {
    pub project: ProjectSection,
    #[serde(default)]
    pub dependencies: HashMap<String, DependencyValue>,
    #[serde(rename = "dev-dependencies", default)]
    pub dev_dependencies: HashMap<String, DependencyValue>,
    #[serde(default)]
    pub boms: HashMap<String, String>,
    #[serde(default)]
    pub exclusions: HashMap<String, Vec<String>>,
    #[serde(default)]
    pub repositories: HashMap<String, String>,
    pub run: Option<RunSection>,
}

#[derive(Deserialize)]
pub struct ProjectSection {
    pub name: String,
    pub version: String,
    #[serde(rename = "java-version")]
    pub java_version: String,
    #[serde(rename = "source-encoding", default = "default_encoding")]
    pub source_encoding: String,
}

fn default_encoding() -> String {
    "UTF-8".to_string()
}

#[derive(Deserialize)]
pub struct RunSection {
    #[serde(rename = "main-class")]
    pub main_class: Option<String>,
    #[serde(rename = "jvm-args", default)]
    pub jvm_args: Vec<String>,
}

/// O valor de uma dependência é uma versão explícita, ou `true` para
/// "gerenciado por BOM" (docs/architecture.md seção 3.3). `false` nunca é
/// válido — rejeitado já na desserialização, não como um valor aceito e
/// filtrado depois.
pub enum DependencyValue {
    Explicit(String),
    BomManaged,
}

impl<'de> Deserialize<'de> for DependencyValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct DependencyValueVisitor;

        impl<'de> Visitor<'de> for DependencyValueVisitor {
            type Value = DependencyValue;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a version string or `true` (BOM-managed)")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(DependencyValue::Explicit(v.to_string()))
            }

            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(DependencyValue::Explicit(v))
            }

            fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if v {
                    Ok(DependencyValue::BomManaged)
                } else {
                    Err(E::custom(
                        "dependency value `false` is not valid; use a version string or `true` for BOM-managed",
                    ))
                }
            }
        }

        deserializer.deserialize_any(DependencyValueVisitor)
    }
}
