pub mod errors;
pub mod db;
pub mod tenant;
pub mod user;
pub mod user_credentials;
pub mod apikey;
pub mod upstream;
pub mod ratelimit;
pub mod route;
pub mod request_log;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod legacy_tests {
    use std::time::Instant;
    use migration::MigratorTrait;
    use sea_orm::{Set, TransactionTrait, ActiveModelTrait};
    use uuid::Uuid;
    use chrono::Utc;

    use crate::{db, tenant, user, upstream, ratelimit};

    #[tokio::test]
    async fn test_tenant_user_crud_and_metrics() {
        let db = match db::connect().await {
            Ok(db) => db,
            Err(e) => {
                eprintln!("skip: cannot connect to db: {}", e);
                return;
            }
        };
        if let Err(e) = migration::Migrator::up(&db, None).await {
            eprintln!("skip: migrate up failed: {}", e);
            return;
        }
        let txn = db.begin().await.expect("begin");

        let start = Instant::now();
        let mut success = 0u32;
        let total = 6u32;

        let t = tenant::create(&db, "acme").await.expect("create tenant"); success += 1;
        let u = user::create(&db, t.id, "bob@example.com", "Bob").await.expect("create user"); success += 1;
        user::soft_delete(&db, u.id).await.expect("soft delete"); success += 1;
        user::hard_delete(&db, u.id).await.expect("hard delete"); success += 1;
        let _up = upstream::create(&db, "jsonplaceholder", "https://jsonplaceholder.typicode.com").await.expect("create upstream"); success += 1;
        let rl = ratelimit::ActiveModel { id: Set(Uuid::new_v4()), tenant_id: Set(Some(t.id)), requests_per_minute: Set(60), burst: Set(10), created_at: Set(Utc::now().into()) };
        let _rlm = rl.insert(&txn).await.expect("insert ratelimit"); success += 1;

        let duration = start.elapsed();
        println!("success_rate={}%, elapsed_ms={}", (success as f32/ total as f32)*100.0, duration.as_millis());

        txn.rollback().await.expect("rollback");
        if let Err(e) = migration::Migrator::down(&db, None).await {
            eprintln!("cleanup: migrate down failed: {}", e);
        }
    }
}
