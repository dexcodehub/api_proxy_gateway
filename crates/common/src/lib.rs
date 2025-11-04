use thiserror::Error;

pub mod types;
pub mod crypto;
pub mod utils;
pub mod pagination;
pub mod env;
pub mod admin_http;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("network error: {0}")]
    Network(String),
    #[error("parse error: {0}")]
    Parse(String),
}

pub mod posts {
    use super::*;

    pub async fn fetch_posts() -> Result<serde_json::Value, CoreError> {
        let url = "https://jsonplaceholder.typicode.com/posts";
        let resp = reqwest::get(url)
            .await
            .map_err(|e| CoreError::Network(e.to_string()))?;
        let json = resp
            .json::<serde_json::Value>()
            .await
            .map_err(|e| CoreError::Parse(e.to_string()))?;
        Ok(json)
    }

    pub async fn fetch_post(id: u32) -> Result<serde_json::Value, CoreError> {
        let url = format!("https://jsonplaceholder.typicode.com/posts/{id}");
        let resp = reqwest::get(&url)
            .await
            .map_err(|e| CoreError::Network(e.to_string()))?;
        let json = resp
            .json::<serde_json::Value>()
            .await
            .map_err(|e| CoreError::Parse(e.to_string()))?;
        Ok(json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_type_ok() {
        let h = types::Health { status: "ok" };
        assert_eq!(h.status, "ok");
    }
}
