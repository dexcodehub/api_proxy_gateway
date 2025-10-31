use thiserror::Error;

#[derive(Debug, Error)]
pub enum ModelError {
    #[error("validation error: {0}")]
    Validation(String),
    #[error("database error: {0}")]
    Db(String),
}