use thiserror::Error;

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("validation error: {0}")]
    Validation(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("database error: {0}")]
    Db(String),
    #[error("model error: {0}")]
    Model(#[from] models::errors::ModelError),
}

impl ServiceError {
    pub fn not_found(entity: &str) -> Self { Self::NotFound(format!("{} not found", entity)) }
}