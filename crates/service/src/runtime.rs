use tracing::warn;

/// Ensure expected directories exist; warn on missing optional ones.
pub async fn ensure_env(frontend_dir: &str, data_dir: &str) -> anyhow::Result<()> {
    if tokio::fs::metadata(frontend_dir).await.is_err() {
        warn!("{frontend_dir} not found; static assets may 404");
    }
    tokio::fs::create_dir_all(data_dir).await.map_err(|e| anyhow::anyhow!("cannot create {data_dir}: {e}"))?;
    Ok(())
}