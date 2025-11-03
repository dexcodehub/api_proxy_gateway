use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Registration input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterInput {
    pub tenant_id: Uuid,
    pub email: String,
    pub name: String,
    pub password: String,
}

/// Login input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginInput {
    pub tenant_id: Uuid,
    pub email: String,
    pub password: String,
}

/// Domain user (business view)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthUser {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub email: String,
    pub name: String,
}

/// Domain credentials (hashed)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credentials {
    pub user_id: Uuid,
    pub password_hash: String,
    pub password_algorithm: String,
}

/// Login result (session)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthSession {
    pub user: AuthUser,
    pub token: Option<String>,
}