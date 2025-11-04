use axum::{Router};
use axum::http::{Request, StatusCode};
use axum::body::Body;
use service::file::admin_kv_store::ApiKeysStore;
use service::file::api_management::ApiStore;
use service::admin::{kv_store::AdminKvStore, api_mgmt_store::ApiManagementStore};
use service::proxy_api::{repository::SeaOrmProxyApiRepository, service::ProxyApiService};
use tower::Service;
use serde_json::json;
use uuid::Uuid;
use migration::MigratorTrait;

use server::routes::{self, auth};

fn cors() -> tower_http::cors::CorsLayer { tower_http::cors::CorsLayer::very_permissive() }

async fn build_app() -> anyhow::Result<Router> {
    let db = models::db::connect().await?;
    // Run migrations to ensure schema（重复运行可能会报唯一约束错误，忽略已应用的情况）
    if let Err(e) = migration::Migrator::up(&db, None).await {
        let msg = format!("{}", e);
        if msg.contains("duplicate key value violates unique constraint") {
            eprintln!("migrations already applied, continue: {}", msg);
        } else {
            return Err(e.into());
        }
    }
    let admin_store = ApiKeysStore::new("data/api_keys.json").await?;
    // 初始化 API 管理存储（用于 /admin/apis 管理端点）
    let api_store = ApiStore::new("data/apis.json").await?;
    // 将文件实现转换为 trait 对象供路由使用
    let admin_kv_store: std::sync::Arc<dyn AdminKvStore> = admin_store.clone();
    let api_mgmt_store: std::sync::Arc<dyn ApiManagementStore> = api_store.clone();

    // 构建 ProxyApiService（基于 SeaORM 仓库实现）
    let repo = SeaOrmProxyApiRepository { db: db.clone() };
    let proxy_api_svc = std::sync::Arc::new(ProxyApiService::new(std::sync::Arc::new(repo)));
    let state = auth::ServerState {
        db,
        auth: auth::ServerAuthConfig { jwt_secret: "test-secret".into() },
        admin_kv_store: std::sync::Arc::clone(&admin_kv_store),
        api_mgmt_store: std::sync::Arc::clone(&api_mgmt_store),
        proxy_api_svc: std::sync::Arc::clone(&proxy_api_svc),
    };
    Ok(routes::build_router(admin_store.clone(), cors(), state))
}

#[tokio::test]
async fn test_register_and_login_flow() -> anyhow::Result<()> {
    if std::env::var("SKIP_DB_TESTS").is_ok() { return Ok(()); }
    let app = build_app().await?;

    let tid = Uuid::new_v4();
    let email = format!("user_{}@example.com", Uuid::new_v4());
    let name = "Tester";
    let password = "S3curePass!";

    // Register
    let req = Request::builder()
        .method("POST")
        .uri("/auth/register")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&json!({"tenant_id": tid, "email": email, "name": name, "password": password}))?))?;
    let resp = app.clone().call(req).await?;
    eprintln!("register status={}", resp.status());
    assert_eq!(resp.status(), StatusCode::OK);

    // Login
    let req = Request::builder()
        .method("POST")
        .uri("/auth/login")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&json!({"tenant_id": tid, "email": email, "password": password}))?))?;
    let resp = app.clone().call(req).await?;
    eprintln!("login status={}", resp.status());
    assert_eq!(resp.status(), StatusCode::OK);
    // Must set cookie
    let cookie = resp.headers().get("set-cookie");
    assert!(cookie.is_some());
    Ok(())
}

#[tokio::test]
async fn test_login_wrong_password() -> anyhow::Result<()> {
    if std::env::var("SKIP_DB_TESTS").is_ok() { return Ok(()); }
    let app = build_app().await?;

    let tid = Uuid::new_v4();
    let email = format!("user_{}@example.com", Uuid::new_v4());
    let name = "Tester";

    // Register with strong pass
    let req = Request::builder().method("POST").uri("/auth/register").header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&json!({"tenant_id": tid, "email": email, "name": name, "password": "StrongPass123"}))?))?;
    let _ = app.clone().call(req).await?;
    eprintln!("register strong pass done");

    // Login with wrong pass
    let req = Request::builder().method("POST").uri("/auth/login").header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&json!({"tenant_id": tid, "email": email, "password": "wrong"}))?))?;
    let resp = app.clone().call(req).await?;
    eprintln!("login wrong pass status={}", resp.status());
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    Ok(())
}

#[tokio::test]
async fn test_register_short_password_rejected() -> anyhow::Result<()> {
    if std::env::var("SKIP_DB_TESTS").is_ok() { return Ok(()); }
    let app = build_app().await?;

    let req = Request::builder().method("POST").uri("/auth/register").header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&json!({"tenant_id": Uuid::new_v4(), "email": "a@b.com", "name": "A", "password": "short"}))?))?;
    let resp = app.clone().call(req).await?;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    Ok(())
}

#[tokio::test]
async fn test_login_performance_basic() -> anyhow::Result<()> {
    if std::env::var("SKIP_DB_TESTS").is_ok() { return Ok(()); }
    let app = build_app().await?;
    let tid = Uuid::new_v4();
    let email = format!("user_{}@example.com", Uuid::new_v4());
    let name = "Perf";
    let password = "PerfPass123!";

    // Register
    let req = Request::builder().method("POST").uri("/auth/register").header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&json!({"tenant_id": tid, "email": email, "name": name, "password": password}))?))?;
    let _ = app.clone().call(req).await?;

    // Perform multiple logins and record durations
    let attempts = 20;
    let mut durs = Vec::new();
    for _ in 0..attempts {
        let req = Request::builder().method("POST").uri("/auth/login").header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&json!({"tenant_id": tid, "email": email, "password": password}))?))?;
        let start = std::time::Instant::now();
        let resp = app.clone().call(req).await?;
        let elapsed = start.elapsed();
        assert_eq!(resp.status(), StatusCode::OK);
        durs.push(elapsed);
    }
    durs.sort();
    let p95 = durs[(attempts as f32 * 0.95) as usize - 1];
    assert!(p95.as_millis() < 500, "p95 too high: {:?}", p95);
    Ok(())
}