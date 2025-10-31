//! Create `rate_limit` table with optional FK to `tenant`.
//!
//! Defines throttling policies; tenant association is nullable.
use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(RateLimit::Table)
                    .if_not_exists()
                    .col(uuid(RateLimit::Id).primary_key())
                    .col(
                        ColumnDef::new(RateLimit::TenantId)
                            .uuid()
                            .null(),
                    )
                    .col(integer(RateLimit::RequestsPerMinute).not_null())
                    .col(integer(RateLimit::Burst).not_null())
                    .col(timestamp_with_time_zone(RateLimit::CreatedAt).not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_ratelimit_tenant")
                            .from(RateLimit::Table, RateLimit::TenantId)
                            .to(Tenant::Table, Tenant::Id)
                            .on_delete(ForeignKeyAction::SetNull)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(RateLimit::Table).to_owned()).await
    }
}

#[derive(DeriveIden)]
enum RateLimit { Table, Id, TenantId, RequestsPerMinute, Burst, CreatedAt }

#[derive(DeriveIden)]
enum Tenant { Table, Id }