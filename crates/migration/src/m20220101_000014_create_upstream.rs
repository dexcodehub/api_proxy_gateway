//! Create `upstream` table.
//!
//! Records backend services and health endpoints.
use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Upstream::Table)
                    .if_not_exists()
                    .col(uuid(Upstream::Id).primary_key())
                    .col(string_len(Upstream::Name, 128).unique_key().not_null())
                    .col(string_len(Upstream::BaseUrl, 512).not_null())
                    .col(
                        ColumnDef::new(Upstream::HealthUrl)
                            .string_len(512)
                            .null(),
                    )
                    .col(boolean(Upstream::Active).not_null())
                    .col(timestamp_with_time_zone(Upstream::CreatedAt).not_null())
                    .col(timestamp_with_time_zone(Upstream::UpdatedAt).not_null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(Upstream::Table).to_owned()).await
    }
}

#[derive(DeriveIden)]
enum Upstream { Table, Id, Name, BaseUrl, HealthUrl, Active, CreatedAt, UpdatedAt }