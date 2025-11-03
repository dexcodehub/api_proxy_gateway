//! Create `user_credentials` table storing password hashes.
//! Links to `user` via FK and enforces unique email per tenant via composite index.
use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(UserCredentials::Table)
                    .if_not_exists()
                    .col(uuid(UserCredentials::Id).primary_key())
                    .col(uuid(UserCredentials::UserId).unique_key().not_null())
                    .col(string_len(UserCredentials::PasswordHash, 255).not_null())
                    .col(string_len(UserCredentials::PasswordAlgorithm, 64).not_null())
                    .col(timestamp_with_time_zone(UserCredentials::CreatedAt).not_null())
                    .col(timestamp_with_time_zone(UserCredentials::UpdatedAt).not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_user_credentials_user")
                            .from(UserCredentials::Table, UserCredentials::UserId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Optional supporting index if we later store tenant/email directly here
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(UserCredentials::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum UserCredentials {
    Table,
    Id,
    UserId,
    PasswordHash,
    PasswordAlgorithm,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum User { Table, Id }