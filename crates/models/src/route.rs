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

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;
    use chrono::Utc;

    #[test]
    fn construct_model() {
        let m = Model {
            id: Uuid::new_v4(),
            tenant_id: Uuid::new_v4(),
            method: "GET".into(),
            path: "/api".into(),
            upstream_id: Uuid::new_v4(),
            timeout_ms: 1000,
            retry_max_attempts: 2,
            circuit_breaker_threshold: 5,
            rate_limit_id: None,
            created_at: Utc::now().into(),
        };
        assert_eq!(m.method, "GET");
        assert_eq!(m.path, "/api");
        assert!(m.rate_limit_id.is_none());
    }
}