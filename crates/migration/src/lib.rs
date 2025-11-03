//! Migrator registering entity-specific migrations in dependency order.
//! Indexes are applied last.
pub use sea_orm_migration::prelude::*;

mod m20220101_000011_create_tenant;
mod m20220101_000012_create_user;
mod m20220101_000013_create_apikey;
mod m20220101_000014_create_upstream;
mod m20220101_000015_create_ratelimit;
mod m20220101_000016_create_route;
mod m20220101_000017_create_request_log;
mod m20220101_000018_create_user_credentials;
mod m20220101_000002_add_indexes;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000011_create_tenant::Migration),
            Box::new(m20220101_000012_create_user::Migration),
            Box::new(m20220101_000018_create_user_credentials::Migration),
            Box::new(m20220101_000013_create_apikey::Migration),
            Box::new(m20220101_000014_create_upstream::Migration),
            Box::new(m20220101_000015_create_ratelimit::Migration),
            Box::new(m20220101_000016_create_route::Migration),
            Box::new(m20220101_000017_create_request_log::Migration),
            // Indexes should always be applied last
            Box::new(m20220101_000002_add_indexes::Migration),
        ]
    }
}
