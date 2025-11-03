use anyhow::Result;
use migration::{Migrator, MigratorTrait};
use sea_orm::EntityTrait;
use uuid::Uuid;

use crate::{db::connect, tenant, proxy_api};

#[tokio::test]
async fn test_create_and_toggle_proxy_api() -> Result<()> {
    if std::env::var("SKIP_DB_TESTS").is_ok() { return Ok(()); }
    let db = connect().await?;
    Migrator::up(&db, None).await?;

    let t = tenant::create(&db, &format!("tenant_{}", Uuid::new_v4())).await?;
    let pa = proxy_api::create(&db, t.id, "/proxy/posts", "GET", "https://jsonplaceholder.typicode.com/posts", false).await?;

    let found = proxy_api::Entity::find_by_id(pa.id).one(&db).await?;
    assert!(found.is_some());
    let found = found.unwrap();
    assert_eq!(found.tenant_id, t.id);
    assert_eq!(found.method, "GET");
    assert!(found.enabled);

    proxy_api::set_enabled(&db, pa.id, false).await?;
    let found = proxy_api::Entity::find_by_id(pa.id).one(&db).await?;
    assert!(!found.unwrap().enabled);

    Ok(())
}

#[tokio::test]
async fn test_unique_per_tenant() -> Result<()> {
    if std::env::var("SKIP_DB_TESTS").is_ok() { return Ok(()); }
    let db = connect().await?;
    Migrator::up(&db, None).await?;

    let t = tenant::create(&db, &format!("tenant_u_{}", Uuid::new_v4())).await?;
    let _a1 = proxy_api::create(&db, t.id, "/proxy/posts", "GET", "https://jsonplaceholder.typicode.com/posts", true).await?;
    // same method + endpoint for same tenant should violate unique index
    let dup = proxy_api::create(&db, t.id, "/proxy/posts", "GET", "https://jsonplaceholder.typicode.com/posts", false).await;
    assert!(dup.is_err());
    Ok(())
}