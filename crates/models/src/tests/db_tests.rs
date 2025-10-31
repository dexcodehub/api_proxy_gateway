use crate::db::{connect, connect_with_config, test_connection, get_pool_stats, DatabaseConfig};
use sea_orm::{DatabaseConnection, DatabaseBackend, Statement, ConnectionTrait};
use std::time::{Duration, Instant};
use tokio::time::timeout;
use anyhow::Result;

/// Test basic database connection
#[tokio::test]
async fn test_basic_connection() -> Result<()> {
    // Skip test if no database available
    if std::env::var("SKIP_DB_TESTS").is_ok() {
        println!("Skipping database tests (SKIP_DB_TESTS is set)");
        return Ok(());
    }

    let start = Instant::now();
    let db = connect().await?;
    let connection_time = start.elapsed();
    
    println!("Database connection established in {:?}", connection_time);
    
    // Verify connection is working with a simple query
    let stmt = Statement::from_string(DatabaseBackend::Postgres, "SELECT 1 as test".to_string());
    let result = db.query_one(stmt).await?;
    
    assert!(result.is_some());
    let row = result.unwrap();
    let test_value: i32 = row.try_get("", "test")?;
    assert_eq!(test_value, 1);
    
    // Connection time should be reasonable (less than 5 seconds)
    assert!(connection_time < Duration::from_secs(5), 
           "Connection took too long: {:?}", connection_time);
    
    Ok(())
}

/// Test connection with custom configuration
#[tokio::test]
async fn test_custom_config_connection() -> Result<()> {
    if std::env::var("SKIP_DB_TESTS").is_ok() {
        return Ok(());
    }

    let mut config = DatabaseConfig::default();
    // Ensure URL is set when using custom config
    config.url = crate::db::DATABASE_URL.clone();
    config.max_connections = 5;
    config.min_connections = 1;
    config.connect_timeout = Duration::from_secs(10);
    
    let db = connect_with_config(&config).await?;
    
    // Test that connection works
    let stmt = Statement::from_string(DatabaseBackend::Postgres, "SELECT current_database()".to_string());
    let result = db.query_one(stmt).await?;
    assert!(result.is_some());
    
    Ok(())
}

/// Test connection pool functionality
#[tokio::test]
async fn test_connection_pool() -> Result<()> {
    if std::env::var("SKIP_DB_TESTS").is_ok() {
        return Ok(());
    }

    let mut config = DatabaseConfig::default();
    config.url = crate::db::DATABASE_URL.clone();
    config.max_connections = 3;
    config.min_connections = 1;
    
    let db = connect_with_config(&config).await?;
    
    // Test multiple concurrent connections
    let mut handles: Vec<tokio::task::JoinHandle<Result<i32, sea_orm::DbErr>>> = vec![];
    
    for i in 0..5 {
        let db_clone = db.clone();
        let handle = tokio::spawn(async move {
            let stmt = Statement::from_string(
                DatabaseBackend::Postgres, 
                format!("SELECT {} as id", i)
            );
            let result = db_clone.query_one(stmt).await?;
            let row = result.unwrap();
            let id: i32 = row.try_get("", "id")?;
            Ok::<i32, sea_orm::DbErr>(id)
        });
        handles.push(handle);
    }
    
    // Wait for all tasks to complete
    for (i, handle) in handles.into_iter().enumerate() {
        let result = handle.await.unwrap()?;
        assert_eq!(result, i as i32);
    }
    
    Ok(())
}

/// Test connection timeout and retry mechanism
#[tokio::test]
async fn test_connection_timeout_and_retry() -> Result<()> {
    // Test with invalid connection string to trigger retry mechanism
    let mut config = DatabaseConfig::default();
    config.url = "postgres://invalid:invalid@nonexistent:5432/nonexistent".to_string();
    config.connect_timeout = Duration::from_millis(100);
    
    let start = Instant::now();
    let result = connect_with_config(&config).await;
    let elapsed = start.elapsed();
    
    // Should fail after retries
    assert!(result.is_err());
    
    // Should have taken some time due to retries (at least 100ms for initial attempt + retries)
    assert!(elapsed > Duration::from_millis(100));
    
    println!("Connection retry test completed in {:?}", elapsed);
    
    Ok(())
}

/// Test connection pool performance
#[tokio::test]
async fn test_connection_performance() -> Result<()> {
    if std::env::var("SKIP_DB_TESTS").is_ok() {
        return Ok(());
    }

    let db = connect().await?;
    
    // Test query performance
    let iterations = 10;
    let start = Instant::now();
    
    for i in 0..iterations {
        let stmt = Statement::from_string(
            DatabaseBackend::Postgres, 
            format!("SELECT {} as iteration", i)
        );
        let _result = db.query_one(stmt).await?;
    }
    
    let total_time = start.elapsed();
    let avg_time = total_time / iterations;
    
    println!("Average query time: {:?}", avg_time);
    println!("Total time for {} queries: {:?}", iterations, total_time);
    
    // Each query should complete reasonably fast (less than 100ms on average)
    assert!(avg_time < Duration::from_millis(100), 
           "Queries are too slow: {:?} average", avg_time);
    
    Ok(())
}

/// Test concurrent connection usage
#[tokio::test]
async fn test_concurrent_connections() -> Result<()> {
    if std::env::var("SKIP_DB_TESTS").is_ok() {
        return Ok(());
    }

    let db = connect().await?;
    let concurrent_tasks = 10;
    let queries_per_task = 5;
    
    let start = Instant::now();
    let mut handles: Vec<tokio::task::JoinHandle<Result<Vec<(i32, i32)>, sea_orm::DbErr>>> = vec![];
    
    for task_id in 0..concurrent_tasks {
        let db_clone = db.clone();
        let handle = tokio::spawn(async move {
            let mut results = vec![];
            for query_id in 0..queries_per_task {
                let stmt = Statement::from_string(
                    DatabaseBackend::Postgres,
                    format!("SELECT {} as task_id, {} as query_id", task_id, query_id)
                );
                let result = db_clone.query_one(stmt).await?;
                let row = result.unwrap();
                let t_id: i32 = row.try_get("", "task_id")?;
                let q_id: i32 = row.try_get("", "query_id")?;
                results.push((t_id, q_id));
            }
            Ok::<Vec<(i32, i32)>, sea_orm::DbErr>(results)
        });
        handles.push(handle);
    }
    
    // Wait for all tasks and verify results
    for (task_id, handle) in handles.into_iter().enumerate() {
        let results = handle.await.unwrap()?;
        assert_eq!(results.len(), queries_per_task);
        
        for (query_id, (t_id, q_id)) in results.into_iter().enumerate() {
            assert_eq!(t_id, task_id as i32);
            assert_eq!(q_id, query_id as i32);
        }
    }
    
    let total_time = start.elapsed();
    println!("Concurrent test completed in {:?}", total_time);
    
    // Should complete in reasonable time (less than 10 seconds)
    assert!(total_time < Duration::from_secs(10));
    
    Ok(())
}

/// Test connection resource cleanup
#[tokio::test]
async fn test_connection_cleanup() -> Result<()> {
    if std::env::var("SKIP_DB_TESTS").is_ok() {
        return Ok(());
    }

    // Create and drop multiple connections to test cleanup
    for i in 0..5 {
        let db = connect().await?;
        
        // Use the connection
        let stmt = Statement::from_string(
            DatabaseBackend::Postgres,
            format!("SELECT {} as connection_test", i)
        );
        let _result = db.query_one(stmt).await?;
        
        // Connection should be automatically cleaned up when dropped
        drop(db);
    }
    
    // Final connection should still work
    let db = connect().await?;
    let stmt = Statement::from_string(DatabaseBackend::Postgres, "SELECT 'cleanup_test' as test".to_string());
    let result = db.query_one(stmt).await?;
    assert!(result.is_some());
    
    Ok(())
}

/// Test connection timeout handling
#[tokio::test]
async fn test_connection_acquire_timeout() -> Result<()> {
    if std::env::var("SKIP_DB_TESTS").is_ok() {
        return Ok(());
    }

    let mut config = DatabaseConfig::default();
    // Ensure URL is set when using custom config
    config.url = crate::db::DATABASE_URL.clone();
    config.max_connections = 1; // Very limited pool
    config.acquire_timeout = Duration::from_millis(500);
    
    let db = connect_with_config(&config).await?;
    
    // This should work fine
    let stmt = Statement::from_string(DatabaseBackend::Postgres, "SELECT 1".to_string());
    let _result = timeout(Duration::from_secs(1), db.query_one(stmt)).await??;
    
    Ok(())
}

/// Test database connection helper functions
#[tokio::test]
async fn test_helper_functions() -> Result<()> {
    if std::env::var("SKIP_DB_TESTS").is_ok() {
        return Ok(());
    }

    // Test connection test function
    test_connection().await?;
    
    // Test pool stats function
    let db = connect().await?;
    let stats = get_pool_stats(&db).await?;
    
    println!("Pool stats: {}", stats);
    assert!(stats.contains("Query executed"));
    
    Ok(())
}

/// Integration test for database operations
#[tokio::test]
async fn test_database_integration() -> Result<()> {
    if std::env::var("SKIP_DB_TESTS").is_ok() {
        return Ok(());
    }

    let db = connect().await?;
    
    // Test database information queries
    let queries = vec![
        "SELECT version()",
        "SELECT current_database()",
        "SELECT current_user",
        "SELECT now()",
    ];
    
    for query in queries {
        let stmt = Statement::from_string(DatabaseBackend::Postgres, query.to_string());
        let result = db.query_one(stmt).await?;
        assert!(result.is_some(), "Query failed: {}", query);
        println!("Query '{}' executed successfully", query);
    }
    
    Ok(())
}