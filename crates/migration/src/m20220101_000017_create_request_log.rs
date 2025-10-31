//! Create `request_log` table with FKs to `route` and optional `api_key`.
//!
//! Stores per-request metrics and outcome for observability.
use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(RequestLog::Table)
                    .if_not_exists()
                    .col(big_integer(RequestLog::Id).primary_key().auto_increment())
                    .col(uuid(RequestLog::RouteId).not_null())
                    .col(
                        ColumnDef::new(RequestLog::ApiKeyId)
                            .uuid()
                            .null(),
                    )
                    .col(integer(RequestLog::StatusCode).not_null())
                    .col(integer(RequestLog::LatencyMs).not_null())
                    .col(boolean(RequestLog::Success).not_null())
                    .col(
                        ColumnDef::new(RequestLog::ErrorMessage)
                            .text()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(RequestLog::ClientIp)
                            .string_len(64)
                            .null(),
                    )
                    .col(timestamp_with_time_zone(RequestLog::Timestamp).not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_requestlog_route")
                            .from(RequestLog::Table, RequestLog::RouteId)
                            .to(Route::Table, Route::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_requestlog_apikey")
                            .from(RequestLog::Table, RequestLog::ApiKeyId)
                            .to(ApiKey::Table, ApiKey::Id)
                            .on_delete(ForeignKeyAction::SetNull)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(RequestLog::Table).to_owned()).await
    }
}

#[derive(DeriveIden)]
enum RequestLog { Table, Id, RouteId, ApiKeyId, StatusCode, LatencyMs, Success, ErrorMessage, ClientIp, Timestamp }

#[derive(DeriveIden)]
enum Route { Table, Id }

#[derive(DeriveIden)]
enum ApiKey { Table, Id }