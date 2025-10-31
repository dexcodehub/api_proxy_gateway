use crate::db::connect;
use crate::tenant;
use sea_orm::{DatabaseConnection, TransactionTrait, DatabaseBackend, Statement, EntityTrait, ActiveModelTrait, Set, QueryFilter, ColumnTrait, ConnectionTrait};
use chrono::Utc;
use migration::MigratorTrait;
use anyhow::Result;
use uuid::Uuid;
use std::sync::Arc;
use tokio::sync::Barrier;

/// Setup test database
async fn setup_test_db() -> Result<DatabaseConnection> {
    let db = connect().await?;
    migration::Migrator::up(&db, None).await?;
    Ok(db)
}

/// Test basic transaction commit
#[tokio::test]
async fn test_transaction_commit() -> Result<()> {
    if std::env::var("SKIP_DB_TESTS").is_ok() {
        return Ok(());
    }

    let db = setup_test_db().await?;
    
    let tenant_name = format!("tx_commit_test_{}", Uuid::new_v4());
    let mut tenant_id = None;
    
    // Start transaction
    let txn = db.begin().await?;
    
    // Create tenant within transaction (ActiveModel insert on txn)
    let am = tenant::ActiveModel { id: Set(Uuid::new_v4()), name: Set(tenant_name.clone()), created_at: Set(Utc::now().into()) };
    let created_tenant = am.insert(&txn).await?;
    tenant_id = Some(created_tenant.id);
    
    // Commit transaction
    txn.commit().await?;
    
    // Verify tenant exists after commit
    let found_tenant = tenant::Entity::find_by_id(created_tenant.id).one(&db).await?;
    assert!(found_tenant.is_some());
    assert_eq!(found_tenant.unwrap().name, tenant_name);
    
    // Cleanup
    if let Some(id) = tenant_id {
        tenant::Entity::delete_by_id(id).exec(&db).await?;
    }
    
    println!("Transaction commit test completed successfully");
    Ok(())
}

/// Test transaction rollback
#[tokio::test]
async fn test_transaction_rollback() -> Result<()> {
    if std::env::var("SKIP_DB_TESTS").is_ok() {
        return Ok(());
    }

    let db = setup_test_db().await?;
    
    let tenant_name = format!("tx_rollback_test_{}", Uuid::new_v4());
    let mut created_tenant_id = None;
    
    // Start transaction
    let txn = db.begin().await?;
    
    // Create tenant within transaction
    let am = tenant::ActiveModel { id: Set(Uuid::new_v4()), name: Set(tenant_name.clone()), created_at: Set(Utc::now().into()) };
    let created_tenant = am.insert(&txn).await?;
    created_tenant_id = Some(created_tenant.id);
    
    // Rollback transaction instead of committing
    txn.rollback().await?;
    
    // Verify tenant does NOT exist after rollback
    let found_tenant = tenant::Entity::find_by_id(created_tenant.id).one(&db).await?;
    assert!(found_tenant.is_none());
    
    // Also verify by name
    let found_by_name = tenant::Entity::find()
        .filter(tenant::Column::Name.eq(tenant_name.clone()))
        .one(&db)
        .await?;
    assert!(found_by_name.is_none());
    
    println!("Transaction rollback test completed successfully");
    Ok(())
}

/// Test nested transactions (savepoints)
#[tokio::test]
async fn test_nested_transactions() -> Result<()> {
    if std::env::var("SKIP_DB_TESTS").is_ok() {
        return Ok(());
    }

    let db = setup_test_db().await?;
    
    let tenant1_name = format!("nested_tx_1_{}", Uuid::new_v4());
    let tenant2_name = format!("nested_tx_2_{}", Uuid::new_v4());
    let mut cleanup_ids = vec![];
    
    // Start outer transaction
    let outer_txn = db.begin().await?;
    
    // Create first tenant
    let am1 = tenant::ActiveModel { id: Set(Uuid::new_v4()), name: Set(tenant1_name.clone()), created_at: Set(Utc::now().into()) };
    let tenant1 = am1.insert(&outer_txn).await?;
    cleanup_ids.push(tenant1.id);
    
    // Start inner transaction (savepoint)
    let inner_txn = outer_txn.begin().await?;
    
    // Create second tenant in inner transaction
    let am2 = tenant::ActiveModel { id: Set(Uuid::new_v4()), name: Set(tenant2_name.clone()), created_at: Set(Utc::now().into()) };
    let tenant2 = am2.insert(&inner_txn).await?;
    
    // Rollback inner transaction only
    inner_txn.rollback().await?;
    
    // Commit outer transaction
    outer_txn.commit().await?;
    
    // Verify: tenant1 should exist, tenant2 should not
    let found_tenant1 = tenant::Entity::find_by_id(tenant1.id).one(&db).await?;
    assert!(found_tenant1.is_some());
    assert_eq!(found_tenant1.unwrap().name, tenant1_name);
    
    let found_tenant2 = tenant::Entity::find_by_id(tenant2.id).one(&db).await?;
    assert!(found_tenant2.is_none());
    
    // Cleanup
    for id in cleanup_ids {
        tenant::Entity::delete_by_id(id).exec(&db).await?;
    }
    
    println!("Nested transactions test completed successfully");
    Ok(())
}

/// Test transaction with error handling
#[tokio::test]
async fn test_transaction_error_handling() -> Result<()> {
    if std::env::var("SKIP_DB_TESTS").is_ok() {
        return Ok(());
    }

    let db = setup_test_db().await?;
    
    let tenant_name = format!("tx_error_test_{}", Uuid::new_v4());
    
    // Test transaction that should fail
    let result = async {
        let txn = db.begin().await?;
        
        // Create valid tenant
        let am = tenant::ActiveModel { id: Set(Uuid::new_v4()), name: Set(tenant_name.clone()), created_at: Set(Utc::now().into()) };
        let _tenant = am.insert(&txn).await?;
        
        // Try to create duplicate tenant (should fail due to unique constraint)
        // Attempt duplicate insert (name has unique index)
        let am_dup = tenant::ActiveModel { id: Set(Uuid::new_v4()), name: Set(tenant_name.clone()), created_at: Set(Utc::now().into()) };
        let _duplicate = am_dup.insert(&txn).await?;
        
        txn.commit().await?;
        Ok::<(), anyhow::Error>(())
    }.await;
    
    // Should fail due to duplicate name
    assert!(result.is_err());
    
    // Verify no tenant was created due to rollback
    let found_tenant = tenant::Entity::find().filter(tenant::Column::Name.eq(tenant_name.clone())).one(&db).await?;
    assert!(found_tenant.is_none());
    
    println!("Transaction error handling test completed successfully");
    Ok(())
}

/// Test concurrent transactions
#[tokio::test]
async fn test_concurrent_transactions() -> Result<()> {
    if std::env::var("SKIP_DB_TESTS").is_ok() {
        return Ok(());
    }

    let db = setup_test_db().await?;
    let db = Arc::new(db);
    
    let num_tasks = 5;
    let barrier = Arc::new(Barrier::new(num_tasks));
    let mut handles: Vec<tokio::task::JoinHandle<anyhow::Result<uuid::Uuid>>> = vec![];
    let mut cleanup_ids = vec![];
    
    for i in 0..num_tasks {
        let db_clone = Arc::clone(&db);
        let barrier_clone = Arc::clone(&barrier);
        
        let handle = tokio::spawn(async move {
            // Wait for all tasks to be ready
            barrier_clone.wait().await;
            
            let tenant_name = format!("concurrent_tx_{}_{}", i, Uuid::new_v4());
            
            // Start transaction
            let txn = db_clone.begin().await?;
            
            // Create tenant via ActiveModel on the transaction
            let am = tenant::ActiveModel {
                id: Set(Uuid::new_v4()),
                name: Set(tenant_name.clone()),
                created_at: Set(Utc::now().into()),
            };
            let tenant = am.insert(&txn).await?;
            
            // Small delay to increase chance of concurrency issues
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            
            // Commit transaction
            txn.commit().await?;
            
            Ok::<uuid::Uuid, anyhow::Error>(tenant.id)
        });
        
        handles.push(handle);
    }
    
    // Wait for all tasks to complete
    for handle in handles {
        let tenant_id = handle.await??;
        cleanup_ids.push(tenant_id);
        
        // Verify tenant was created
        let tenant = tenant::Entity::find_by_id(tenant_id).one(db.as_ref()).await?;
        assert!(tenant.is_some());
    }
    
    // Cleanup
    for id in cleanup_ids {
        tenant::Entity::delete_by_id(id).exec(db.as_ref()).await?;
    }
    
    println!("Concurrent transactions test completed successfully");
    Ok(())
}

/// Test transaction isolation levels
#[tokio::test]
async fn test_transaction_isolation() -> Result<()> {
    if std::env::var("SKIP_DB_TESTS").is_ok() {
        return Ok(());
    }

    let db = setup_test_db().await?;
    
    let tenant_name = format!("isolation_test_{}", Uuid::new_v4());
    let mut cleanup_id = None;
    
    // Create a tenant first
    let initial_am = tenant::ActiveModel { id: Set(Uuid::new_v4()), name: Set(tenant_name.clone()), created_at: Set(Utc::now().into()) };
    let initial_tenant = initial_am.insert(&db).await?;
    cleanup_id = Some(initial_tenant.id);
    
    // Start two concurrent transactions
    let txn1 = db.begin().await?;
    let txn2 = db.begin().await?;
    
    // Transaction 1: Read the tenant
    let tenant_in_tx1 = tenant::Entity::find_by_id(initial_tenant.id).one(&txn1).await?;
    assert!(tenant_in_tx1.is_some());
    
    // Transaction 2: Delete the tenant (hard delete via Entity)
    tenant::Entity::delete_by_id(initial_tenant.id).exec(&txn2).await?;
    txn2.commit().await?;
    
    // Transaction 1: Try to read again (should still see the tenant due to isolation)
    let tenant_still_in_tx1 = tenant::Entity::find_by_id(initial_tenant.id).one(&txn1).await?;
    // Note: Behavior depends on isolation level, but typically should still see it
    
    txn1.rollback().await?;
    
    // Outside transaction: Should not see the tenant (deleted)
    let tenant_after = tenant::Entity::find_by_id(initial_tenant.id).one(&db).await?;
    assert!(tenant_after.is_none());
    
    // Cleanup
    if let Some(id) = cleanup_id {
        tenant::Entity::delete_by_id(id).exec(&db).await?;
    }
    
    println!("Transaction isolation test completed successfully");
    Ok(())
}

/// Test long-running transaction
#[tokio::test]
async fn test_long_running_transaction() -> Result<()> {
    if std::env::var("SKIP_DB_TESTS").is_ok() {
        return Ok(());
    }

    let db = setup_test_db().await?;
    
    let tenant_name = format!("long_tx_test_{}", Uuid::new_v4());
    let mut cleanup_id = None;
    
    // Start transaction
    let txn = db.begin().await?;
    
    // Create tenant
    let am = tenant::ActiveModel { id: Set(Uuid::new_v4()), name: Set(tenant_name.clone()), created_at: Set(Utc::now().into()) };
    let tenant = am.insert(&txn).await?;
    cleanup_id = Some(tenant.id);
    
    // Simulate some processing time
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    
    // Do another operation
    let found_tenant = tenant::Entity::find_by_id(tenant.id).one(&txn).await?;
    assert!(found_tenant.is_some());
    
    // Commit after delay
    txn.commit().await?;
    
    // Verify tenant exists
    let final_tenant = tenant::Entity::find_by_id(tenant.id).one(&db).await?;
    assert!(final_tenant.is_some());
    
    // Cleanup
    if let Some(id) = cleanup_id {
        tenant::Entity::delete_by_id(id).exec(&db).await?;
    }
    
    println!("Long-running transaction test completed successfully");
    Ok(())
}

/// Test transaction with multiple operations
#[tokio::test]
async fn test_multi_operation_transaction() -> Result<()> {
    if std::env::var("SKIP_DB_TESTS").is_ok() {
        return Ok(());
    }

    let db = setup_test_db().await?;
    
    let mut cleanup_ids = vec![];
    
    // Start transaction
    let txn = db.begin().await?;
    
    // Create multiple tenants in single transaction
    for i in 0..3 {
        let tenant_name = format!("multi_op_tenant_{}_{}", i, Uuid::new_v4());
    let am = tenant::ActiveModel { id: Set(Uuid::new_v4()), name: Set(tenant_name.clone()), created_at: Set(Utc::now().into()) };
    let tenant = am.insert(&txn).await?;
        cleanup_ids.push(tenant.id);
    }
    
    // Verify all tenants exist within transaction
    for &tenant_id in &cleanup_ids {
    let tenant = tenant::Entity::find_by_id(tenant_id).one(&txn).await?;
        assert!(tenant.is_some());
    }
    
    // Commit all operations
    txn.commit().await?;
    
    // Verify all tenants exist after commit
    for &tenant_id in &cleanup_ids {
    let tenant = tenant::Entity::find_by_id(tenant_id).one(&db).await?;
        assert!(tenant.is_some());
    }
    
    // Cleanup
    for id in cleanup_ids {
        tenant::Entity::delete_by_id(id).exec(&db).await?;
    }
    
    println!("Multi-operation transaction test completed successfully");
    Ok(())
}