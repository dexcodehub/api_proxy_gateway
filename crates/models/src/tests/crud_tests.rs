use crate::db::connect;
use crate::{tenant, user, apikey, upstream, ratelimit, route, request_log};
use sea_orm::{DatabaseConnection, DatabaseBackend, Statement, EntityTrait, ActiveModelTrait, Set, QueryFilter, ColumnTrait, ConnectionTrait};
use anyhow::Result;
use migration::MigratorTrait;
use uuid::Uuid;
use std::time::Instant;

/// Setup test database with migrations
async fn setup_test_db() -> Result<DatabaseConnection> {
    let db = connect().await?;
    
    // Run migrations if needed
    migration::Migrator::up(&db, None).await?;
    
    Ok(db)
}

/// Test tenant CRUD operations
#[tokio::test]
async fn test_tenant_crud() -> Result<()> {
    if std::env::var("SKIP_DB_TESTS").is_ok() {
        return Ok(());
    }

    let db = setup_test_db().await?;
    
    // Test Create
    let tenant_name = format!("test_tenant_{}", Uuid::new_v4());
    let created_tenant = tenant::create(&db, &tenant_name).await?;
    
    assert_eq!(created_tenant.name, tenant_name);
    
    println!("Created tenant: {:?}", created_tenant);
    
    // Test Read
    let found_tenant = tenant::Entity::find_by_id(created_tenant.id).one(&db).await?;
    assert!(found_tenant.is_some());
    let found_tenant = found_tenant.unwrap();
    assert_eq!(found_tenant.id, created_tenant.id);
    assert_eq!(found_tenant.name, tenant_name);
    
    // Test find by name
    let found_by_name = tenant::Entity::find().filter(tenant::Column::Name.eq(tenant_name.clone())).one(&db).await?;
    assert!(found_by_name.is_some());
    assert_eq!(found_by_name.unwrap().id, created_tenant.id);
    
    // Test Update (if update method exists)
    // Note: Current implementation doesn't have update, but we can test soft delete
    
    // Test Soft Delete
    // No soft-delete column on tenant; skip soft delete
    
    // Verify soft delete
    // Verify still exists prior to hard delete
    let existing_tenant = tenant::Entity::find_by_id(created_tenant.id).one(&db).await?;
    assert!(existing_tenant.is_some());
    
    // Test Hard Delete
    tenant::Entity::delete_by_id(created_tenant.id).exec(&db).await?;
    
    println!("Tenant CRUD test completed successfully");
    Ok(())
}

/// Test user CRUD operations
#[tokio::test]
async fn test_user_crud() -> Result<()> {
    if std::env::var("SKIP_DB_TESTS").is_ok() {
        return Ok(());
    }

    let db = setup_test_db().await?;
    
    // Create a tenant first
    let tenant_name = format!("user_test_tenant_{}", Uuid::new_v4());
    let test_tenant = tenant::create(&db, &tenant_name).await?;
    
    // Test Create User
    let user_email = format!("test_{}@example.com", Uuid::new_v4());
    let user_name = format!("Test User {}", Uuid::new_v4());
    
    let created_user = user::create(&db, test_tenant.id, &user_email, &user_name).await?;
    
    assert_eq!(created_user.email, user_email);
    assert_eq!(created_user.name, user_name);
    assert_eq!(created_user.tenant_id, test_tenant.id);
    assert_eq!(created_user.tenant_id, test_tenant.id);
    
    println!("Created user: {:?}", created_user);
    
    // Test Read
    let found_user = user::Entity::find_by_id(created_user.id).one(&db).await?;
    assert!(found_user.is_some());
    let found_user = found_user.unwrap();
    assert_eq!(found_user.id, created_user.id);
    assert_eq!(found_user.email, user_email);
    
    // Test find by email
    let found_by_email = user::Entity::find().filter(user::Column::Email.eq(user_email.clone())).one(&db).await?;
    assert!(found_by_email.is_some());
    assert_eq!(found_by_email.unwrap().id, created_user.id);
    
    // Test Soft Delete
    user::soft_delete(&db, created_user.id).await?;
    
    // Verify soft delete
    let deleted_user = user::Entity::find_by_id(created_user.id).one(&db).await?;
    assert!(deleted_user.is_none());
    
    // Test Hard Delete
    user::hard_delete(&db, created_user.id).await?;
    
    // Cleanup
    tenant::Entity::delete_by_id(test_tenant.id).exec(&db).await?;
    
    println!("User CRUD test completed successfully");
    Ok(())
}

/// Test API key CRUD operations
#[tokio::test]
async fn test_apikey_crud() -> Result<()> {
    if std::env::var("SKIP_DB_TESTS").is_ok() {
        return Ok(());
    }

    let db = setup_test_db().await?;
    
    // Setup prerequisites
    let tenant_name = format!("apikey_test_tenant_{}", Uuid::new_v4());
    let test_tenant = tenant::create(&db, &tenant_name).await?;
    
    let user_email = format!("apikey_test_{}@example.com", Uuid::new_v4());
    let user_name = format!("API Test User {}", Uuid::new_v4());
    let test_user = user::create(&db, test_tenant.id, &user_email, &user_name).await?;
    
    // Test Create API Key
    let key_hash = "a".repeat(64); // 64 character hash
    let created_apikey = apikey::create(&db, test_user.id, &key_hash).await?;
    
    assert_eq!(created_apikey.key_hash, key_hash);
    assert_eq!(created_apikey.user_id, test_user.id);
    assert!(created_apikey.key_hash.len() >= 12);
    
    println!("Created API key: {:?}", created_apikey);
    
    // Test Read
    let found_apikey = apikey::Entity::find_by_id(created_apikey.id).one(&db).await?;
    assert!(found_apikey.is_some());
    let found_apikey = found_apikey.unwrap();
    assert_eq!(found_apikey.id, created_apikey.id);
    assert_eq!(found_apikey.key_hash, key_hash);
    
    // Test find by hash
    let found_by_hash = apikey::Entity::find().filter(apikey::Column::KeyHash.eq(key_hash.clone())).one(&db).await?;
    assert!(found_by_hash.is_some());
    assert_eq!(found_by_hash.unwrap().id, created_apikey.id);
    
    // Test Soft Delete
    // Delete via Entity since no soft-delete implementation
    apikey::Entity::delete_by_id(created_apikey.id).exec(&db).await?;
    
    // Cleanup
    user::hard_delete(&db, test_user.id).await?;
    tenant::Entity::delete_by_id(test_tenant.id).exec(&db).await?;
    
    println!("API key CRUD test completed successfully");
    Ok(())
}

/// Test upstream CRUD operations
#[tokio::test]
async fn test_upstream_crud() -> Result<()> {
    if std::env::var("SKIP_DB_TESTS").is_ok() {
        return Ok(());
    }

    let db = setup_test_db().await?;
    
    // Test Create Upstream
    let up_name = format!("upstream_{}", Uuid::new_v4());
    let base_url = "https://api.example.com";
    let created_upstream = upstream::create(&db, &up_name, base_url).await?;
    
    assert_eq!(created_upstream.name, up_name);
    assert_eq!(created_upstream.base_url, base_url);
    assert!(created_upstream.health_url.is_none());
    
    println!("Created upstream: {:?}", created_upstream);
    
    // Test Read
    let found_upstream = upstream::Entity::find_by_id(created_upstream.id).one(&db).await?;
    assert!(found_upstream.is_some());
    let found_upstream = found_upstream.unwrap();
    assert_eq!(found_upstream.id, created_upstream.id);
    assert_eq!(found_upstream.base_url, base_url);
    
    // Test Soft Delete
    // Delete via Entity (no soft-delete/hard-delete helpers)
    upstream::Entity::delete_by_id(created_upstream.id).exec(&db).await?;
    
    println!("Upstream CRUD test completed successfully");
    Ok(())
}

/// Test rate limit CRUD operations
#[tokio::test]
async fn test_ratelimit_crud() -> Result<()> {
    if std::env::var("SKIP_DB_TESTS").is_ok() {
        return Ok(());
    }

    let db = setup_test_db().await?;
    
    // Setup prerequisites (optional tenant)
    let tenant_name = format!("ratelimit_test_tenant_{}", Uuid::new_v4());
    let test_tenant = tenant::create(&db, &tenant_name).await?;
    
    // Test Create Rate Limit
    let requests_per_minute = 100;
    let burst = 10;
    
    let rl = ratelimit::ActiveModel {
        id: Set(Uuid::new_v4()),
        tenant_id: Set(Some(test_tenant.id)),
        requests_per_minute: Set(requests_per_minute),
        burst: Set(burst),
        created_at: Set(chrono::Utc::now().into()),
    };
    let created_ratelimit = rl.insert(&db).await?;
    
    assert_eq!(created_ratelimit.requests_per_minute, requests_per_minute);
    assert_eq!(created_ratelimit.burst, burst);
    assert_eq!(created_ratelimit.tenant_id, Some(test_tenant.id));
    
    println!("Created rate limit: {:?}", created_ratelimit);
    
    // Test Read
    let found_ratelimit = ratelimit::Entity::find_by_id(created_ratelimit.id).one(&db).await?;
    assert!(found_ratelimit.is_some());
    let found_ratelimit = found_ratelimit.unwrap();
    assert_eq!(found_ratelimit.id, created_ratelimit.id);
    assert_eq!(found_ratelimit.requests_per_minute, requests_per_minute);
    
    // Test Soft Delete
    // Delete via Entity (no soft-delete/hard-delete helpers)
    ratelimit::Entity::delete_by_id(created_ratelimit.id).exec(&db).await?;
    
    // Cleanup
    tenant::Entity::delete_by_id(test_tenant.id).exec(&db).await?;
    
    println!("Rate limit CRUD test completed successfully");
    Ok(())
}

/// Test performance of CRUD operations
#[tokio::test]
async fn test_crud_performance() -> Result<()> {
    if std::env::var("SKIP_DB_TESTS").is_ok() {
        return Ok(());
    }

    let db = setup_test_db().await?;
    
    // Test tenant creation performance
    let iterations = 10;
    let start = Instant::now();
    
    let mut tenant_ids = vec![];
    
    for i in 0..iterations {
        let tenant_name = format!("perf_test_tenant_{}", i);
        let tenant = tenant::create(&db, &tenant_name).await?;
        tenant_ids.push(tenant.id);
    }
    
    let creation_time = start.elapsed();
    println!("Created {} tenants in {:?}", iterations, creation_time);
    
    // Test read performance
    let start = Instant::now();
    for &tenant_id in &tenant_ids {
        let _tenant = tenant::Entity::find_by_id(tenant_id).one(&db).await?;
    }
    let read_time = start.elapsed();
    println!("Read {} tenants in {:?}", iterations, read_time);
    
    // Test deletion performance
    let start = Instant::now();
    for &tenant_id in &tenant_ids {
        tenant::Entity::delete_by_id(tenant_id).exec(&db).await?;
    }
    let deletion_time = start.elapsed();
    println!("Deleted {} tenants in {:?}", iterations, deletion_time);
    
    // Performance assertions
    let avg_creation = creation_time / iterations as u32;
    let avg_read = read_time / iterations as u32;
    let avg_deletion = deletion_time / iterations as u32;
    
    println!("Average times - Create: {:?}, Read: {:?}, Delete: {:?}", 
             avg_creation, avg_read, avg_deletion);
    
    // Each operation should be reasonably fast
    assert!(avg_creation < std::time::Duration::from_millis(500));
    assert!(avg_read < std::time::Duration::from_millis(100));
    assert!(avg_deletion < std::time::Duration::from_millis(200));
    
    Ok(())
}

/// Test batch operations
#[tokio::test]
async fn test_batch_operations() -> Result<()> {
    if std::env::var("SKIP_DB_TESTS").is_ok() {
        return Ok(());
    }

    let db = setup_test_db().await?;
    
    // Create multiple tenants in sequence
    let batch_size = 5;
    let mut tenant_ids = vec![];
    
    for i in 0..batch_size {
        let tenant_name = format!("batch_test_tenant_{}", i);
        let tenant = tenant::create(&db, &tenant_name).await?;
        tenant_ids.push(tenant.id);
    }
    
    // Verify all were created
    for &tenant_id in &tenant_ids {
        let tenant = tenant::Entity::find_by_id(tenant_id).one(&db).await?;
        assert!(tenant.is_some());
    }
    
    // Clean up
    for &tenant_id in &tenant_ids {
        tenant::Entity::delete_by_id(tenant_id).exec(&db).await?;
    }
    
    println!("Batch operations test completed successfully");
    Ok(())
}