use axum::{Json, extract::State, http::{StatusCode, HeaderMap}};
use axum_extra::extract::cookie::{Cookie, CookieJar};
use serde::{Deserialize, Serialize};
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, ActiveModelTrait, Set};
use uuid::Uuid;

use service::auth::{service::{AuthService, AuthConfig}, domain::{RegisterInput, LoginInput, AuthUser}};
use service::auth::repo::seaorm::SeaOrmAuthRepository;
use std::sync::Arc;
use argon2::{Argon2, password_hash::{PasswordHasher, SaltString}};
use rand::rngs::OsRng;
use models::{user, user_credentials, tenant};
use chrono::Utc;

#[derive(Clone)]
pub struct ServerAuthConfig {
    pub jwt_secret: String,
}

#[derive(Clone)]
pub struct ServerState {
    pub db: DatabaseConnection,
    pub auth: ServerAuthConfig,
    pub admin_store: std::sync::Arc<crate::admin::ApiKeysStore>,
    pub api_store: std::sync::Arc<service::api_management::ApiStore>,
}

// RegisterInput is provided by service::auth::domain

#[derive(Serialize)]
pub struct RegisterOutput { pub user_id: Uuid }

// LoginInput is provided by service::auth::domain

#[derive(Serialize)]
pub struct MeOutput { pub user_id: Uuid, pub email: String, pub name: String }

// Token creation handled by AuthService

pub async fn register(State(state): State<ServerState>, Json(input): Json<RegisterInput>) -> Result<Json<RegisterOutput>, (StatusCode, String)> {
    // Validate using models helpers
    if let Err(e) = user::validate_email(&input.email) { return Err((StatusCode::BAD_REQUEST, e.to_string())); }
    if let Err(e) = user::validate_name(&input.name) { return Err((StatusCode::BAD_REQUEST, e.to_string())); }
    if input.password.len() < 8 { return Err((StatusCode::BAD_REQUEST, "password too short (>=8)".into())); }

    // Ensure not duplicated for tenant + email
    let existing = user::Entity::find()
        .filter(user::Column::TenantId.eq(input.tenant_id))
        .filter(user::Column::Email.eq(input.email.clone()))
        .one(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    if existing.is_some() { return Err((StatusCode::CONFLICT, "user already exists".into())); }

    // Ensure tenant exists (FK constraint). Create if missing with generated name.
    let maybe_tenant = tenant::Entity::find_by_id(input.tenant_id)
        .one(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    if maybe_tenant.is_none() {
        let am = tenant::ActiveModel {
            id: Set(input.tenant_id),
            name: Set(format!("auto-tenant-{}", input.tenant_id)),
            created_at: Set(Utc::now().into()),
        };
        am.insert(&state.db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    // Create user
    let created = user::create(&state.db, input.tenant_id, &input.email, &input.name)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    // Hash password and upsert credentials
    let salt = SaltString::generate(&mut OsRng);
    let argon = Argon2::default();
    let hash = argon.hash_password(input.password.as_bytes(), &salt)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .to_string();
    let _cred = user_credentials::upsert_password(&state.db, created.id, hash, "argon2")
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(RegisterOutput { user_id: created.id }))
}

pub async fn login(State(state): State<ServerState>, jar: CookieJar, Json(input): Json<LoginInput>) -> Result<(CookieJar, Json<MeOutput>), (StatusCode, String)> {
    let repo = Arc::new(SeaOrmAuthRepository { db: state.db.clone() });
    let svc = AuthService::new(repo, AuthConfig { jwt_secret: Some(state.auth.jwt_secret.clone()), password_algorithm: "argon2".into() });
    let session = svc.login(input).await.map_err(|e| (StatusCode::UNAUTHORIZED, e.to_string()))?;
    let user = session.user;
    if let Some(token) = session.token {
        let mut cookie = Cookie::new("auth_token", token);
        cookie.set_path("/");
        cookie.set_http_only(true);
        cookie.set_secure(false);
        cookie.set_same_site(axum_extra::extract::cookie::SameSite::Lax);
        let jar = jar.add(cookie);
        let me = MeOutput { user_id: user.id, email: user.email, name: user.name };
        return Ok((jar, Json(me)));
    }
    Err((StatusCode::INTERNAL_SERVER_ERROR, "token generation failed".into()))
}

pub async fn logout(jar: CookieJar) -> (CookieJar, StatusCode) {
    let jar = jar.remove(Cookie::from("auth_token"));
    (jar, StatusCode::NO_CONTENT)
}

pub async fn me(State(_state): State<ServerState>, jar: CookieJar) -> Result<Json<MeOutput>, (StatusCode, String)> {
    if let Some(tok) = jar.get("auth_token") {
        // For simplicity, we trust the cookie exists; a full implementation would decode/verify JWT.
        // Here we only return 204 if missing.
        // Token decoding could be added for stricter checks.
        let _ = tok; // placeholder
        return Err((StatusCode::NOT_IMPLEMENTED, "decode not implemented".into()));
    }
    Err((StatusCode::UNAUTHORIZED, "no auth".into()))
}