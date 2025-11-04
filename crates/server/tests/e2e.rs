use std::net::SocketAddr;
use std::sync::Arc;

use axum::{Router};
use service::file::{admin_kv_store::ApiKeysStore, api_management::ApiStore};
use tower_http::cors::CorsLayer;
use tokio::net::TcpListener;
use serde_json::json;
use uuid::Uuid;
use reqwest::StatusCode as HttpStatusCode;
use migration::MigratorTrait;

use server::routes::{self, auth};

// Optional: use testcontainers for Postgres; skip if unavailable
#[allow(unused_imports)]

fn cors() -> CorsLayer { CorsLayer::very_permissive() }

struct TestApp {
    base_url: String,
}

async fn start_server() -> anyhow::Result<TestApp> {
    // Ensure models prefer env over config file
    std::env::set_var("CONFIG_PATH", "/nonexistent-config-for-tests.toml");

    // Use DATABASE_URL from environment; if not present, skip tests gracefully
    if std::env::var("DATABASE_URL").is_err() {
        eprintln!("DATABASE_URL missing; skip e2e tests. Provide .env.test or env var.");
        // Create a tiny server instance for public route check without DB
        // but since most routes need DB, we return early
        return Err(anyhow::anyhow!("missing DATABASE_URL"));
    }

    // Connect DB and run migrations
    let db = models::db::connect().await?;
    if let Err(e) = migration::Migrator::up(&db, None).await { eprintln!("migrations notice: {}", e); }

    // Use isolated temp files for admin/api stores per test run
    let temp_id = Uuid::new_v4();
    let api_keys_path = format!("target/test-data/{}/api_keys.json", temp_id);
    let apis_path = format!("target/test-data/{}/apis.json", temp_id);
    let admin_store = ApiKeysStore::new(&api_keys_path).await?;
    let api_store = ApiStore::new(&apis_path).await?;

    let state = auth::ServerState {
        db,
        auth: auth::ServerAuthConfig { jwt_secret: "test-secret".into() },
        admin_store,
        api_store: Arc::clone(&api_store),
    };

    let app: Router = routes::build_router(state.admin_store.clone(), cors(), state);
    let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).await?;
    let addr: SocketAddr = listener.local_addr()?;
    let base_url = format!("http://{}:{}", addr.ip(), addr.port());

    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await { eprintln!("server error: {}", e); }
    });

    Ok(TestApp { base_url })
}

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .cookie_store(true)
        .build()
        .expect("reqwest client")
}

#[tokio::test]
async fn e2e_public_health() -> anyhow::Result<()> {
    if std::env::var("SKIP_DB_TESTS").is_ok() { return Ok(()); }
    let app = match start_server().await {
        Ok(a) => a,
        Err(_) => return Ok(()),
    };
    let res = client().get(format!("{}/health", app.base_url)).send().await?;
    assert_eq!(res.status(), HttpStatusCode::OK);
    let body = res.json::<serde_json::Value>().await?;
    assert_eq!(body["status"], "ok");
    Ok(())
}

#[tokio::test]
async fn e2e_auth_register_login_and_cookie() -> anyhow::Result<()> {
    if std::env::var("SKIP_DB_TESTS").is_ok() { return Ok(()); }
    let app = match start_server().await {
        Ok(a) => a,
        Err(_) => return Ok(()),
    };
    let c = client();

    let tid = Uuid::new_v4();
    let email = format!("user_{}@example.com", Uuid::new_v4());
    let name = "Tester";
    let password = "S3curePass!";

    // Register
    let res = c.post(format!("{}/auth/register", app.base_url))
        .json(&json!({"tenant_id": tid, "email": email, "name": name, "password": password}))
        .send().await?;
    assert_eq!(res.status(), HttpStatusCode::OK);

    // Login -> set-cookie
    let res = c.post(format!("{}/auth/login", app.base_url))
        .json(&json!({"tenant_id": tid, "email": email, "password": password}))
        .send().await?;
    assert_eq!(res.status(), HttpStatusCode::OK);
    let set_cookie = res.headers().get("set-cookie");
    assert!(set_cookie.is_some());
    Ok(())
}

#[tokio::test]
async fn e2e_protected_without_token_denied() -> anyhow::Result<()> {
    if std::env::var("SKIP_DB_TESTS").is_ok() { return Ok(()); }
    let app = match start_server().await {
        Ok(a) => a,
        Err(_) => return Ok(()),
    };
    let c = reqwest::Client::new();
    let res = c.get(format!("{}/admin/apis", app.base_url)).send().await?;
    // Global middleware: missing Authorization and auth_token cookie -> 400
    assert_eq!(res.status(), HttpStatusCode::BAD_REQUEST);
    Ok(())
}

#[tokio::test]
async fn e2e_protected_with_expired_token_unauthorized() -> anyhow::Result<()> {
    if std::env::var("SKIP_DB_TESTS").is_ok() { return Ok(()); }
    let app = match start_server().await {
        Ok(a) => a,
        Err(_) => return Ok(()),
    };
    let c = reqwest::Client::new();

    // Create an expired JWT token signed with test-secret
    use jsonwebtoken::{encode, EncodingKey, Header};
    #[derive(serde::Serialize)]
    struct Claims { sub: String, exp: usize, iat: usize }
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_secs() as usize;
    let claims = Claims { sub: "u".into(), exp: now.saturating_sub(60), iat: now.saturating_sub(120) };
    let token = encode(&Header::default(), &claims, &EncodingKey::from_secret("test-secret".as_bytes()))?;

    let res = c.get(format!("{}/admin/apis", app.base_url))
        .header("Authorization", format!("Bearer {}", token))
        .send().await?;
    assert_eq!(res.status(), HttpStatusCode::UNAUTHORIZED);
    Ok(())
}

#[tokio::test]
async fn e2e_admin_api_key_and_access_api_posts() -> anyhow::Result<()> {
    if std::env::var("SKIP_DB_TESTS").is_ok() { return Ok(()); }
    if std::env::var("SKIP_EXTERNAL").is_ok() { return Ok(()); }
    let app = match start_server().await {
        Ok(a) => a,
        Err(_) => return Ok(()),
    };
    let c = client();

    // Login so we have JWT cookie for admin endpoints
    let tid = Uuid::new_v4();
    let email = format!("user_{}@example.com", Uuid::new_v4());
    let password = "StrongPass123";
    let name = "AdminUser";

    let _ = c.post(format!("{}/auth/register", app.base_url))
        .json(&json!({"tenant_id": tid, "email": email, "name": name, "password": password}))
        .send().await?;
    let _ = c.post(format!("{}/auth/login", app.base_url))
        .json(&json!({"tenant_id": tid, "email": email, "password": password}))
        .send().await?;

    // Set an API key via admin endpoint
    let res = c.post(format!("{}/admin/api-keys", app.base_url))
        .json(&json!({"user": "svc-user", "api_key": "k-123"}))
        .send().await?;
    assert_eq!(res.status(), HttpStatusCode::OK);

    // Access protected API with both cookie (JWT) and X-API-Key header
    let res = c.get(format!("{}/api/posts/1", app.base_url))
        .header("X-API-Key", "k-123")
        .send().await?;
    assert_eq!(res.status(), HttpStatusCode::OK);
    let body = res.json::<serde_json::Value>().await?;
    assert_eq!(body["id"], 1);
    Ok(())
}