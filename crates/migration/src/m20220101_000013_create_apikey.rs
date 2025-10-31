use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ApiKey::Table)
                    .if_not_exists()
                    .col(uuid(ApiKey::Id).primary_key())
                    .col(uuid(ApiKey::UserId).not_null())
                    .col(string_len(ApiKey::KeyHash, 255).unique_key().not_null())
                    .col(string_len(ApiKey::Status, 32).not_null())
                    .col(timestamp_with_time_zone(ApiKey::CreatedAt).not_null())
                    .col(timestamp_with_time_zone(ApiKey::LastUsedAt).null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_apikey_user")
                            .from(ApiKey::Table, ApiKey::UserId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(ApiKey::Table).to_owned()).await
    }
}

#[derive(DeriveIden)]
enum ApiKey { Table, Id, UserId, KeyHash, Status, CreatedAt, LastUsedAt }

#[derive(DeriveIden)]
enum User { Table, Id }