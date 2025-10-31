//! Create `user` table with FK to `tenant`.
//!
//! Stores end-users; includes soft-delete timestamp.
use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(User::Table)
                    .if_not_exists()
                    .col(uuid(User::Id).primary_key())
                    .col(uuid(User::TenantId).not_null())
                    .col(string_len(User::Email, 255).unique_key().not_null())
                    .col(string_len(User::Name, 128).not_null())
                    .col(string_len(User::Status, 32).not_null())
                    .col(timestamp_with_time_zone(User::CreatedAt).not_null())
                    .col(timestamp_with_time_zone(User::UpdatedAt).not_null())
                    // Explicitly define nullable deleted_at to avoid conflicting NULL/NOT NULL
                    .col(
                        ColumnDef::new(User::DeletedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_user_tenant")
                            .from(User::Table, User::TenantId)
                            .to(Tenant::Table, Tenant::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(User::Table).to_owned()).await
    }
}

#[derive(DeriveIden)]
enum User { Table, Id, TenantId, Email, Name, Status, CreatedAt, UpdatedAt, DeletedAt }

#[derive(DeriveIden)]
enum Tenant { Table, Id }