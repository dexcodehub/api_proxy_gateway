use sea_orm::entity::prelude::*;
use uuid::Uuid;
use serde::{Deserialize, Serialize};

use crate::{tenant, upstream, ratelimit};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "route")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub method: String,
    pub path: String,
    pub upstream_id: Uuid,
    pub timeout_ms: i32,
    pub retry_max_attempts: i32,
    pub circuit_breaker_threshold: i32,
    pub rate_limit_id: Option<Uuid>,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation { Tenant, Upstream, RateLimit }

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Relation::Tenant => Entity::belongs_to(tenant::Entity).from(Column::TenantId).to(tenant::Column::Id).into(),
            Relation::Upstream => Entity::belongs_to(upstream::Entity).from(Column::UpstreamId).to(upstream::Column::Id).into(),
            Relation::RateLimit => Entity::belongs_to(ratelimit::Entity).from(Column::RateLimitId).to(ratelimit::Column::Id).into(),
        }
    }
}

impl ActiveModelBehavior for ActiveModel {}