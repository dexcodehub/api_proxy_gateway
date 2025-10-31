//! Create `tenant` table.
//!
//! Root entity for multi-tenancy; other tables reference it.
use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Tenant::Table)
                    .if_not_exists()
                    .col(uuid(Tenant::Id).primary_key())
                    .col(string_len(Tenant::Name, 128).unique_key().not_null())
                    .col(timestamp_with_time_zone(Tenant::CreatedAt).not_null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(Tenant::Table).to_owned()).await
    }
}

#[derive(DeriveIden)]
enum Tenant { Table, Id, Name, CreatedAt }