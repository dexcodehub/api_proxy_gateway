use thiserror::Error;

/// Business errors for auth workflows
#[derive(Debug, Error)]
pub enum AuthError {
    #[error("validation failed: {0}")]
    Validation(String),
    #[error("user already exists")]
    Conflict,
    #[error("user not found")]
    NotFound,
    #[error("invalid credentials")]
    Unauthorized,
    #[error("hashing error: {0}")]
    HashError(String),
    #[error("token error: {0}")]
    TokenError(String),
    #[error("repository error: {0}")]
    Repository(String),
}

impl AuthError {
    /// Stable numeric code for external mapping/logging
    pub fn code(&self) -> u16 {
        match self {
            AuthError::Validation(_) => 1001,
            AuthError::Conflict => 1002,
            AuthError::NotFound => 1003,
            AuthError::Unauthorized => 1004,
            AuthError::HashError(_) => 1101,
            AuthError::TokenError(_) => 1102,
            AuthError::Repository(_) => 1200,
        }
    }
}