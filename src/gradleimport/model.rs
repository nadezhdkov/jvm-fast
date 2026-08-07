use serde::Deserialize;

/// Mirrors `dev.jvmfast.gradlebridge.model.JvmfastDependencyModel`'s JSON
/// shape exactly (`Main.toJson` on the Java side) — the Tooling API bridge
/// prints this, and only this, to its own stdout (docs/architecture.md
/// seção 10, step 4).
#[derive(Debug, Deserialize)]
pub struct BridgeModel {
    pub modules: Vec<BridgeModule>,
}

#[derive(Debug, Deserialize)]
pub struct BridgeModule {
    pub name: String,
    pub version: String,
    pub dependencies: Vec<BridgeDependency>,
}

#[derive(Debug, Deserialize)]
pub struct BridgeDependency {
    pub coordinate: String,
    pub version: String,
    pub configuration: String,
}
