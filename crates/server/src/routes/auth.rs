use axum::{Json, extract::{State, Request}, http::StatusCode, middleware::Next, response::Response};
use axum_extra::extract::cookie::{Cookie, CookieJar};
use serde::{Deserialize, Serialize};
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, ActiveModelTrait, Set};
use uuid::Uuid;

use service::{auth::{domain::{ LoginInput, RegisterInput}, service::{AuthConfig, AuthService}}, file::{admin_kv_store::ApiKeysStore, api_management::ApiStore}};
use service::auth::repo::seaorm::SeaOrmAuthRepository;
use std::sync::Arc;
use argon2::{Argon2, password_hash::{PasswordHasher, SaltString}};
use rand::rngs::OsRng;
use models::{user, user_credentials, tenant};
use chrono::Utc;
use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};
// use proper attribute form: #[utoipa::path] on handlers

#[derive(Clone)]
pub struct ServerAuthConfig {
    pub jwt_secret: String,
}

#[derive(Clone)]
pub struct ServerState {
    pub db: DatabaseConnection,
    pub auth: ServerAuthConfig,
    pub admin_store: std::sync::Arc<ApiKeysStore>,
    pub api_store: std::sync::Arc<ApiStore>,
}

// RegisterInput is provided by service::auth::domain

#[derive(Serialize)]
pub struct RegisterOutput { pub user_id: Uuid }

// LoginInput is provided by service::auth::domain

#[derive(Serialize)]
pub struct MeOutput { pub user_id: Uuid, pub email: String, pub name: String }

#[derive(Serialize)]
pub struct LoginOutput { pub user_id: Uuid, pub email: String, pub name: String, pub token: String }

// Token creation handled by AuthService

#[utoipa::path(post, path = "/auth/register", tag = "auth", request_body = crate::openapi::RegisterRequest, responses((status = 200, description = "Registered"), (status = 400, description = "Bad Request"), (status = 409, description = "Conflict")))]
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

#[utoipa::path(post, path = "/auth/login", tag = "auth", request_body = crate::openapi::LoginRequest, responses((status = 200, description = "Logged In"), (status = 401, description = "Unauthorized")))]
pub async fn login(State(state): State<ServerState>, jar: CookieJar, Json(input): Json<LoginInput>) -> Result<(CookieJar, Json<LoginOutput>), (StatusCode, String)> {
    let repo = Arc::new(SeaOrmAuthRepository { db: state.db.clone() });
    let svc = AuthService::new(repo, AuthConfig { jwt_secret: Some(state.auth.jwt_secret.clone()), password_algorithm: "argon2".into() });
    let session = svc.login(input).await.map_err(|e| (StatusCode::UNAUTHORIZED, e.to_string()))?;
    let user = session.user;
    if let Some(token) = session.token {
        let mut cookie = Cookie::new("auth_token", token.clone());
        cookie.set_path("/");
        cookie.set_http_only(true);
        cookie.set_secure(false);
        cookie.set_same_site(axum_extra::extract::cookie::SameSite::Lax);
        let jar = jar.add(cookie);
        let out = LoginOutput { user_id: user.id, email: user.email, name: user.name, token };
        return Ok((jar, Json(out)));
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
#[derive(Debug, Deserialize)]
struct Claims {
    sub: Option<String>,
    exp: Option<usize>,
    iat: Option<usize>,
}

/// 全局中间件：除健康检查与预检外，校验 Authorization: Bearer <token>
/// 缺失 token 返回 400，非法或过期返回 401；失败记录日志
pub async fn require_bearer_token_state(
    State(state): State<ServerState>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let path = req.uri().path();
    let method = req.method().clone();

    // 白名单：健康检查、登录与注册、Swagger 文档、CORS 预检
    if path == "/health"
        || path == "/auth/login"
        || path == "/auth/register"
        || path.starts_with("/docs")
        || path.starts_with("/api-docs")
        || method == axum::http::Method::OPTIONS {
        return Ok(next.run(req).await);
    }

    // 读取 Authorization 头；如缺失则回退从 Cookie 中解析 auth_token
    let token = {
        let authz = req
            .headers()
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok());

        if let Some(h) = authz {
            let prefix = "Bearer ";
            if !h.starts_with(prefix) {
                tracing::warn!(path = %path, authz = %h, "invalid Authorization format (expect Bearer)");
                return Err(StatusCode::UNAUTHORIZED);
            }
            h[prefix.len()..].to_string()
        } else {
            // Cookie 回退：解析 Cookie 头获取 auth_token
            let cookie_header = req
                .headers()
                .get(axum::http::header::COOKIE)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");

            let mut token_val: Option<String> = None;
            for part in cookie_header.split(';') {
                let kv = part.trim();
                if let Some(rest) = kv.strip_prefix("auth_token=") {
                    token_val = Some(rest.to_string());
                    break;
                }
            }

            match token_val {
                Some(t) if !t.is_empty() => t,
                _ => {
                    tracing::warn!(path = %path, "missing Authorization header and auth_token cookie");
                    return Err(StatusCode::BAD_REQUEST);
                }
            }
        }
    };
    let key = DecodingKey::from_secret(state.auth.jwt_secret.as_bytes());
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;

    match decode::<Claims>(&token, &key, &validation) {
        Ok(_data) => {
            // 可按需将 claims 注入 request 扩展供后续使用
            Ok(next.run(req).await)
        }
        Err(e) => {
            tracing::error!(path = %path, err = %e, "token validation failed");
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}