/// Database connection and configuration tests
pub mod db_tests;

/// CRUD operations tests for all models
pub mod crud_tests;

/// Transaction handling and isolation tests
pub mod transaction_tests;

/// Performance and resource management tests
pub mod performance_tests;

/// Integration tests combining multiple components
pub mod integration_tests {
    use crate::db::connect;
    use crate::{tenant, user, apikey, upstream, ratelimit, route};
    use sea_orm::{EntityTrait, ActiveModelTrait, ConnectionTrait};
    use migration::MigratorTrait;
    use anyhow::Result;
    use uuid::Uuid;
    
    /// Test complete workflow: tenant -> user -> apikey -> upstream -> route
    #[tokio::test]
    async fn test_complete_workflow() -> Result<()> {
        if std::env::var("SKIP_DB_TESTS").is_ok() {
            return Ok(());
        }

        let db = connect().await?;
        migration::Migrator::up(&db, None).await?;
        
        // Create tenant
        let tenant_name = format!("workflow_tenant_{}", Uuid::new_v4());
        let test_tenant = tenant::create(&db, &tenant_name).await?;
        
        // Create user
        let user_email = format!("workflow_{}@example.com", Uuid::new_v4());
        let user_name = format!("Workflow User {}", Uuid::new_v4());
        let test_user = user::create(&db, test_tenant.id, &user_email, &user_name).await?;
        
        // Create API key
        let key_hash = "workflow_".to_string() + &"a".repeat(56); // 64 chars total
        let test_apikey = apikey::create(&db, test_user.id, &key_hash).await?;
        
        // Create upstream
        let up_name = format!("workflow_upstream_{}", Uuid::new_v4());
        let base_url = "https://workflow-api.example.com";
        let test_upstream = upstream::create(&db, &up_name, base_url).await?;
        
        // Create rate limit
        let rl = ratelimit::ActiveModel {
            id: sea_orm::Set(Uuid::new_v4()),
            tenant_id: sea_orm::Set(Some(test_tenant.id)),
            requests_per_minute: sea_orm::Set(1000),
            burst: sea_orm::Set(50),
            created_at: sea_orm::Set(chrono::Utc::now().into()),
        };
        let test_ratelimit = rl.insert(&db).await?;
        
        // Create route
        let rt = route::ActiveModel {
            id: sea_orm::Set(Uuid::new_v4()),
            tenant_id: sea_orm::Set(test_tenant.id),
            method: sea_orm::Set("GET".to_string()),
            path: sea_orm::Set("/api/v1/test".to_string()),
            upstream_id: sea_orm::Set(test_upstream.id),
            timeout_ms: sea_orm::Set(30000),
            retry_max_attempts: sea_orm::Set(3),
            circuit_breaker_threshold: sea_orm::Set(5000),
            rate_limit_id: sea_orm::Set(Some(test_ratelimit.id)),
            created_at: sea_orm::Set(chrono::Utc::now().into()),
        };
        let test_route = rt.insert(&db).await?;
        
        // Verify all entities exist and are properly linked
        let found_tenant = tenant::Entity::find_by_id(test_tenant.id).one(&db).await?;
        assert!(found_tenant.is_some());
        
        let found_user = user::Entity::find_by_id(test_user.id).one(&db).await?;
        assert!(found_user.is_some());
        assert_eq!(found_user.unwrap().tenant_id, test_tenant.id);
        
        let found_apikey = apikey::Entity::find_by_id(test_apikey.id).one(&db).await?;
        assert!(found_apikey.is_some());
        assert_eq!(found_apikey.unwrap().user_id, test_user.id);
        
        let found_upstream = upstream::Entity::find_by_id(test_upstream.id).one(&db).await?;
        assert!(found_upstream.is_some());
        
        let found_ratelimit = ratelimit::Entity::find_by_id(test_ratelimit.id).one(&db).await?;
        assert!(found_ratelimit.is_some());
        assert_eq!(found_ratelimit.unwrap().tenant_id, Some(test_tenant.id));
        
        let found_route = route::Entity::find_by_id(test_route.id).one(&db).await?;
        assert!(found_route.is_some());
        let found_route = found_route.unwrap();
        assert_eq!(found_route.tenant_id, test_tenant.id);
        assert_eq!(found_route.upstream_id, test_upstream.id);
        assert_eq!(found_route.rate_limit_id, Some(test_ratelimit.id));
        
        // Cleanup in reverse order
        route::Entity::delete_by_id(test_route.id).exec(&db).await?;
        ratelimit::Entity::delete_by_id(test_ratelimit.id).exec(&db).await?;
        upstream::Entity::delete_by_id(test_upstream.id).exec(&db).await?;
        apikey::Entity::delete_by_id(test_apikey.id).exec(&db).await?;
        user::Entity::delete_by_id(test_user.id).exec(&db).await?;
        tenant::Entity::delete_by_id(test_tenant.id).exec(&db).await?;
        
        println!("Complete workflow test passed successfully");
        Ok(())
    }
    
    /// Test data consistency across related entities
    #[tokio::test]
    async fn test_data_consistency() -> Result<()> {
        if std::env::var("SKIP_DB_TESTS").is_ok() {
            return Ok(());
        }

        let db = connect().await?;
        migration::Migrator::up(&db, None).await?;
        
        // Create tenant with multiple users
        let tenant_name = format!("consistency_tenant_{}", Uuid::new_v4());
        let test_tenant = tenant::create(&db, &tenant_name).await?;
        
        let mut user_ids = vec![];
        let mut apikey_ids = vec![];
        
        // Create multiple users and API keys
        for i in 0..3 {
            let user_email = format!("consistency_user_{}@example.com", i);
            let user_name = format!("Consistency User {}", i);
            let user = user::create(&db, test_tenant.id, &user_email, &user_name).await?;
            user_ids.push(user.id);
            
            // Create API key for each user
            let key_hash = format!("consistency_key_{}_", i) + &"a".repeat(50);
            let apikey = apikey::create(&db, user.id, &key_hash).await?;
            apikey_ids.push(apikey.id);
        }
        
        // Verify all users belong to the same tenant
        for &user_id in &user_ids {
            let user = user::Entity::find_by_id(user_id).one(&db).await?;
            assert!(user.is_some());
            assert_eq!(user.unwrap().tenant_id, test_tenant.id);
        }
        
        // Verify all API keys belong to the correct users
        for (i, &apikey_id) in apikey_ids.iter().enumerate() {
            let apikey = apikey::Entity::find_by_id(apikey_id).one(&db).await?;
            assert!(apikey.is_some());
            assert_eq!(apikey.unwrap().user_id, user_ids[i]);
        }
        
        // Test cascading delete via FK on_delete=CASCADE by deleting tenant
        tenant::Entity::delete_by_id(test_tenant.id).exec(&db).await?;
        
        // Users should not be findable after tenant soft delete
        for &user_id in &user_ids {
            let user = user::Entity::find_by_id(user_id).one(&db).await?;
            assert!(user.is_none());
        }
        
        // Cleanup
        for &apikey_id in &apikey_ids {
            apikey::Entity::delete_by_id(apikey_id).exec(&db).await?;
        }
        // users and tenant already deleted via cascade
        
        println!("Data consistency test passed successfully");
        Ok(())
    }
}