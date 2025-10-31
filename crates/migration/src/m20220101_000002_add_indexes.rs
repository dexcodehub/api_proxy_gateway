use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Users: index on tenant_id
        manager
            .create_index(
                Index::create()
                    .name("idx_user_tenant")
                    .table(User::Table)
                    .col(User::TenantId)
                    .to_owned(),
            )
            .await?;

        // ApiKey: index on user_id
        manager
            .create_index(
                Index::create()
                    .name("idx_apikey_user")
                    .table(ApiKey::Table)
                    .col(ApiKey::UserId)
                    .to_owned(),
            )
            .await?;

        // Route: composite unique (tenant_id, method, path)
        manager
            .create_index(
                Index::create()
                    .name("uniq_route_tenant_method_path")
                    .table(Route::Table)
                    .col(Route::TenantId)
                    .col(Route::Method)
                    .col(Route::Path)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // RequestLog: index on route_id and timestamp
        manager
            .create_index(
                Index::create()
                    .name("idx_log_route")
                    .table(RequestLog::Table)
                    .col(RequestLog::RouteId)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_log_timestamp")
                    .table(RequestLog::Table)
                    .col(RequestLog::Timestamp)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(Index::drop().name("idx_user_tenant").table(User::Table).to_owned())
            .await?;
        manager
            .drop_index(Index::drop().name("idx_apikey_user").table(ApiKey::Table).to_owned())
            .await?;
        manager
            .drop_index(Index::drop().name("uniq_route_tenant_method_path").table(Route::Table).to_owned())
            .await?;
        manager
            .drop_index(Index::drop().name("idx_log_route").table(RequestLog::Table).to_owned())
            .await?;
        manager
            .drop_index(Index::drop().name("idx_log_timestamp").table(RequestLog::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum User { Table, TenantId }

#[derive(DeriveIden)]
enum ApiKey { Table, UserId }

#[derive(DeriveIden)]
enum Route { Table, TenantId, Method, Path }

#[derive(DeriveIden)]
enum RequestLog { Table, RouteId, Timestamp }