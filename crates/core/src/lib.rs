use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod types {
    use super::*;

    #[derive(Serialize, Deserialize, Debug)]
    pub struct Health {
        pub status: &'static str,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct Post {
        pub userId: Option<u32>,
        pub id: Option<u32>,
        pub title: String,
        pub body: String,
    }
}

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
    use crate::types::Health;

    #[test]
    fn health_type_ok() {
        let h = Health { status: "ok" };
        assert_eq!(h.status, "ok");
    }
}
