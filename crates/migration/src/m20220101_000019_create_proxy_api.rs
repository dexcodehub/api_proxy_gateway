//! Create `proxy_api` table.
//! Stores proxied API definitions with forwarding target and auth requirements.
use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create table
        manager
            .create_table(
                Table::create()
                    .table(ProxyApi::Table)
                    .if_not_exists()
                    .col(uuid(ProxyApi::Id).primary_key())
                    .col(uuid(ProxyApi::TenantId).not_null())
                    .col(string_len(ProxyApi::EndpointUrl, 256).not_null())
                    .col(string_len(ProxyApi::Method, 16).not_null())
                    .col(string_len(ProxyApi::ForwardTarget, 512).not_null())
                    .col(boolean(ProxyApi::RequireApiKey).not_null())
                    .col(boolean(ProxyApi::Enabled).not_null())
                    .col(timestamp_with_time_zone(ProxyApi::CreatedAt).not_null())
                    .col(timestamp_with_time_zone(ProxyApi::UpdatedAt).not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_proxy_api_tenant")
                            .from(ProxyApi::Table, ProxyApi::TenantId)
                            .to(Tenant::Table, Tenant::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Composite unique index to prevent duplicates per tenant
        manager
            .create_index(
                Index::create()
                    .name("idx_proxy_api_unique")
                    .table(ProxyApi::Table)
                    .col(ProxyApi::TenantId)
                    .col(ProxyApi::Method)
                    .col(ProxyApi::EndpointUrl)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(ProxyApi::Table).to_owned()).await
    }
}

#[derive(DeriveIden)]
enum ProxyApi {
    Table,
    Id,
    TenantId,
    EndpointUrl,
    Method,
    ForwardTarget,
    RequireApiKey,
    Enabled,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Tenant { Table, Id }