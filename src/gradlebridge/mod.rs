mod error;

pub use error::GradleBridgeError;

use crate::cache::CacheStore;
use std::path::PathBuf;

/// `jvmfast-gradle-bridge.jar` (seção 10) — built from
/// [`gradle-bridge/`](../../gradle-bridge) by `build.rs` at `cargo build`
/// time and embedded straight into the `jvmfast` binary, so `jvmfast
/// import-gradle` never needs a runtime download to obtain it. Today the
/// jar only carries the model/plugin classes
/// (`JvmfastModelBuilderPlugin`/`JvmfastModelBuilder`) meant to be applied
/// via a generated init-script inside the *target* build — the Tooling API
/// client-side driver that would actually invoke it is a separate,
/// not-yet-implemented piece (see CLAUDE.md's Fase 4 gaps).
const BRIDGE_JAR_BYTES: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/jvmfast-gradle-bridge.jar"));

/// Filename the extracted jar is cached under (seção 5's `CacheStore`
/// convention — content-addressable by SHA-256, so a rebuilt bridge jar
/// with different bytes never collides with a stale extracted copy).
const BRIDGE_JAR_FILENAME: &str = "jvmfast-gradle-bridge.jar";

/// Extracts the embedded bridge jar to `<cache_root>/artifacts/sha256/...`
/// (same atomic temp-file → verify → rename discipline as any other cached
/// artifact, seção 5.1) and returns its path. Idempotent and safe to call
/// on every `jvmfast import-gradle` invocation — a matching path already on
/// disk is reused without rewriting it.
pub fn extract_bridge_jar(cache_root: &std::path::Path) -> Result<PathBuf, GradleBridgeError> {
    let store = CacheStore::new(cache_root);
    let sha256 = crate::cache::hash_bytes(BRIDGE_JAR_BYTES);
    store
        .write_artifact(BRIDGE_JAR_BYTES, &sha256, BRIDGE_JAR_FILENAME)
        .map_err(GradleBridgeError::Cache)
}
