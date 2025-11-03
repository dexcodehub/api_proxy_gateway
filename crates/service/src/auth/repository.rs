use async_trait::async_trait;
use uuid::Uuid;

use super::domain::{AuthUser, Credentials};
use super::errors::AuthError;

/// Repository abstraction for auth-related persistence.
#[async_trait]
pub trait AuthRepository: Send + Sync {
    async fn find_user_by_tenant_email(&self, tenant_id: Uuid, email: &str) -> Result<Option<AuthUser>, AuthError>;
    async fn create_user(&self, tenant_id: Uuid, email: &str, name: &str) -> Result<AuthUser, AuthError>;

    async fn get_credentials(&self, user_id: Uuid) -> Result<Option<Credentials>, AuthError>;
    async fn upsert_password(&self, user_id: Uuid, password_hash: String, password_algorithm: String) -> Result<Credentials, AuthError>;
}

/// Simple in-memory mock repository for tests and doc examples
pub mod mock {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    #[derive(Default)]
    pub struct MockAuthRepository {
        users: Mutex<HashMap<(Uuid, String), AuthUser>>, // key: (tenant_id, email)
        creds: Mutex<HashMap<Uuid, Credentials>>,        // key: user_id
    }

    #[async_trait]
    impl AuthRepository for MockAuthRepository {
        async fn find_user_by_tenant_email(&self, tenant_id: Uuid, email: &str) -> Result<Option<AuthUser>, AuthError> {
            let users = self.users.lock().unwrap();
            Ok(users.get(&(tenant_id, email.to_string())).cloned())
        }

        async fn create_user(&self, tenant_id: Uuid, email: &str, name: &str) -> Result<AuthUser, AuthError> {
            let mut users = self.users.lock().unwrap();
            if users.contains_key(&(tenant_id, email.to_string())) {
                return Err(AuthError::Conflict);
            }
            let user = AuthUser { id: Uuid::new_v4(), tenant_id, email: email.to_string(), name: name.to_string() };
            users.insert((tenant_id, email.to_string()), user.clone());
            Ok(user)
        }

        async fn get_credentials(&self, user_id: Uuid) -> Result<Option<Credentials>, AuthError> {
            let creds = self.creds.lock().unwrap();
            Ok(creds.get(&user_id).cloned())
        }

        async fn upsert_password(&self, user_id: Uuid, password_hash: String, password_algorithm: String) -> Result<Credentials, AuthError> {
            let mut creds = self.creds.lock().unwrap();
            let c = Credentials { user_id, password_hash, password_algorithm };
            creds.insert(user_id, c.clone());
            Ok(c)
        }
    }
}