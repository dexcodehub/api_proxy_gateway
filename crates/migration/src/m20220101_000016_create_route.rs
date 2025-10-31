use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Route::Table)
                    .if_not_exists()
                    .col(uuid(Route::Id).primary_key())
                    .col(uuid(Route::TenantId).not_null())
                    .col(string_len(Route::Method, 16).not_null())
                    .col(string_len(Route::Path, 256).not_null())
                    .col(uuid(Route::UpstreamId).not_null())
                    .col(integer(Route::TimeoutMs).not_null())
                    .col(integer(Route::RetryMaxAttempts).not_null())
                    .col(integer(Route::CircuitBreakerThreshold).not_null())
                    .col(uuid(Route::RateLimitId).null())
                    .col(timestamp_with_time_zone(Route::CreatedAt).not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_route_tenant")
                            .from(Route::Table, Route::TenantId)
                            .to(Tenant::Table, Tenant::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_route_upstream")
                            .from(Route::Table, Route::UpstreamId)
                            .to(Upstream::Table, Upstream::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_route_ratelimit")
                            .from(Route::Table, Route::RateLimitId)
                            .to(RateLimit::Table, RateLimit::Id)
                            .on_delete(ForeignKeyAction::SetNull)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(Route::Table).to_owned()).await
    }
}

#[derive(DeriveIden)]
enum Route {
    Table,
    Id,
    TenantId,
    Method,
    Path,
    UpstreamId,
    TimeoutMs,
    RetryMaxAttempts,
    CircuitBreakerThreshold,
    RateLimitId,
    CreatedAt,
}

#[derive(DeriveIden)]
enum Tenant { Table, Id }

#[derive(DeriveIden)]
enum Upstream { Table, Id }

#[derive(DeriveIden)]
enum RateLimit { Table, Id }