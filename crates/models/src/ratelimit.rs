use sea_orm::entity::prelude::*;
use uuid::Uuid;
use serde::{Deserialize, Serialize};

use crate::tenant;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "rate_limit")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub tenant_id: Option<Uuid>,
    pub requests_per_minute: i32,
    pub burst: i32,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation { Tenant }

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self { Relation::Tenant => Entity::belongs_to(tenant::Entity).from(Column::TenantId).to(tenant::Column::Id).into() }
    }
}

impl ActiveModelBehavior for ActiveModel {}