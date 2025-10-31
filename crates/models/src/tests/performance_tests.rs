use crate::db::{connect, connect_with_config, DatabaseConfig, get_pool_stats};
use crate::tenant;
use sea_orm::{DatabaseConnection, DatabaseBackend, Statement, EntityTrait, ConnectionTrait};
use migration::MigratorTrait;
use anyhow::Result;
use uuid::Uuid;
use std::time::{Duration, Instant};
use std::sync::Arc;
use tokio::sync::Semaphore;

/// Setup test database
async fn setup_test_db() -> Result<DatabaseConnection> {
    let db = connect().await?;
    migration::Migrator::up(&db, None).await?;
    Ok(db)
}

/// Test connection pool performance under load
#[tokio::test]
async fn test_connection_pool_performance() -> Result<()> {
    if std::env::var("SKIP_DB_TESTS").is_ok() {
        return Ok(());
    }

    let mut config = DatabaseConfig::default();
    config.max_connections = 10;
    config.min_connections = 2;
    
    let db = connect_with_config(&config).await?;
    migration::Migrator::up(&db, None).await?;
    
    let concurrent_tasks = 20;
    let queries_per_task = 10;
    
    println!("Starting connection pool performance test:");
    println!("- Concurrent tasks: {}", concurrent_tasks);
    println!("- Queries per task: {}", queries_per_task);
    println!("- Max connections: {}", config.max_connections);
    
    let start = Instant::now();
    let mut handles: Vec<tokio::task::JoinHandle<anyhow::Result<(Duration, Duration, Duration)>>> = vec![];
    
    for task_id in 0..concurrent_tasks {
        let db_clone = db.clone();
        let handle = tokio::spawn(async move {
            let task_start = Instant::now();
            let mut query_times = vec![];
            
            for query_id in 0..queries_per_task {
                let query_start = Instant::now();
                
                let stmt = Statement::from_string(
                    DatabaseBackend::Postgres,
                    format!("SELECT {} as task_id, {} as query_id, now() as timestamp", task_id, query_id)
                );
                
                let _result = db_clone.query_one(stmt).await?;
                query_times.push(query_start.elapsed());
            }
            
            let task_duration = task_start.elapsed();
            let avg_query_time = query_times.iter().sum::<Duration>() / query_times.len() as u32;
            let max_query_time = query_times.iter().max().unwrap();
            
            Ok::<(Duration, Duration, Duration), anyhow::Error>((task_duration, avg_query_time, *max_query_time))
        });
        handles.push(handle);
    }
    
    // Collect results
    let mut task_durations = vec![];
    let mut avg_query_times = vec![];
    let mut max_query_times = vec![];
    
    for handle in handles {
        let (task_duration, avg_query_time, max_query_time) = handle.await??;
        task_durations.push(task_duration);
        avg_query_times.push(avg_query_time);
        max_query_times.push(max_query_time);
    }
    
    let total_duration = start.elapsed();
    let total_queries = concurrent_tasks * queries_per_task;
    
    // Calculate statistics
    let avg_task_duration = task_durations.iter().sum::<Duration>() / task_durations.len() as u32;
    let max_task_duration = task_durations.iter().max().unwrap();
    let overall_avg_query_time = avg_query_times.iter().sum::<Duration>() / avg_query_times.len() as u32;
    let overall_max_query_time = max_query_times.iter().max().unwrap();
    
    println!("Performance test results:");
    println!("- Total duration: {:?}", total_duration);
    println!("- Total queries: {}", total_queries);
    println!("- Queries per second: {:.2}", total_queries as f64 / total_duration.as_secs_f64());
    println!("- Average task duration: {:?}", avg_task_duration);
    println!("- Max task duration: {:?}", max_task_duration);
    println!("- Average query time: {:?}", overall_avg_query_time);
    println!("- Max query time: {:?}", overall_max_query_time);
    
    // Performance assertions
    assert!(total_duration < Duration::from_secs(30), "Test took too long: {:?}", total_duration);
    assert!(overall_avg_query_time < Duration::from_millis(100), "Queries too slow: {:?}", overall_avg_query_time);
    assert!(*overall_max_query_time < Duration::from_secs(1), "Max query time too high: {:?}", overall_max_query_time);
    
    // Test pool stats
    let stats = get_pool_stats(&db).await?;
    println!("Pool stats: {}", stats);
    
    Ok(())
}

/// Test memory usage and resource cleanup
#[tokio::test]
async fn test_resource_cleanup() -> Result<()> {
    if std::env::var("SKIP_DB_TESTS").is_ok() {
        return Ok(());
    }

    println!("Testing resource cleanup...");
    
    // Test multiple connection cycles
    for cycle in 0..5 {
        println!("Resource cleanup cycle {}", cycle + 1);
        
        // Create connection
        let db = connect().await?;
        migration::Migrator::up(&db, None).await?;
        
        // Use connection for multiple operations
        let mut tenant_ids = vec![];
        
        for i in 0..10 {
            let tenant_name = format!("cleanup_test_{}_{}", cycle, i);
            let tenant = tenant::create(&db, &tenant_name).await?;
            tenant_ids.push(tenant.id);
        }
        
        // Verify operations worked
        for &tenant_id in &tenant_ids {
            let tenant = tenant::Entity::find_by_id(tenant_id).one(&db).await?;
            assert!(tenant.is_some());
        }
        
        // Cleanup data
        for &tenant_id in &tenant_ids {
            tenant::Entity::delete_by_id(tenant_id).exec(&db).await?;
        }
        
        // Drop connection explicitly
        drop(db);
        
        // Small delay to allow cleanup
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    
    println!("Resource cleanup test completed successfully");
    Ok(())
}

/// Test connection pool exhaustion and recovery
#[tokio::test]
async fn test_connection_pool_exhaustion() -> Result<()> {
    if std::env::var("SKIP_DB_TESTS").is_ok() {
        return Ok(());
    }

    let mut config = DatabaseConfig::default();
    config.max_connections = 3; // Very small pool
    config.acquire_timeout = Duration::from_millis(500);
    
    let db = connect_with_config(&config).await?;
    migration::Migrator::up(&db, None).await?;
    
    println!("Testing connection pool exhaustion with {} max connections", config.max_connections);
    
    // Create more concurrent tasks than available connections
    let concurrent_tasks = 6;
    let semaphore = Arc::new(Semaphore::new(concurrent_tasks));
    let mut handles: Vec<tokio::task::JoinHandle<anyhow::Result<(usize, Duration, bool)>>> = vec![];
    
    for task_id in 0..concurrent_tasks {
        let db_clone = db.clone();
        let semaphore_clone = Arc::clone(&semaphore);
        
        let handle = tokio::spawn(async move {
            let _permit = semaphore_clone.acquire().await.unwrap();
            
            let start = Instant::now();
            
            // Hold connection for a while
            let stmt = Statement::from_string(
                DatabaseBackend::Postgres,
                format!("SELECT {} as task_id, pg_sleep(0.1)", task_id)
            );
            
            let result = db_clone.query_one(stmt).await;
            let duration = start.elapsed();
            
            match result {
                Ok(_) => Ok((task_id, duration, true)),
                Err(e) => {
                    println!("Task {} failed after {:?}: {}", task_id, duration, e);
                    Ok((task_id, duration, false))
                }
            }
        });
        
        handles.push(handle);
    }
    
    // Collect results
    let mut successful_tasks = 0;
    let mut failed_tasks = 0;
    
    for handle in handles {
        let (task_id, duration, success) = handle.await??;
        
        if success {
            successful_tasks += 1;
            println!("Task {} succeeded in {:?}", task_id, duration);
        } else {
            failed_tasks += 1;
        }
    }
    
    println!("Pool exhaustion test results:");
    println!("- Successful tasks: {}", successful_tasks);
    println!("- Failed tasks: {}", failed_tasks);
    
    // Should have some successful tasks (pool should handle some concurrency)
    assert!(successful_tasks > 0, "No tasks succeeded");
    
    // Test that pool recovers after exhaustion
    let stmt = Statement::from_string(DatabaseBackend::Postgres, "SELECT 'recovery_test' as test".to_string());
    let result = db.query_one(stmt).await?;
    assert!(result.is_some());
    
    println!("Connection pool recovery verified");
    Ok(())
}

/// Test database operation performance benchmarks
#[tokio::test]
async fn test_operation_benchmarks() -> Result<()> {
    if std::env::var("SKIP_DB_TESTS").is_ok() {
        return Ok(());
    }

    let db = setup_test_db().await?;
    
    println!("Running database operation benchmarks...");
    
    // Benchmark tenant creation
    let create_iterations = 100;
    let start = Instant::now();
    let mut tenant_ids = vec![];
    
    for i in 0..create_iterations {
        let tenant_name = format!("benchmark_tenant_{}", i);
        let tenant = tenant::create(&db, &tenant_name).await?;
        tenant_ids.push(tenant.id);
    }
    
    let create_duration = start.elapsed();
    let avg_create_time = create_duration / create_iterations;
    
    println!("Create benchmark:");
    println!("- {} operations in {:?}", create_iterations, create_duration);
    println!("- Average: {:?} per operation", avg_create_time);
    println!("- Rate: {:.2} ops/sec", create_iterations as f64 / create_duration.as_secs_f64());
    
    // Benchmark tenant reads
    let start = Instant::now();
    
    for &tenant_id in &tenant_ids {
        let _tenant = tenant::Entity::find_by_id(tenant_id).one(&db).await?;
    }
    
    let read_duration = start.elapsed();
    let avg_read_time = read_duration / create_iterations;
    
    println!("Read benchmark:");
    println!("- {} operations in {:?}", create_iterations, read_duration);
    println!("- Average: {:?} per operation", avg_read_time);
    println!("- Rate: {:.2} ops/sec", create_iterations as f64 / read_duration.as_secs_f64());
    
    // Benchmark tenant deletions
    let start = Instant::now();
    
    for &tenant_id in &tenant_ids {
        tenant::Entity::delete_by_id(tenant_id).exec(&db).await?;
    }
    
    let delete_duration = start.elapsed();
    let avg_delete_time = delete_duration / create_iterations;
    
    println!("Delete benchmark:");
    println!("- {} operations in {:?}", create_iterations, delete_duration);
    println!("- Average: {:?} per operation", avg_delete_time);
    println!("- Rate: {:.2} ops/sec", create_iterations as f64 / delete_duration.as_secs_f64());
    
    // Performance assertions
    assert!(avg_create_time < Duration::from_millis(50), "Create operations too slow");
    assert!(avg_read_time < Duration::from_millis(10), "Read operations too slow");
    assert!(avg_delete_time < Duration::from_millis(20), "Delete operations too slow");
    
    Ok(())
}

/// Test memory usage during bulk operations
#[tokio::test]
async fn test_bulk_operation_memory() -> Result<()> {
    if std::env::var("SKIP_DB_TESTS").is_ok() {
        return Ok(());
    }

    let db = setup_test_db().await?;
    
    println!("Testing memory usage during bulk operations...");
    
    let batch_size = 50;
    let num_batches = 5;
    
    for batch in 0..num_batches {
        println!("Processing batch {} of {}", batch + 1, num_batches);
        
        let mut tenant_ids = vec![];
        
        // Create batch
        for i in 0..batch_size {
            let tenant_name = format!("bulk_test_{}_{}", batch, i);
            let tenant = tenant::create(&db, &tenant_name).await?;
            tenant_ids.push(tenant.id);
        }
        
        // Verify batch
        for &tenant_id in &tenant_ids {
            let tenant = tenant::Entity::find_by_id(tenant_id).one(&db).await?;
            assert!(tenant.is_some());
        }
        
        // Cleanup batch immediately to test memory cleanup
        for &tenant_id in &tenant_ids {
            tenant::Entity::delete_by_id(tenant_id).exec(&db).await?;
        }
        
        // Force cleanup
        drop(tenant_ids);
        
        // Small delay for cleanup
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    
    println!("Bulk operation memory test completed successfully");
    Ok(())
}

/// Test connection timeout under load
#[tokio::test]
async fn test_connection_timeout_under_load() -> Result<()> {
    if std::env::var("SKIP_DB_TESTS").is_ok() {
        return Ok(());
    }

    let mut config = DatabaseConfig::default();
    config.max_connections = 2;
    config.acquire_timeout = Duration::from_millis(1000);
    config.connect_timeout = Duration::from_millis(5000);
    
    let db = connect_with_config(&config).await?;
    migration::Migrator::up(&db, None).await?;
    
    println!("Testing connection timeouts under load...");
    
    let concurrent_tasks = 5;
    let mut handles: Vec<tokio::task::JoinHandle<anyhow::Result<(usize, Duration, bool)>>> = vec![];
    
    for task_id in 0..concurrent_tasks {
        let db_clone = db.clone();
        
        let handle = tokio::spawn(async move {
            let start = Instant::now();
            
            // Try to get connection and execute query
            let stmt = Statement::from_string(
                DatabaseBackend::Postgres,
                format!("SELECT {} as task_id, pg_sleep(0.2)", task_id)
            );
            
            match tokio::time::timeout(Duration::from_secs(3), db_clone.query_one(stmt)).await {
                Ok(Ok(_)) => {
                    let duration = start.elapsed();
                    println!("Task {} completed in {:?}", task_id, duration);
                    Ok((task_id, duration, true))
                }
                Ok(Err(e)) => {
                    let duration = start.elapsed();
                    println!("Task {} failed in {:?}: {}", task_id, duration, e);
                    Ok((task_id, duration, false))
                }
                Err(_) => {
                    let duration = start.elapsed();
                    println!("Task {} timed out in {:?}", task_id, duration);
                    Ok((task_id, duration, false))
                }
            }
        });
        
        handles.push(handle);
    }
    
    // Collect results
    let mut completed_tasks = 0;
    let mut total_duration = Duration::new(0, 0);
    
    for handle in handles {
        let (task_id, duration, success) = handle.await??;
        total_duration += duration;
        
        if success {
            completed_tasks += 1;
        }
        
        // Each task should complete within reasonable time
        assert!(duration < Duration::from_secs(5), 
               "Task {} took too long: {:?}", task_id, duration);
    }
    
    println!("Timeout test results:");
    println!("- Completed tasks: {}/{}", completed_tasks, concurrent_tasks);
    println!("- Average duration: {:?}", total_duration / (concurrent_tasks as u32));
    
    // At least some tasks should complete successfully
    assert!(completed_tasks > 0, "No tasks completed successfully");
    
    Ok(())
}