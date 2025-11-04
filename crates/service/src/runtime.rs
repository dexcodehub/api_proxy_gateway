//! Runtime environment helpers
//!
//! Thin wrapper around `common::env` to keep binary crates importing
//! `service::runtime::ensure_env` without depending directly on `common`.

/// Ensure expected directories exist; warn on missing optional ones.
pub async fn ensure_env(frontend_dir: &str, data_dir: &str) -> anyhow::Result<()> {
    common::env::ensure_env(frontend_dir, data_dir).await
}