use std::sync::Arc;

use argon2::{Argon2, password_hash::{PasswordHasher, PasswordVerifier, SaltString}, PasswordHash};
use jsonwebtoken::{encode, Header as JwtHeader, EncodingKey};
use rand::rngs::OsRng;
use tracing::{info, debug, instrument};

use super::domain::{RegisterInput, LoginInput, AuthUser, AuthSession};
use super::errors::AuthError;
use super::repository::AuthRepository;

/// Auth service configuration
#[derive(Clone)]
pub struct AuthConfig {
    pub jwt_secret: Option<String>,
    pub password_algorithm: String,
}

/// Auth business service independent of web framework
pub struct AuthService<R: AuthRepository> {
    repo: Arc<R>,
    cfg: AuthConfig,
}

impl<R: AuthRepository> AuthService<R> {
    pub fn new(repo: Arc<R>, cfg: AuthConfig) -> Self { Self { repo, cfg } }

    /// Register a new user with a hashed password.
    ///
    /// # Examples
    /// ```
    /// use service::auth::{service::{AuthService, AuthConfig}, repository::mock::MockAuthRepository};
    /// use service::auth::domain::RegisterInput;
    /// use std::sync::Arc;
    /// let repo = Arc::new(MockAuthRepository::default());
    /// let svc = AuthService::new(repo, AuthConfig { jwt_secret: None, password_algorithm: "argon2".into() });
    /// let input = RegisterInput { tenant_id: uuid::Uuid::new_v4(), email: "user@example.com".into(), name: "Test".into(), password: "Secret123".into() };
    /// let user = tokio_test::block_on(svc.register(input)).unwrap();
    /// assert_eq!(user.email, "user@example.com");
    /// ```
    #[instrument(skip(self, input), fields(email = %input.email, tenant_id = %input.tenant_id))]
    pub async fn register(&self, input: RegisterInput) -> Result<AuthUser, AuthError> {
        if input.password.len() < 8 {
            return Err(AuthError::Validation("password too short (>=8)".into()));
        }
        if let Some(existing) = self.repo.find_user_by_tenant_email(input.tenant_id, &input.email).await? {
            debug!("user exists: {}", existing.email);
            return Err(AuthError::Conflict);
        }

        let user = self.repo.create_user(input.tenant_id, &input.email, &input.name).await?;
        let salt = SaltString::generate(&mut OsRng);
        let hash = Argon2::default()
            .hash_password(input.password.as_bytes(), &salt)
            .map_err(|e| AuthError::HashError(e.to_string()))?
            .to_string();

        let _cred = self.repo.upsert_password(user.id, hash, self.cfg.password_algorithm.clone()).await?;
        info!(user_id = %user.id, tenant_id = %user.tenant_id, email = %user.email, "user_registered");
        Ok(user)
    }

    /// Authenticate a user and optionally issue a token.
    ///
    /// # Examples
    /// ```
    /// use service::auth::{service::{AuthService, AuthConfig}, repository::mock::MockAuthRepository};
    /// use service::auth::domain::{RegisterInput, LoginInput};
    /// use std::sync::Arc;
    /// let repo = Arc::new(MockAuthRepository::default());
    /// let svc = AuthService::new(repo.clone(), AuthConfig { jwt_secret: Some("secret".into()), password_algorithm: "argon2".into() });
    /// let tid = uuid::Uuid::new_v4();
    /// let _ = tokio_test::block_on(svc.register(RegisterInput { tenant_id: tid, email: "u@e.com".into(), name: "N".into(), password: "Passw0rd".into() }));
    /// let session = tokio_test::block_on(svc.login(LoginInput { tenant_id: tid, email: "u@e.com".into(), password: "Passw0rd".into() })).unwrap();
    /// assert_eq!(session.user.email, "u@e.com");
    /// assert!(session.token.is_some());
    /// ```
    #[instrument(skip(self, input), fields(email = %input.email, tenant_id = %input.tenant_id))]
    pub async fn login(&self, input: LoginInput) -> Result<AuthSession, AuthError> {
        let user = self.repo
            .find_user_by_tenant_email(input.tenant_id, &input.email)
            .await?
            .ok_or(AuthError::Unauthorized)?;

        let cred = self.repo
            .get_credentials(user.id)
            .await?
            .ok_or(AuthError::Unauthorized)?;

        let parsed = PasswordHash::new(&cred.password_hash).map_err(|e| AuthError::HashError(e.to_string()))?;
        if Argon2::default().verify_password(input.password.as_bytes(), &parsed).is_err() {
            return Err(AuthError::Unauthorized);
        }

        let mut token = None;
        if let Some(secret) = &self.cfg.jwt_secret {
            #[derive(serde::Serialize)]
            struct Claims { sub: String, uid: String, tid: String, exp: usize }
            let exp = (chrono::Utc::now() + chrono::Duration::hours(12)).timestamp() as usize;
            let claims = Claims { sub: user.email.clone(), uid: user.id.to_string(), tid: user.tenant_id.to_string(), exp };
            token = Some(encode(&JwtHeader::default(), &claims, &EncodingKey::from_secret(secret.as_bytes())).map_err(|e| AuthError::TokenError(e.to_string()))?);
        }

        Ok(AuthSession { user, token })
    }
}